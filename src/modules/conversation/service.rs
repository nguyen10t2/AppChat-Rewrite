use std::{collections::HashMap, sync::Arc};
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
    },
};

#[derive(Clone)]
pub struct ConversationService<R, P, L>
where
    R: ConversationRepository + Send + Sync + 'static,
    P: ParticipantRepository + Send + Sync + 'static,
    L: MessageRepository + Send + Sync + 'static,
{
    conversation_repo: Arc<R>,
    participant_repo: Arc<P>,
    message_repo: Arc<L>,
}

impl<R, P, L> ConversationService<R, P, L>
where
    R: ConversationRepository + Send + Sync + 'static,
    P: ParticipantRepository + Send + Sync + 'static,
    L: MessageRepository + Send + Sync + 'static,
{
    pub fn with_dependencies(
        conversation_repo: Arc<R>,
        participant_repo: Arc<P>,
        message_repo: Arc<L>,
    ) -> Self {
        ConversationService { conversation_repo, participant_repo, message_repo }
    }

    #[allow(unused)]
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

        Ok(conversation_detail)
    }

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

    #[allow(unused)]
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
}
