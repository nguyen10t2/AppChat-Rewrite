use crate::modules::message::schema::MessageType;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct InsertMessage {
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub reply_to_id: Option<Uuid>,
    pub _type: Option<MessageType>,
    pub content: Option<String>,
    pub file_url: Option<String>,
    pub is_edited: bool,
}

#[derive(Debug, Clone)]
pub struct MessageQuery {
    pub conversation_id: Uuid,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}
