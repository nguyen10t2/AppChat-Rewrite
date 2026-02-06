use serde::Serialize;
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FriendEntity {
    pub user_a: Uuid,
    pub user_b: Uuid,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FriendRequestEntity {
    pub id: Uuid,
    pub from_user_id: Uuid,
    pub to_user_id: Uuid,
    pub message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
