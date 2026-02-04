use actix_web::{
    HttpRequest,
    cookie::{Cookie, time},
    get, patch, post, web,
};
use uuid::Uuid;

use crate::api::{error, success};
use crate::modules::user::model::SignUpResponse;
use crate::modules::user::{model, service::UserService};
use crate::{ENV, middlewares::get_claims};

#[get("/{id:[0-9a-fA-F-]{36}}")]
pub async fn get_user(
    user_service: web::Data<UserService>,
    user_id: web::Path<Uuid>,
) -> Result<success::Success<model::UserResponse>, error::Error> {
    let user = user_service.get_by_id(user_id.into_inner()).await?;
    Ok(success::Success::ok(Some(user)).message("User retrieved successfully"))
}

#[get("/profile")]
pub async fn get_profile(
    user_service: web::Data<UserService>,
    req: HttpRequest,
) -> Result<success::Success<model::UserResponse>, error::Error> {
    let id = { get_claims(&req)?.sub };
    let user = user_service.get_by_id(id).await?;
    Ok(success::Success::ok(Some(user)).message("Profile retrieved successfully"))
}

#[patch("/{id:[0-9a-fA-F-]{36}}")]
pub async fn update_user(
    user_service: web::Data<UserService>,
    user_id: web::Path<Uuid>,
    user_data: web::Json<model::UpdateUserModel>,
) -> Result<success::Success<()>, error::Error> {
    user_service.update_user(user_id.into_inner(), user_data.into_inner()).await?;
    Ok(success::Success::ok(None).message("User updated successfully"))
}

#[post("/signup")]
pub async fn sign_up(
    user_service: web::Data<UserService>,
    user_data: web::Json<model::SignUpModel>,
) -> Result<success::Success<SignUpResponse>, error::Error> {
    let user_id = user_service.sign_up(user_data.into_inner()).await?;
    Ok(success::Success::created(Some(SignUpResponse { id: user_id })).message("Signup successful"))
}

#[post("/signin")]
pub async fn sign_in(
    user_service: web::Data<UserService>,
    user_data: web::Json<model::SignInModel>,
) -> Result<success::Success<model::SignInResponse>, error::Error> {
    let (access, refresh) = user_service.sign_in(user_data.into_inner()).await?;
    let response = model::SignInResponse { access_token: access };
    let refresh_cookie = Cookie::build("refresh_token", refresh)
        .path("/")
        .http_only(true)
        .max_age(time::Duration::seconds(ENV.refresh_token_expiration as i64))
        .finish();

    Ok(success::Success::ok(Some(response))
        .message("Signin successful")
        .cookies(vec![refresh_cookie]))
}
