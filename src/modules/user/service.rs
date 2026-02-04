use log::info;
use std::sync::Arc;
use uuid::Uuid;

use crate::ENV;
use crate::api::error;
use crate::configs::RedisCache;

use crate::modules::user::model::{
    SignInModel, SignUpModel, UpdateUser, UpdateUserModel, UserResponse,
};
use crate::modules::user::{model::InsertUser, repository::UserRepository};
use crate::utils::{Claims, hash_password, verify_password};

#[derive(Clone)]
pub struct UserService {
    repo: Arc<dyn UserRepository + Send + Sync>,
    cache: Arc<RedisCache>,
}

impl UserService {
    pub fn with_dependencies(
        repo: Arc<dyn UserRepository + Send + Sync>,
        cache: Arc<RedisCache>,
    ) -> Self {
        info!("UserService initialized with dependencies");
        UserService { repo, cache }
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<UserResponse, error::SystemError> {
        let key = format!("user:{}", id);
        if let Some(cached_user) = self.cache.get::<UserResponse>(&key).await? {
            info!("User {} found in cache", id);
            return Ok(cached_user);
        }
        let user_entity = self.repo.find_by_id(&id).await?;
        if let Some(entity) = user_entity {
            self.cache.set(&key, &UserResponse::from(entity.clone()), 3600).await?;
            info!("User {} cached", id);
            Ok(UserResponse::from(entity))
        } else {
            Err(error::SystemError::not_found("User not found"))
        }
    }

    pub async fn update_user(
        &self,
        id: Uuid,
        user: UpdateUserModel,
    ) -> Result<(), error::SystemError> {
        if user.username.is_none()
            && user.email.is_none()
            && user.first_name.is_none()
            && user.last_name.is_none()
            && user.avatar_url.is_none()
            && user.bio.is_none()
            && user.phone.is_none()
        {
            return Err(error::SystemError::bad_request("No fields to update"));
        }

        let update_user = UpdateUser {
            username: user.username,
            email: user.email,
            display_name: match (user.first_name, user.last_name) {
                (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
                _ => None,
            },
            avatar_url: user.avatar_url,
            bio: user.bio,
            phone: user.phone,
        };

        self.repo.update(&id, &update_user).await?;

        let key = format!("user:{}", id);
        self.cache.delete(&key).await?;
        Ok(())
    }

    pub async fn sign_up(&self, user: SignUpModel) -> Result<uuid::Uuid, error::SystemError> {
        let hash_password = hash_password(&user.password)?;

        let new_user = InsertUser {
            username: user.username,
            email: user.email,
            hash_password,
            display_name: format!("{} {}", user.first_name, user.last_name),
        };

        let user_id = self.repo.create(&new_user).await?;
        Ok(user_id)
    }

    pub async fn sign_in(&self, user: SignInModel) -> Result<(String, String), error::SystemError> {
        let user_entity = self
            .repo
            .find_by_username(&user.username)
            .await?
            .ok_or_else(|| error::SystemError::unauthorized("Invalid username or password"))?;

        let valid = verify_password(&user_entity.hash_password, &user.password)?;
        if !valid {
            return Err(error::SystemError::unauthorized("Invalid username or password"));
        }

        let access_token =
            Claims::new(&user_entity.id, &user_entity.role, ENV.access_token_expiration, None)
                .encode(ENV.jwt_secret.as_ref())?;

        let jti = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

        let refresh_token = Claims::new(
            &user_entity.id,
            &user_entity.role,
            ENV.refresh_token_expiration,
            Some(&jti),
        )
        .encode(ENV.jwt_secret.as_ref())?;

        let refresh_key = format!("refresh_token:{jti}");
        self.cache
            .set(&refresh_key, &user_entity.id, ENV.refresh_token_expiration as usize)
            .await?;

        Ok((access_token, refresh_token))
    }
}
