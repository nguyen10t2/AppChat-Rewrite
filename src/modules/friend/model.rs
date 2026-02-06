use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;
use validator::Validate;

use crate::modules::user::schema::UserEntity;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FriendResponse {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

impl From<UserEntity> for FriendResponse {
    fn from(user: UserEntity) -> Self {
        FriendResponse {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IdOrInfo {
    Id(Uuid),
    Info(FriendResponse),
}

#[derive(sqlx::FromRow)]
pub struct FriendUserRow {
    pub req_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FriendRequestResponse {
    pub id: Uuid,
    pub from: IdOrInfo,
    pub to: IdOrInfo,
    pub message: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequestBody {
    pub recipient_id: Uuid,
    pub message: Option<String>,
}
