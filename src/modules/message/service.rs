/// Message Service
///
/// Service layer xử lý business logic cho messages, bao gồm:
/// - Gửi tin nhắn (direct và group)
/// - Xóa và chỉnh sửa tin nhắn
/// - Broadcast real-time qua WebSocket
use actix::Addr;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::error;
use crate::configs::RedisCache;
use crate::modules::conversation::model::NewLastMessage;
use crate::modules::conversation::repository::{
    ConversationRepository, LastMessageRepository, ParticipantRepository,
};
use crate::modules::message::model::InsertMessage;
use crate::modules::message::repository::MessageRepository;
use crate::modules::message::schema::MessageEntity;
use crate::modules::websocket::events::BroadcastToRoom;
use crate::modules::websocket::message::{LastMessageInfo, SenderInfo, ServerMessage};
use crate::modules::websocket::server::WebSocketServer;

/// Message service với generic repositories để dễ testing
#[derive(Clone)]
pub struct MessageService<M, C, P, L>
where
    M: MessageRepository + Send + Sync,
    C: ConversationRepository + Send + Sync,
    P: ParticipantRepository + Send + Sync,
    L: LastMessageRepository + Send + Sync,
{
    message_repo: Arc<M>,
    conversation_repo: Arc<C>,
    participant_repo: Arc<P>,
    last_message_repo: Arc<L>,
    cache: Arc<RedisCache>,
    ws_server: Arc<Addr<WebSocketServer>>,
}

impl<M, C, P, L> MessageService<M, C, P, L>
where
    C: ConversationRepository + Send + Sync,
    M: MessageRepository + Send + Sync,
    P: ParticipantRepository + Send + Sync,
    L: LastMessageRepository + Send + Sync,
{
    /// Tạo MessageService với các dependencies
    pub fn with_dependencies(
        conversation_repo: Arc<C>,
        message_repo: Arc<M>,
        participant_repo: Arc<P>,
        last_message_repo: Arc<L>,
        cache: Arc<RedisCache>,
        ws_server: Arc<Addr<WebSocketServer>>,
    ) -> Self {
        MessageService {
            conversation_repo,
            message_repo,
            participant_repo,
            last_message_repo,
            cache,
            ws_server,
        }
    }

    /// Gửi direct message giữa 2 users
    ///
    /// Flow:
    /// 1. Tìm hoặc tạo conversation
    /// 2. Tạo message trong DB
    /// 3. Increment unread count cho recipient
    /// 4. Upsert last message
    /// 5. Broadcast qua WebSocket
    pub async fn send_direct_message(
        &self,
        sender_id: Uuid,
        recipient_id: Uuid,
        content: String,
        conversation_id: Option<Uuid>,
    ) -> Result<MessageEntity, error::SystemError> {
        let mut tx = self.conversation_repo.get_pool().begin().await?;

        let conversation = match conversation_id {
            Some(conv_id) => self
                .conversation_repo
                .find_by_id(&conv_id, self.conversation_repo.get_pool())
                .await?
                .ok_or_else(|| error::SystemError::not_found("Conversation not found"))?,
            None => self
                .conversation_repo
                .find_direct_between_users(&sender_id, &recipient_id, tx.as_mut())
                .await?
                .unwrap_or(
                    self.conversation_repo
                        .create_direct_conversation(&sender_id, &recipient_id, &mut tx)
                        .await?,
                ),
        };

        let message = self
            .message_repo
            .create(
                &InsertMessage {
                    conversation_id: conversation.id,
                    sender_id,
                    content: Some(content.clone()),
                },
                tx.as_mut(),
            )
            .await?;

        self.participant_repo
            .increment_unread_count(&conversation.id, &recipient_id, tx.as_mut())
            .await?;

        self.last_message_repo
            .upsert_last_message(
                &NewLastMessage {
                    conversation_id: conversation.id,
                    sender_id,
                    content: Some(content.clone()),
                    created_at: message.created_at,
                },
                tx.as_mut(),
            )
            .await?;

        self.conversation_repo.update_timestamp(&conversation.id, tx.as_mut()).await?;

        // Get unread counts for all participants
        let unread_counts = self
            .participant_repo
            .get_unread_counts(&conversation.id, tx.as_mut())
            .await?;

        tx.commit().await?;

        // Build and broadcast new message
        let server_message = self.build_new_message_event(&message, &unread_counts);
        self.ws_server.do_send(BroadcastToRoom {
            conversation_id: conversation.id,
            message: server_message,
            skip_user_id: Some(sender_id),
        });

        Ok(message)
    }

    /// Gửi group message
    ///
    /// Flow:
    /// 1. Tạo message trong DB
    /// 2. Increment unread count cho tất cả participants (trừ sender)
    /// 3. Upsert last message
    /// 4. Broadcast qua WebSocket
    pub async fn send_group_message(
        &self,
        sender_id: Uuid,
        content: String,
        conversation_id: Uuid,
    ) -> Result<MessageEntity, error::SystemError> {
        let mut tx = self.conversation_repo.get_pool().begin().await?;

        let message = self
            .message_repo
            .create(
                &InsertMessage { content: Some(content.clone()), conversation_id, sender_id },
                tx.as_mut(),
            )
            .await?;

        self.participant_repo
            .increment_unread_count_for_others(&conversation_id, &sender_id, tx.as_mut())
            .await?;

        self.last_message_repo
            .upsert_last_message(
                &NewLastMessage {
                    conversation_id,
                    sender_id,
                    content: Some(content),
                    created_at: message.created_at,
                },
                tx.as_mut(),
            )
            .await?;

        self.conversation_repo.update_timestamp(&conversation_id, tx.as_mut()).await?;

        // Get unread counts for all participants
        let unread_counts = self
            .participant_repo
            .get_unread_counts(&conversation_id, tx.as_mut())
            .await?;

        tx.commit().await?;

        // Build and broadcast new message
        let server_message = self.build_new_message_event(&message, &unread_counts);
        self.ws_server.do_send(BroadcastToRoom {
            conversation_id,
            message: server_message,
            skip_user_id: Some(sender_id),
        });

        Ok(message)
    }

    /// Xóa message (soft delete)
    ///
    /// Chỉ sender mới có thể xóa message của mình
    pub async fn delete_message(
        &self,
        message_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), error::SystemError> {
        let mut tx = self.conversation_repo.get_pool().begin().await?;

        let message = self
            .message_repo
            .find_by_id(&message_id, tx.as_mut())
            .await?
            .ok_or_else(|| error::SystemError::not_found("Message not found"))?;

        if message.sender_id != user_id {
            return Err(error::SystemError::forbidden("You can only delete your own messages"));
        }

        let deleted = self.message_repo.delete_message(&message_id, &user_id, tx.as_mut()).await?;

        if !deleted {
            return Err(error::SystemError::not_found("Message not found or already deleted"));
        }

        tx.commit().await?;

        self.ws_server.do_send(BroadcastToRoom {
            conversation_id: message.conversation_id,
            message: ServerMessage::MessageDeleted {
                conversation_id: message.conversation_id,
                message_id,
            },
            skip_user_id: None,
        });

        Ok(())
    }

    /// Chỉnh sửa message
    ///
    /// Chỉ sender mới có thể edit message của mình
    pub async fn edit_message(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        new_content: String,
    ) -> Result<MessageEntity, error::SystemError> {
        let mut tx = self.conversation_repo.get_pool().begin().await?;

        let message = self
            .message_repo
            .find_by_id(&message_id, tx.as_mut())
            .await?
            .ok_or_else(|| error::SystemError::not_found("Message not found"))?;

        if message.sender_id != user_id {
            return Err(error::SystemError::forbidden("You can only edit your own messages"));
        }

        let edited_message = self
            .message_repo
            .edit_message(&message_id, &user_id, &new_content, tx.as_mut())
            .await?
            .ok_or_else(|| error::SystemError::not_found("Message not found"))?;

        tx.commit().await?;

        self.ws_server.do_send(BroadcastToRoom {
            conversation_id: message.conversation_id,
            message: ServerMessage::MessageEdited {
                conversation_id: message.conversation_id,
                message_id,
                new_content,
            },
            skip_user_id: None,
        });

        Ok(edited_message)
    }

    /// Helper: Build new-message event với format tương thích Socket.IO
    fn build_new_message_event(
        &self,
        message: &MessageEntity,
        unread_counts: &HashMap<Uuid, i32>,
    ) -> ServerMessage {
        let message_json = serde_json::to_value(message).unwrap_or_default();

        let last_message = LastMessageInfo {
            _id: message.id,
            content: message.content.clone(),
            created_at: message.created_at.to_rfc3339(),
            sender: SenderInfo {
                _id: message.sender_id,
                display_name: String::new(), // Will be filled by frontend from cache
                avatar_url: None,
            },
        };

        // Convert HashMap<Uuid, i32> to JSON object with string keys
        let unread_counts_json: serde_json::Value = unread_counts
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::Number((*v).into())))
            .collect();

        ServerMessage::new_message(
            message_json,
            message.conversation_id,
            last_message,
            message.created_at.to_rfc3339(),
            unread_counts_json,
        )
    }
}
