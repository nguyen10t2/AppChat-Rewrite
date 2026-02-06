use uuid::Uuid;

use crate::api::error;
use crate::modules::friend::model::{FriendRequestResponse, FriendResponse};
use crate::modules::friend::schema::{FriendEntity, FriendRequestEntity};

#[async_trait::async_trait]
pub trait FriendRepository {
    async fn find_friendship(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
    ) -> Result<Option<FriendEntity>, error::SystemError>;

    async fn find_friends(&self, user_id: &Uuid)
    -> Result<Vec<FriendResponse>, error::SystemError>;

    #[allow(dead_code)]
    async fn create_friendship(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
    ) -> Result<(), error::SystemError>;

    async fn delete_friendship(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
    ) -> Result<(), error::SystemError>;
}

#[async_trait::async_trait]
pub trait FriendRequestRepository {
    async fn find_friend_request(
        &self,
        sender_id: &Uuid,
        receiver_id: &Uuid,
    ) -> Result<Option<FriendRequestEntity>, error::SystemError>;

    async fn find_friend_request_by_id(
        &self,
        request_id: &Uuid,
    ) -> Result<Option<FriendRequestEntity>, error::SystemError>;
    async fn find_friend_request_from_user(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError>;

    async fn find_friend_request_to_user(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError>;

    async fn create_friend_request(
        &self,
        sender_id: &Uuid,
        receiver_id: &Uuid,
        message: &Option<String>,
    ) -> Result<FriendRequestEntity, error::SystemError>;

    async fn delete_friend_request(&self, request_id: &Uuid) -> Result<(), error::SystemError>;
}

#[async_trait::async_trait]
pub trait FriendRepo: FriendRepository + FriendRequestRepository + Send + Sync {
    async fn accept_friend_request_atomic(
        &self,
        request_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Uuid, error::SystemError>;
}
