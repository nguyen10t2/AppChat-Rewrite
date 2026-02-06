use serde::{Deserialize, Serialize};
use sqlx::prelude::{FromRow, Type};
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, Type, Serialize, Deserialize)]
#[sqlx(type_name = "message_type", rename_all = "lowercase")]
pub enum MessageType {
    Text,
    Image,
    Video,
    File,
    System,
}

#[derive(Debug, Clone, FromRow)]
pub struct MessageEntity {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub reply_to_id: Option<Uuid>,
    #[sqlx(rename = "type")]
    pub _type: MessageType,
    pub content: Option<String>,
    pub file_url: Option<String>,
    pub is_edited: bool,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
