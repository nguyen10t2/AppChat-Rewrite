use uuid::Uuid;

use crate::{
    api::error, modules::user::model::InsertUser, modules::user::model::UpdateUser,
    modules::user::schema::UserEntity,
};

#[async_trait::async_trait]
pub trait UserRepository {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<UserEntity>, error::SystemError>;
    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserEntity>, error::SystemError>;
    async fn create(&self, user: &InsertUser) -> Result<Uuid, error::SystemError>;
    #[allow(unused)]
    async fn update(&self, id: &Uuid, user: &UpdateUser) -> Result<UserEntity, error::SystemError>;
    async fn delete(&self, id: &Uuid) -> Result<bool, error::SystemError>;

    /// Search users by username or display name (case-insensitive, partial match)
    async fn search_users(
        &self,
        query: &str,
        limit: i32,
    ) -> Result<Vec<UserEntity>, error::SystemError>;
}
