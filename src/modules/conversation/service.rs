/// Conversation Service
///
/// Service layer xử lý business logic cho conversations.
/// Bao gồm tạo conversation, lấy danh sách, mark as seen, và WebSocket notifications.
use std::{collections::HashMap, sync::Arc};

use actix::Addr;
use uuid::Uuid;

use crate::{
    api::error,
    modules::{
        conversation::{
            model::{ConversationDetail, ParticipantDetailWithConversation, ParticipantRow},
            repository::{ConversationRepository, ParticipantRepository},
            schema::{ConversationEntity, ConversationType},
        },
        message::{model::MessageQuery, repository::MessageRepository, schema::MessageEntity},
        websocket::{
            events::{SendToUsers, BroadcastToRoom},
            message::{LastMessageInfo, SenderInfo, ServerMessage},
            server::WebSocketServer,
        },
    },
};

/// ConversationService với generic repositories để dễ testing và decoupling
#[derive(Clone)]
pub struct ConversationService<R, P, L>
where
    R: ConversationRepository + Send + Sync,
    P: ParticipantRepository + Send + Sync,
    L: MessageRepository + Send + Sync,
{
    conversation_repo: Arc<R>,
    participant_repo: Arc<P>,
    message_repo: Arc<L>,
    ws_server: Arc<Addr<WebSocketServer>>,
}

impl<R, P, L> ConversationService<R, P, L>
where
    R: ConversationRepository + Send + Sync,
    P: ParticipantRepository + Send + Sync,
    L: MessageRepository + Send + Sync,
{
    /// Tạo ConversationService với tất cả dependencies
    pub fn with_dependencies(
        conversation_repo: Arc<R>,
        participant_repo: Arc<P>,
        message_repo: Arc<L>,
        ws_server: Arc<Addr<WebSocketServer>>,
    ) -> Self {
        ConversationService { conversation_repo, participant_repo, message_repo, ws_server }
    }

    /// Lấy conversation theo ID
    pub async fn get_by_id(
        &self,
        conversation_id: Uuid,
    ) -> Result<ConversationEntity, error::SystemError> {
        let conversation = self
            .conversation_repo
            .find_by_id(&conversation_id, self.conversation_repo.get_pool())
            .await?
            .ok_or_else(|| error::SystemError::not_found("Conversation not found"))?;

        Ok(conversation)
    }

    /// Tạo conversation mới (direct hoặc group)
    ///
    /// Với direct: tạo hoặc trả về conversation hiện có giữa 2 users
    /// Với group: tạo group mới và notify tất cả members
    pub async fn create_conversation(
        &self,
        _type: ConversationType,
        name: String,
        member_ids: Vec<Uuid>,
        user_id: Uuid,
    ) -> Result<Option<ConversationDetail>, error::SystemError> {
        let mut tx = self.conversation_repo.get_pool().begin().await?;

        let participant = member_ids.first().ok_or_else(|| {
            error::SystemError::bad_request(
                "At least one member is required to create a conversation",
            )
        })?;

        let conversation = match _type {
            ConversationType::Direct => {
                if let Some(conv) = self
                    .conversation_repo
                    .find_direct_between_users(&user_id, participant, tx.as_mut())
                    .await?
                {
                    conv
                } else {
                    self.conversation_repo
                        .create_direct_conversation(&user_id, participant, &mut tx)
                        .await?
                }
            }

            ConversationType::Group => {
                self.conversation_repo
                    .create_group_conversation(&name, &member_ids, &user_id, &mut tx)
                    .await?
            }
        };

        tx.commit().await?;

        let conversation_detail =
            self.conversation_repo.find_one_conversation_detail(&conversation.id).await?;

        // Serialize conversation for WebSocket broadcast
        let conversation_json = serde_json::to_value(&conversation_detail).map_err(|e| {
            error::SystemError::internal_error(format!("Failed to serialize conversation: {}", e))
        })?;

        // Broadcast dựa trên type
        match _type {
            ConversationType::Group => {
                // Gửi new-group event tới tất cả members (trừ creator)
                // Format tương thích với Socket.IO client
                self.ws_server.do_send(SendToUsers {
                    user_ids: member_ids.clone(),
                    message: ServerMessage::NewGroup { conversation: conversation_json },
                });
            }
            ConversationType::Direct => {
                // Direct message không cần broadcast khi tạo mới
                // Sẽ broadcast khi có message đầu tiên
            }
        }

        Ok(conversation_detail)
    }

    /// Lấy tất cả conversations của user
    pub async fn get_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConversationDetail>, error::SystemError> {
        let pool = self.conversation_repo.get_pool();
        let conversations = self
            .conversation_repo
            .find_all_conversation_with_details_by_user(&user_id, pool)
            .await?;

        let conversation_ids: Vec<Uuid> =
            conversations.iter().map(|conv_row| conv_row.conversation_id).collect();

        let participants = self
            .participant_repo
            .find_participants_by_conversation_id(&conversation_ids, pool)
            .await?;

        let participant_map = participants.into_iter().fold(
            HashMap::<Uuid, Vec<ParticipantDetailWithConversation>>::new(),
            |mut acc, participant| {
                acc.entry(participant.conversation_id).or_insert_with(Vec::new).push(participant);
                acc
            },
        );

        let res = conversations.into_iter().map(|conv| {
            let participants: Vec<ParticipantRow> = participant_map
                .get(&conv.conversation_id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|p| ParticipantRow {
                    user_id: p.user_id,
                    display_name: p.display_name,
                    avatar_url: p.avatar_url,
                    unread_count: p.unread_count,
                    joined_at: p.joined_at,
                })
                .collect();

            ConversationDetail {
                conversation_id: conv.conversation_id,
                _type: conv._type,
                group_info: conv.group_info,
                last_message: conv.last_message,
                participants,
                created_at: conv.created_at,
                updated_at: conv.updated_at,
            }
        });

        Ok(res.collect())
    }

    /// Lấy messages của conversation với cursor-based pagination
    pub async fn get_message(
        &self,
        conversation_id: Uuid,
        limit: i32,
        cursor: Option<String>,
    ) -> Result<(Vec<MessageEntity>, Option<String>), error::SystemError> {
        let created_at = match cursor {
            Some(c) => Some(
                chrono::DateTime::parse_from_rfc3339(&c)
                    .map_err(|_| error::SystemError::bad_request("Invalid cursor format"))?
                    .with_timezone(&chrono::Utc),
            ),
            None => None,
        };

        let mut messages = self
            .message_repo
            .find_by_query(
                &MessageQuery { conversation_id, created_at },
                limit,
                self.message_repo.get_pool(),
            )
            .await?;

        let next_cursor = if messages.len() > limit as usize {
            messages.pop().map(|m| m.created_at)
        } else {
            None
        };

        messages.reverse();
        Ok((messages, next_cursor.map(|c| c.to_rfc3339())))
    }

    /// Lấy participants của conversation
    pub async fn get_participants_by_conversation_id(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<ParticipantDetailWithConversation>, error::SystemError> {
        let participants = self
            .participant_repo
            .find_participants_by_conversation_id(
                &[conversation_id],
                self.conversation_repo.get_pool(),
            )
            .await?;

        Ok(participants)
    }

    /// Kiểm tra user có phải member của conversation không
    pub async fn get_conversation_and_check_membership(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<(Option<ConversationEntity>, bool), error::SystemError> {
        self.conversation_repo
            .get_conversation_and_check_membership(
                &conversation_id,
                &user_id,
                self.conversation_repo.get_pool(),
            )
            .await
    }

    /// Mark messages as seen
    ///
    /// Cập nhật last_seen_message_id và reset unread count
    /// Broadcast read-message event tới conversation room
    pub async fn mark_as_seen(
        &self,
        conversation_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), error::SystemError> {
        let mut tx = self.conversation_repo.get_pool().begin().await?;

        // Verify user is a participant of the conversation
        let (_, is_member) = self
            .conversation_repo
            .get_conversation_and_check_membership(&conversation_id, &user_id, tx.as_mut())
            .await?;

        if !is_member {
            return Err(error::SystemError::forbidden(
                "User is not a participant of this conversation",
            ));
        }

        // Get last message of the conversation
        let last_message = self
            .message_repo
            .get_last_message_by_conversation(&conversation_id, tx.as_mut())
            .await?;

        if let Some(msg) = last_message {
            // Check if user is the sender of the last message
            if msg.sender_id == user_id {
                // Sender doesn't need to mark as seen
                tx.commit().await?;
                return Ok(());
            }

            // Mark as seen with the last message ID
            self.participant_repo
                .mark_as_seen(&conversation_id, &user_id, &msg.id, tx.as_mut())
                .await?;

            tx.commit().await?;

            // Broadcast read-message event với format tương thích Socket.IO
            let last_message_info = LastMessageInfo {
                _id: msg.id,
                content: msg.content.clone(),
                created_at: msg.created_at.to_rfc3339(),
                sender: SenderInfo {
                    _id: msg.sender_id,
                    display_name: String::new(),
                    avatar_url: None,
                },
            };

            // Tạo conversation update info
            let conversation_update = serde_json::json!({
                "_id": conversation_id,
                "unreadCounts": {},
                "seenBy": [user_id]
            });

            self.ws_server.do_send(BroadcastToRoom {
                conversation_id,
                message: ServerMessage::read_message(conversation_update, last_message_info),
                skip_user_id: None,
            });
        } else {
            tx.commit().await?;
        }

        Ok(())
    }
}
