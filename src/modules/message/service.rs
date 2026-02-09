#![allow(unused)]
use jsonwebtoken::signature::digest::crypto_common::ParBlocks;
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
use crate::modules::message::schema::{MessageEntity, MessageType};

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
}

impl<M, C, P, L> MessageService<M, C, P, L>
where
    C: ConversationRepository + Send + Sync,
    M: MessageRepository + Send + Sync,
    P: ParticipantRepository + Send + Sync,
    L: LastMessageRepository + Send + Sync,
{
    pub fn with_dependencies(
        conversation_repo: Arc<C>,
        message_repo: Arc<M>,
        participant_repo: Arc<P>,
        last_message_repo: Arc<L>,
        cache: Arc<RedisCache>,
    ) -> Self {
        MessageService {
            conversation_repo,
            message_repo,
            participant_repo,
            last_message_repo,
            cache,
        }
    }

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
                    content: Some(content),
                    created_at: message.created_at,
                },
                tx.as_mut(),
            )
            .await?;

        tx.commit().await?;

        Ok(message)
    }

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
            .increment_unread_count(&conversation_id, &sender_id, tx.as_mut())
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

        Ok(message)
    }
}
