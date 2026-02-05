use std::sync::Arc;
use uuid::Uuid;

use crate::ENV;
use crate::api::error;
use crate::configs::RedisCache;
use crate::modules::user::CACHE_TTL;
use crate::modules::user::model::{
    SignInModel, SignUpModel, UpdateUser, UpdateUserModel, UserResponse,
};
use crate::modules::user::{model::InsertUser, repository::UserRepository};
use crate::utils::{Claims, TypeClaims, hash_password, verify_password};

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
        UserService { repo, cache }
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<UserResponse, error::SystemError> {
        let key = format!("user:{}", id);
        if let Some(cached_user) = self.cache.get::<UserResponse>(&key).await? {
            return Ok(cached_user);
        }
        let user_entity = self.repo.find_by_id(&id).await?;
        if let Some(entity) = user_entity {
            self.cache.set(&key, &UserResponse::from(entity.clone()), CACHE_TTL).await?;
            Ok(UserResponse::from(entity))
        } else {
            Err(error::SystemError::not_found("User not found"))
        }
    }

    pub async fn update(
        &self,
        id: Uuid,
        user: UpdateUserModel,
    ) -> Result<UserResponse, error::SystemError> {
        println!("UpdateUserModel: {:?}", user);
        if user.is_empty() {
            return Err(error::SystemError::bad_request("No fields to update"));
        }

        let update_user = UpdateUser {
            username: user.username,
            email: user.email,
            display_name: match (user.first_name, user.last_name) {
                (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
                (Some(first), None) => Some(first),
                (None, Some(last)) => Some(last),
                _ => None,
            },
            avatar_url: user.avatar_url,
            bio: user.bio,
            phone: user.phone,
        };

        let updated_user = self.repo.update(&id, &update_user).await?;

        let key = format!("user:{}", id);
        let response = UserResponse::from(updated_user);
        self.cache.set(&key, &response, CACHE_TTL).await?;

        Ok(response)
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), error::SystemError> {
        let deleted = self.repo.delete(&id).await?;
        if !deleted {
            return Err(error::SystemError::not_found("User not found"));
        }
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
            Claims::new(&user_entity.id, &user_entity.role, ENV.access_token_expiration)
                .with_type(TypeClaims::AccessToken)
                .encode(ENV.jwt_secret.as_ref())?;

        let jti = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

        let refresh_token =
            Claims::new(&user_entity.id, &user_entity.role, ENV.refresh_token_expiration)
                .with_jti(jti)
                .with_type(TypeClaims::RefreshToken)
                .encode(ENV.jwt_secret.as_ref())?;

        let refresh_key = format!("refresh_token:{jti}");
        self.cache
            .set(&refresh_key, &user_entity.id, ENV.refresh_token_expiration as usize)
            .await?;

        Ok((access_token, refresh_token))
    }

    pub async fn sign_out(&self, refresh_token: Option<String>) -> Result<(), error::SystemError> {
        let Some(token) = refresh_token else {
            return Ok(());
        };

        let payload = Claims::decode(&token, ENV.jwt_secret.as_ref())?;

        let Some(TypeClaims::RefreshToken) = payload._type else {
            return Ok(());
        };

        let Some(jti) = payload.jti else {
            return Ok(());
        };

        let refresh_key = format!("refresh_token:{jti}");
        self.cache.delete(&refresh_key).await?;

        Ok(())
    }

    pub async fn refresh(
        &self,
        old_refresh_token: Option<String>,
    ) -> Result<(String, String), error::SystemError> {
        let invalid = || error::SystemError::unauthorized("Invalid token");

        let Some(old_refresh_token) = old_refresh_token else {
            return Err(invalid());
        };

        let payload = Claims::decode(&old_refresh_token, ENV.jwt_secret.as_ref())?;

        let Some(TypeClaims::RefreshToken) = payload._type else {
            return Err(invalid());
        };

        let Some(jti) = payload.jti else {
            return Err(invalid());
        };

        let old_key = format!("refresh_token:{jti}");

        if self.cache.get::<String>(&old_key).await?.is_none() {
            return Err(invalid());
        }

        self.cache.delete(&old_key).await?;

        let new_jti = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let new_key = format!("refresh_token:{new_jti}");

        let new_access_token =
            Claims::new(&payload.sub, &payload.role, ENV.access_token_expiration)
                .with_type(TypeClaims::AccessToken)
                .encode(ENV.jwt_secret.as_ref())?;

        let new_refresh_token =
            Claims::new(&payload.sub, &payload.role, ENV.refresh_token_expiration)
                .with_jti(new_jti)
                .with_type(TypeClaims::RefreshToken)
                .encode(ENV.jwt_secret.as_ref())?;

        self.cache.set(&new_key, &payload.sub, ENV.refresh_token_expiration as usize).await?;

        Ok((new_access_token, new_refresh_token))
    }
}
