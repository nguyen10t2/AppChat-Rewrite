#![allow(unused)]
use actix::Addr;
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
use crate::modules::websocket::events::BroadcastToRoom;
use crate::modules::websocket::message::ServerMessage;
use crate::modules::websocket::server::WebSocketServer;

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

        // BUG-003 FIX: Update conversation timestamp when new message is sent
        self.conversation_repo.update_timestamp(&conversation.id, tx.as_mut()).await?;

        tx.commit().await?;

        // Broadcast new message to conversation room
        let message_json = serde_json::to_value(&message).map_err(|e| {
            error::SystemError::internal_error(format!("Failed to serialize message: {}", e))
        })?;

        self.ws_server.do_send(BroadcastToRoom {
            conversation_id: conversation.id,
            message: ServerMessage::NewMessage {
                conversation_id: conversation.id,
                message: message_json,
            },
            skip_user_id: Some(sender_id), // Sender không cần nhận lại
        });

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

        // BUG-001 FIX: Increment unread count for all participants EXCEPT the sender
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

        // BUG-003 FIX: Update conversation timestamp when new message is sent
        self.conversation_repo.update_timestamp(&conversation_id, tx.as_mut()).await?;

        tx.commit().await?;

        // Broadcast new message to conversation room
        let message_json = serde_json::to_value(&message).map_err(|e| {
            error::SystemError::internal_error(format!("Failed to serialize message: {}", e))
        })?;

        self.ws_server.do_send(BroadcastToRoom {
            conversation_id,
            message: ServerMessage::NewMessage { conversation_id, message: message_json },
            skip_user_id: Some(sender_id), // Sender không cần nhận lại
        });

        Ok(message)
    }

    /// Delete a message by ID (soft delete)
    pub async fn delete_message(
        &self,
        message_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), error::SystemError> {
        let mut tx = self.conversation_repo.get_pool().begin().await?;

        // Get message to verify ownership and get conversation_id
        let message = self
            .message_repo
            .find_by_id(&message_id, tx.as_mut())
            .await?
            .ok_or_else(|| error::SystemError::not_found("Message not found"))?;

        // Verify user is the sender
        if message.sender_id != user_id {
            return Err(error::SystemError::forbidden("You can only delete your own messages"));
        }

        // Delete the message
        let deleted = self.message_repo.delete_message(&message_id, &user_id, tx.as_mut()).await?;

        if !deleted {
            return Err(error::SystemError::not_found("Message not found or already deleted"));
        }

        tx.commit().await?;

        // Broadcast message deletion to conversation room
        self.ws_server.do_send(BroadcastToRoom {
            conversation_id: message.conversation_id,
            message: ServerMessage::MessageDeleted {
                conversation_id: message.conversation_id,
                message_id,
            },
            skip_user_id: None, // Broadcast to all including sender
        });

        Ok(())
    }

    /// Edit a message by ID (only content can be edited)
    pub async fn edit_message(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        new_content: String,
    ) -> Result<MessageEntity, error::SystemError> {
        let mut tx = self.conversation_repo.get_pool().begin().await?;

        // Get message to verify ownership and get conversation_id
        let message = self
            .message_repo
            .find_by_id(&message_id, tx.as_mut())
            .await?
            .ok_or_else(|| error::SystemError::not_found("Message not found"))?;

        // Verify user is the sender
        if message.sender_id != user_id {
            return Err(error::SystemError::forbidden("You can only edit your own messages"));
        }

        // Edit the message
        let edited_message = self
            .message_repo
            .edit_message(&message_id, &user_id, &new_content, tx.as_mut())
            .await?
            .ok_or_else(|| error::SystemError::not_found("Message not found"))?;

        tx.commit().await?;

        // Broadcast message edit to conversation room
        self.ws_server.do_send(BroadcastToRoom {
            conversation_id: message.conversation_id,
            message: ServerMessage::MessageEdited {
                conversation_id: message.conversation_id,
                message_id,
                new_content,
            },
            skip_user_id: None, // Broadcast to all including sender
        });

        Ok(edited_message)
    }
}
