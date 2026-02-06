use uuid::Uuid;

use crate::api::error;
use crate::modules::friend::model::{FriendRequestResponse, FriendResponse};
use crate::modules::friend::schema::{FriendEntity, FriendRequestEntity};

#[async_trait::async_trait]
pub trait FriendRepository {
    async fn find_friendship<'e, E>(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
        tx: E,
    ) -> Result<Option<FriendEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_friends<'e, E>(
        &self,
        user_id: &Uuid,
        tx: E,
    ) -> Result<Vec<FriendResponse>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    #[allow(dead_code)]
    async fn create_friendship<'e, E>(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn delete_friendship<'e, E>(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;
}

#[async_trait::async_trait]
pub trait FriendRequestRepository {
    async fn find_friend_request<'e, E>(
        &self,
        sender_id: &Uuid,
        receiver_id: &Uuid,
        tx: E,
    ) -> Result<Option<FriendRequestEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_friend_request_by_id<'e, E>(
        &self,
        request_id: &Uuid,
        tx: E,
    ) -> Result<Option<FriendRequestEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_friend_request_from_user<'e, E>(
        &self,
        user_id: &Uuid,
        tx: E,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_friend_request_to_user<'e, E>(
        &self,
        user_id: &Uuid,
        tx: E,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn create_friend_request<'e, E>(
        &self,
        sender_id: &Uuid,
        receiver_id: &Uuid,
        message: &Option<String>,
        tx: E,
    ) -> Result<FriendRequestEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn delete_friend_request<'e, E>(
        &self,
        request_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;
}

#[async_trait::async_trait]
pub trait FriendRepo: FriendRepository + FriendRequestRepository + Send + Sync {
    fn get_pool(&self) -> &sqlx::PgPool;
}
