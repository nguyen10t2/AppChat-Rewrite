use core::str;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::modules::user::schema::UserEntity;

#[derive(Deserialize, Validate)]
pub struct SignUpModel {
    #[validate(length(min = 3, message = "Username must be at least 3 characters long"))]
    pub username: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 6, message = "Password must be at least 6 characters long"))]
    pub password: String,
    #[validate(length(min = 1, message = "First name cannot be empty"))]
    pub first_name: String,
    #[validate(length(min = 1, message = "Last name cannot be empty"))]
    pub last_name: String,
}

#[derive(Deserialize, Validate)]
pub struct SignInModel {
    #[validate(length(min = 3, message = "Username must be at least 3 characters long"))]
    pub username: String,
    #[validate(length(min = 6, message = "Password must be at least 6 characters long"))]
    pub password: String,
}

#[allow(unused)]
#[derive(Deserialize, Validate)]
pub struct RefreshTokenModel {
    #[validate(length(min = 1, message = "Refresh token cannot be empty"))]
    pub refresh_token: String,
}

use crate::utils::double_option;

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserModel {
    #[validate(length(min = 3, message = "Username must be at least 3 characters long"))]
    pub username: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    #[validate(length(min = 1, message = "First name cannot be empty"))]
    pub first_name: Option<String>,
    #[validate(length(min = 1, message = "Last name cannot be empty"))]
    pub last_name: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub avatar_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub bio: Option<Option<String>>,
    #[validate(length(min = 10, message = "Phone number must be at least 10 digits long"))]
    #[serde(default, deserialize_with = "double_option")]
    pub phone: Option<Option<String>>,
}

impl UpdateUserModel {
    pub fn is_empty(&self) -> bool {
        self.username.is_none()
            && self.email.is_none()
            && self.first_name.is_none()
            && self.last_name.is_none()
            && self.avatar_url.is_none()
            && self.bio.is_none()
            && self.phone.is_none()
    }
}

pub struct InsertUser {
    pub username: String,
    pub email: String,
    pub hash_password: String,
    pub display_name: String,
}

#[allow(unused)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<Option<String>>,
    pub bio: Option<Option<String>>,
    pub phone: Option<Option<String>>,
}

#[derive(Serialize)]
pub struct SignUpResponse {
    pub id: uuid::Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignInResponse {
    pub access_token: String,
}

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchQuery {
    #[validate(length(min = 2, message = "Search query must be at least 2 characters"))]
    pub q: String,
    #[validate(range(min = 1, max = 50, message = "Limit must be between 1 and 50"))]
    pub limit: Option<i32>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub phone: Option<String>,
}

impl From<UserEntity> for UserResponse {
    fn from(entity: UserEntity) -> Self {
        UserResponse {
            id: entity.id,
            username: entity.username,
            email: entity.email,
            display_name: entity.display_name,
            avatar_url: entity.avatar_url,
            bio: entity.bio,
            phone: entity.phone,
        }
    }
}
