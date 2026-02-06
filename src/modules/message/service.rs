#![allow(unused)]
use std::sync::Arc;
use uuid::Uuid;

use crate::api::error;
use crate::configs::RedisCache;
use crate::modules::message::repository::MessageRepository;
use crate::modules::message::schema::MessageEntity;

#[derive(Clone)]
pub struct MessageService {
    repo: Arc<dyn MessageRepository + Send + Sync>,
    cache: Arc<RedisCache>,
}

impl MessageService {
    pub fn with_dependencies(
        repo: Arc<dyn MessageRepository + Send + Sync>,
        cache: Arc<RedisCache>,
    ) -> Self {
        MessageService { repo, cache }
    }

    pub async fn send_direct_message(
        &self,
        sender_id: Uuid,
        recipient_id: Uuid,
        content: String,
        conversation_id: Option<Uuid>,
    ) -> Result<MessageEntity, error::SystemError> {
        // transaction logic to create or find conversation and send message
        todo!()
    }
}
