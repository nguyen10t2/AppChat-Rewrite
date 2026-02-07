use actix_web::{
    body::{to_bytes, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    Error, HttpMessage, HttpRequest,
};
use futures_util::{future::LocalBoxFuture, FutureExt};
use serde::Deserialize;
use std::rc::Rc;
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::error,
    modules::{conversation::schema::ConversationEntity, user::schema::UserRole},
    utils::Claims,
    ENV,
};

pub async fn authentication<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error>
where
    B: MessageBody + 'static,
{
    let auth = req.headers().get("Authorization").and_then(|h| h.to_str().ok());
    let token = match auth.and_then(|h| h.strip_prefix("Bearer ")) {
        Some(t) => t,
        None => {
            return Err(error::Error::unauthorized("Token Invalid or Expired").into());
        }
    };

    let claims = Claims::decode(token, ENV.jwt_secret.as_ref())
        .map_err(|_| error::Error::forbidden("Token Invalid or Expired"))?;

    req.extensions_mut().insert(claims);

    next.call(req).await
}

pub fn get_claims(req: &HttpRequest) -> Result<Claims, error::Error> {
    let extensions = req.extensions();

    let claims = extensions
        .get::<Claims>()
        .ok_or_else(|| error::Error::unauthorized("Unauthorized"))?
        .clone();

    Ok(claims)
}

pub fn get_conversation(req: &HttpRequest) -> Result<ConversationEntity, error::Error> {
    let extensions = req.extensions();

    let conversation = extensions
        .get::<ConversationEntity>()
        .ok_or_else(|| error::Error::unauthorized("Unauthorized"))?
        .clone();

    Ok(conversation)
}

pub fn authorization<B>(
    allowed_roles: Vec<UserRole>,
) -> impl Fn(
    ServiceRequest,
    Next<B>,
) -> LocalBoxFuture<'static, Result<ServiceResponse<B>, actix_web::Error>>
where
    B: MessageBody + 'static,
{
    let allowd_roles = Rc::new(allowed_roles);
    move |req: ServiceRequest, next: Next<B>| {
        let roles = allowd_roles.clone();
        async move {
            let role = get_claims(req.request())?.role;

            if !roles.contains(&role) {
                return Err(error::Error::forbidden("No permission").into());
            }
            next.call(req).await
        }
        .boxed_local()
    }
}

#[derive(Deserialize, Validate)]
pub struct RequireBody {
    pub recipient_id: Option<Uuid>,
    #[validate(length(min = 1))]
    pub member_ids: Option<Vec<Uuid>>,
}

pub async fn require_friend<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse, Error> {
    // req body require_friend must have recipient_id and member_ids (and any)
    todo!()
}
