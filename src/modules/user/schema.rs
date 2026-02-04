use serde::{Deserialize, Serialize};
use sqlx::prelude::{FromRow, Type};
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, Type, Serialize, Deserialize)]
#[sqlx(type_name = "user_role", rename_all = "UPPERCASE")]
pub enum UserRole {
    #[sqlx(rename = "ADMIN")]
    Admin,
    #[sqlx(rename = "USER")]
    User,
}

impl TryFrom<&str> for UserRole {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Admin" => Ok(UserRole::Admin),
            "User" => Ok(UserRole::User),
            _ => Err(()),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, FromRow)]
pub struct UserEntity {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub hash_password: String,
    pub role: UserRole,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
