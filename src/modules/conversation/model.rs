use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::modules::conversation::schema::ConversationType;

#[derive(Debug, Clone, FromRow)]
pub struct GroupInfo {
    pub name: String,
    pub created_by: Uuid,
    pub avatar_url: Option<String>,
}

#[derive(FromRow)]
pub struct ConversationRaw {
    pub id: Uuid,
    #[sqlx(rename = "type")]
    pub _type: ConversationType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,

    pub group_name: Option<String>,
    pub group_created_by: Option<Uuid>,
    pub group_avatar_url: Option<String>,

    pub last_content: Option<String>,
    pub last_sender_id: Option<Uuid>,
    pub last_created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ParticipantRow {
    pub user_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub unread_count: i32,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct LastMessageRow {
    pub content: Option<String>,
    pub sender_id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ConversationRow {
    pub conversation_id: Uuid,
    #[sqlx(rename = "type")]
    pub _type: ConversationType,
    pub group_info: Option<GroupInfo>,
    pub last_message: Option<LastMessageRow>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ConversationDetail {
    pub conversation_id: Uuid,
    #[sqlx(rename = "type")]
    pub _type: ConversationType,
    pub group_info: Option<GroupInfo>,
    pub last_message: Option<LastMessageRow>,
    pub participants: Vec<ParticipantRow>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
