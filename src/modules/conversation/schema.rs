#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use sqlx::prelude::{FromRow, Type};
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, Type, Serialize, Deserialize)]
#[sqlx(type_name = "conversation_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ConversationType {
    Direct,
    Group,
}

#[derive(Debug, Clone, FromRow)]
pub struct ConversationEntity {
    pub id: Uuid,
    #[sqlx(rename = "type")]
    pub _type: ConversationType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ParticipantEntity {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub unread_count: i32,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct GroupConversationEntity {
    pub conversation_id: Uuid,
    pub name: String,
    pub created_by: Uuid,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct LastMessageEntity {
    pub id: Uuid,
    pub content: Option<String>,
    pub conversation_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
