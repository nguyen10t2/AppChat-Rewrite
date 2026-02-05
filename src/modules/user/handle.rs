use actix_web::{
    HttpRequest, cookie::{Cookie, time}, delete, get, patch, post, web
};
use uuid::Uuid;

use crate::{api::{error, success}, utils::ValidatedJson};
use crate::modules::user::model::SignUpResponse;
use crate::modules::user::{model, service::UserService};
use crate::{ENV, middlewares::get_claims};

#[get("/profile")]
pub async fn get_profile(
    user_service: web::Data<UserService>,
    req: HttpRequest,
) -> Result<success::Success<model::UserResponse>, error::Error> {
    let id = get_claims(&req)?.sub;
    let user = user_service.get_by_id(id).await?;
    Ok(success::Success::ok(Some(user)).message("Profile retrieved successfully"))
}

#[get("/{id:[0-9a-fA-F-]{36}}")]
pub async fn get_user(
    user_service: web::Data<UserService>,
    user_id: web::Path<Uuid>,
) -> Result<success::Success<model::UserResponse>, error::Error> {
    let user = user_service.get_by_id(user_id.into_inner()).await?;
    Ok(success::Success::ok(Some(user)).message("User retrieved successfully"))
}

#[patch("/{id:[0-9a-fA-F-]{36}}")]
pub async fn update_user(
    user_service: web::Data<UserService>,
    user_id: web::Path<Uuid>,
    user_data: ValidatedJson<model::UpdateUserModel>,
) -> Result<success::Success<()>, error::Error> {
    user_service.update(user_id.into_inner(), user_data.0).await?;
    Ok(success::Success::ok(None).message("User updated successfully"))
}

#[delete("/{id:[0-9a-fA-F-]{36}}")]
pub async fn delete_user(
    user_service: web::Data<UserService>,
    user_id: web::Path<Uuid>,
) -> Result<success::Success<()>, error::Error> {
    user_service.delete(user_id.into_inner()).await?;
    Ok(success::Success::no_content())
}

#[post("/signup")]
pub async fn sign_up(
    user_service: web::Data<UserService>,
    user_data: ValidatedJson<model::SignUpModel>,
) -> Result<success::Success<SignUpResponse>, error::Error> {
    let user_id = user_service.sign_up(user_data.0).await?;
    Ok(success::Success::created(Some(SignUpResponse { id: user_id })).message("Signup successful"))
}

#[post("/signin")]
pub async fn sign_in(
    user_service: web::Data<UserService>,
    user_data: ValidatedJson<model::SignInModel>,
) -> Result<success::Success<model::SignInResponse>, error::Error> {
    let (access_token, refresh_token) = user_service.sign_in(user_data.0).await?;
    let response = model::SignInResponse { access_token };
    let refresh_cookie = Cookie::build("refresh_token", refresh_token)
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(ENV.refresh_token_expiration as i64))
        .finish();

    Ok(success::Success::ok(Some(response))
        .message("Signin successful")
        .cookies(vec![refresh_cookie]))
}

#[get("/signout")]
pub async fn sign_out(
    user_service: web::Data<UserService>,
    req: HttpRequest,
) -> Result<success::Success<()>, error::Error> {
    let refresh_token = req.cookie("refresh_token").map(|c| c.value().to_string());
    user_service.sign_out(refresh_token).await?;
    let refresh_cookie = Cookie::build("refresh_token", "")
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(0))
        .expires(time::OffsetDateTime::UNIX_EPOCH)
        .finish();

    Ok(success::Success::no_content().cookies(vec![refresh_cookie]))
}

#[post("/refresh")]
pub async fn refresh(
    user_service: web::Data<UserService>,
    req: HttpRequest,
) -> Result<success::Success<model::SignInResponse>, error::Error> {
    let refresh_token = req.cookie("refresh_token").map(|c| c.value().to_string());
    let (access_token, refresh_token) = user_service.refresh(refresh_token).await?;
    let response = model::SignInResponse { access_token };
    let refresh_cookie = Cookie::build("refresh_token", refresh_token)
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(ENV.refresh_token_expiration as i64))
        .finish();
    Ok(success::Success::ok(Some(response))
        .message("Refresh successful")
        .cookies(vec![refresh_cookie]))
}
