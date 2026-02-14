use actix_web::{
    cookie::{self, time, Cookie},
    delete, get, patch, post, web, HttpRequest,
};
use uuid::Uuid;

use crate::modules::user::{model, service::UserService};
use crate::{
    api::{error, success},
    utils::{ValidatedJson, ValidatedQuery},
};
use crate::{middlewares::get_extensions, ENV};
use crate::{
    modules::user::{model::SignUpResponse, repository_pg::UserRepositoryPg},
    utils::Claims,
};
use crate::modules::websocket::presence::{PresenceInfo, PresenceService};

pub type UserSvc = UserService<UserRepositoryPg>;

#[get("/profile")]
pub async fn get_profile(
    user_service: web::Data<UserSvc>,
    req: HttpRequest,
) -> Result<success::Success<model::UserResponse>, error::Error> {
    let id = get_extensions::<Claims>(&req)?.sub;
    let user = user_service.get_by_id(id).await?;
    Ok(success::Success::ok(Some(user)).message("Profile retrieved successfully"))
}

#[get("/{id:[0-9a-fA-F-]{36}}")]
pub async fn get_user(
    user_service: web::Data<UserSvc>,
    user_id: web::Path<Uuid>,
) -> Result<success::Success<model::UserResponse>, error::Error> {
    let user = user_service.get_by_id(user_id.into_inner()).await?;
    Ok(success::Success::ok(Some(user)).message("User retrieved successfully"))
}

#[patch("/{id:[0-9a-fA-F-]{36}}")]
pub async fn update_user(
    user_service: web::Data<UserSvc>,
    user_id: web::Path<Uuid>,
    req: HttpRequest,
    ValidatedJson(user_data): ValidatedJson<model::UpdateUserModel>,
) -> Result<success::Success<()>, error::Error> {
    let auth_user_id = get_extensions::<Claims>(&req)?.sub;
    let target_id = user_id.into_inner();
    if auth_user_id != target_id {
        return Err(error::Error::forbidden("You can only update your own profile"));
    }
    user_service.update(target_id, user_data).await?;
    Ok(success::Success::ok(None).message("User updated successfully"))
}

#[delete("/{id:[0-9a-fA-F-]{36}}")]
pub async fn delete_user(
    user_service: web::Data<UserSvc>,
    user_id: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<success::Success<()>, error::Error> {
    let auth_user_id = get_extensions::<Claims>(&req)?.sub;
    let target_id = user_id.into_inner();
    if auth_user_id != target_id {
        return Err(error::Error::forbidden("You can only delete your own account"));
    }
    user_service.delete(target_id).await?;
    Ok(success::Success::no_content())
}

#[post("/signup")]
pub async fn sign_up(
    user_service: web::Data<UserSvc>,
    ValidatedJson(user_data): ValidatedJson<model::SignUpModel>,
) -> Result<success::Success<SignUpResponse>, error::Error> {
    let user_id = user_service.sign_up(user_data).await?;
    Ok(success::Success::created(Some(SignUpResponse { id: user_id })).message("Signup successful"))
}

#[post("/signin")]
pub async fn sign_in(
    user_service: web::Data<UserSvc>,
    ValidatedJson(user_data): ValidatedJson<model::SignInModel>,
) -> Result<success::Success<model::SignInResponse>, error::Error> {
    let (access_token, refresh_token) = user_service.sign_in(user_data).await?;
    let response = model::SignInResponse { access_token };
    let refresh_cookie = Cookie::build("refresh_token", refresh_token)
        .path("/")
        .http_only(true)
        .same_site(cookie::SameSite::Strict)
        .secure(true)
        .max_age(time::Duration::seconds(ENV.refresh_token_expiration as i64))
        .finish();

    Ok(success::Success::ok(Some(response))
        .message("Signin successful")
        .cookies(vec![refresh_cookie]))
}

#[get("/signout")]
pub async fn sign_out(
    user_service: web::Data<UserSvc>,
    req: HttpRequest,
) -> Result<success::Success<()>, error::Error> {
    let refresh_token = req.cookie("refresh_token").map(|c| c.value().to_string());
    user_service.sign_out(refresh_token).await?;
    let refresh_cookie = Cookie::build("refresh_token", "")
        .path("/")
        .http_only(true)
        .same_site(cookie::SameSite::Strict)
        .secure(true)
        .max_age(time::Duration::seconds(0))
        .expires(time::OffsetDateTime::UNIX_EPOCH)
        .finish();

    Ok(success::Success::no_content().cookies(vec![refresh_cookie]))
}

#[post("/refresh")]
pub async fn refresh(
    user_service: web::Data<UserSvc>,
    req: HttpRequest,
) -> Result<success::Success<model::SignInResponse>, error::Error> {
    let refresh_token = req.cookie("refresh_token").map(|c| c.value().to_string());
    let (access_token, refresh_token) = user_service.refresh(refresh_token).await?;
    let response = model::SignInResponse { access_token };
    let refresh_cookie = Cookie::build("refresh_token", refresh_token)
        .path("/")
        .http_only(true)
        .same_site(cookie::SameSite::Strict)
        .secure(true)
        .max_age(time::Duration::seconds(ENV.refresh_token_expiration as i64))
        .finish();
    Ok(success::Success::ok(Some(response))
        .message("Refresh successful")
        .cookies(vec![refresh_cookie]))
}

#[get("/search")]
pub async fn search_users(
    user_service: web::Data<UserSvc>,
    ValidatedQuery(query): ValidatedQuery<model::UserSearchQuery>,
) -> Result<success::Success<Vec<model::UserResponse>>, error::Error> {
    let users = user_service.search_users(&query.q, query.limit.unwrap_or(10)).await?;
    Ok(success::Success::ok(Some(users)).message("Users found successfully"))
}

/// Batch query presence status cho nhiều users
///
/// POST /users/presence
/// Body: { "user_ids": ["uuid1", "uuid2", ...] }
///
/// Response: [{ "user_id": "...", "is_online": true, "last_seen": null }, ...]
#[post("/presence")]
pub async fn get_presence(
    presence_service: web::Data<PresenceService>,
    body: web::Json<model::PresenceQuery>,
) -> Result<success::Success<Vec<PresenceInfo>>, error::Error> {
    if body.user_ids.is_empty() {
        return Ok(success::Success::ok(Some(vec![])));
    }

    // Giới hạn số lượng users per request để tránh abuse
    if body.user_ids.len() > 200 {
        return Err(error::Error::bad_request("Maximum 200 user IDs per request"));
    }

    let presences = presence_service.get_online_status_batch(&body.user_ids).await?;
    Ok(success::Success::ok(Some(presences)))
}
