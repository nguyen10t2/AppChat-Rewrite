use std::sync::Arc;

use uuid::Uuid;

use crate::{
    api::error,
    modules::{
        friend::{
            model::{FriendRequestResponse, FriendResponse},
            repository::FriendRepo,
            schema::{FriendEntity, FriendRequestEntity},
        },
        user::repository::UserRepository,
    },
};

#[derive(Clone)]
pub struct FriendService<R, U>
where
    R: FriendRepo + Send + Sync,
    U: UserRepository + Send + Sync,
{
    friend_repo: Arc<R>,
    user_repo: Arc<U>,
}

impl<R, U> FriendService<R, U>
where
    R: FriendRepo + Send + Sync,
    U: UserRepository + Send + Sync,
{
    pub fn with_dependencies(friend_repo: Arc<R>, user_repo: Arc<U>) -> Self {
        FriendService { friend_repo, user_repo }
    }

    #[allow(dead_code)]
    pub async fn is_friend(
        &self,
        user_id: Uuid,
        friend_id: Uuid,
    ) -> Result<bool, error::SystemError> {
        let friendship = self
            .friend_repo
            .find_friendship(&user_id, &friend_id, self.friend_repo.get_pool())
            .await?;
        Ok(friendship.is_some())
    }

    pub async fn get_friends(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<FriendResponse>, error::SystemError> {
        let friends = self.friend_repo.find_friends(&user_id, self.friend_repo.get_pool()).await?;
        Ok(friends)
    }

    pub async fn remove_friend(
        &self,
        user_id: Uuid,
        friend_id: Uuid,
    ) -> Result<(), error::SystemError> {
        self.friend_repo.delete_friendship(&user_id, &friend_id, self.friend_repo.get_pool()).await
    }

    pub async fn send_friend_request(
        &self,
        sender_id: Uuid,
        receiver_id: Uuid,
        message: Option<String>,
    ) -> Result<FriendRequestEntity, error::SystemError> {
        if receiver_id == sender_id {
            return Err(error::SystemError::bad_request("Cannot send friend request to yourself"));
        }

        if self.user_repo.find_by_id(&receiver_id).await?.is_none() {
            return Err(error::SystemError::not_found("Receiver user not found"));
        }

        let (u1, u2) = if sender_id <= receiver_id {
            (sender_id, receiver_id)
        } else {
            (receiver_id, sender_id)
        };

        let pool = self.friend_repo.get_pool();

        let (friends, requests): (Option<FriendEntity>, Option<FriendRequestEntity>) = tokio::try_join!(
            self.friend_repo.find_friendship(&u1, &u2, pool),
            self.friend_repo.find_friend_request(&sender_id, &receiver_id, pool),
        )?;

        if friends.is_some() {
            return Err(error::SystemError::bad_request("Users are already friends"));
        }

        if requests.is_some() {
            return Err(error::SystemError::bad_request("Friend request already exists"));
        }

        let friend_request = self
            .friend_repo
            .create_friend_request(&sender_id, &receiver_id, &message, pool)
            .await?;

        Ok(friend_request)
    }

    pub async fn accept_friend_request(
        &self,
        user_id: Uuid,
        request_id: Uuid,
    ) -> Result<FriendResponse, error::SystemError> {
        let mut tx = self.friend_repo.get_pool().begin().await?;

        let request = self
            .friend_repo
            .find_friend_request_by_id(&request_id, tx.as_mut())
            .await?
            .ok_or_else(|| error::SystemError::not_found("Friend request not found"))?;

        if request.to_user_id != user_id {
            return Err(error::SystemError::forbidden(
                "You are not allowed to accept this friend request",
            ));
        }

        let (u1, u2) = if request.from_user_id <= request.to_user_id {
            (request.from_user_id, request.to_user_id)
        } else {
            (request.to_user_id, request.from_user_id)
        };

        self.friend_repo.create_friendship(&u1, &u2, tx.as_mut()).await?;

        self.friend_repo.delete_friend_request(&request_id, tx.as_mut()).await?;

        tx.commit().await?;

        let from_user = self
            .user_repo
            .find_by_id(&request.from_user_id)
            .await?
            .ok_or_else(|| error::SystemError::not_found("User not found"))?;

        Ok(FriendResponse::from(from_user))
    }

    pub async fn decline_friend_request(
        &self,
        user_id: Uuid,
        request_id: Uuid,
    ) -> Result<(), error::SystemError> {
        let pool = self.friend_repo.get_pool();

        let request = self
            .friend_repo
            .find_friend_request_by_id(&request_id, pool)
            .await?
            .ok_or_else(|| error::SystemError::not_found("Friend request not found"))?;

        if request.to_user_id != user_id {
            return Err(error::SystemError::forbidden(
                "You are not allowed to decline this friend request",
            ));
        }

        self.friend_repo.delete_friend_request(&request_id, pool).await?;

        Ok(())
    }

    pub async fn get_friend_requests(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError> {
        let pool = self.friend_repo.get_pool();
        let (requests_to, requests_from) = tokio::try_join!(
            self.friend_repo.find_friend_request_to_user(&user_id, pool),
            self.friend_repo.find_friend_request_from_user(&user_id, pool),
        )?;

        let mut all = Vec::with_capacity(requests_to.len() + requests_from.len());
        all.extend(requests_to);
        all.extend(requests_from);
        Ok(all)
    }
}
