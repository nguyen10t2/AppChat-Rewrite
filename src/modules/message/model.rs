use crate::modules::message::schema::MessageEntity;
use crate::modules::message::schema::MessageType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InsertMessage {
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageQuery {
    pub conversation_id: Uuid,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetMessageResponse {
    pub messages: Vec<MessageEntity>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendDirectMessage {
    pub conversation_id: Uuid,
    pub recipient_id: Uuid,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendGroupMessage {
    pub content: String,
}
