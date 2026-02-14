use actix_web::{
    body::{BoxBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
    web, Error, FromRequest, HttpMessage, HttpRequest,
};
use futures_util::{future::LocalBoxFuture, FutureExt};
use serde::{Serialize, Deserialize};
use std::rc::Rc;
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::error,
    modules::{
        conversation::handle::ConversationSvc, friend::handle::FriendSvc, user::schema::UserRole,
    },
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
    if req.method() == actix_web::http::Method::OPTIONS {
        return next.call(req).await;
    }

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

pub fn get_extensions<T: Clone + 'static>(req: &HttpRequest) -> Result<T, error::Error> {
    let extensions = req.extensions();

    let claims =
        extensions.get::<T>().ok_or_else(|| error::Error::unauthorized("Unauthorized"))?.clone();

    Ok(claims)
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
    let allowed_roles = Rc::new(allowed_roles);
    move |req: ServiceRequest, next: Next<B>| {
        let roles = allowed_roles.clone();
        async move {
            let role = get_extensions::<Claims>(req.request())?.role;

            if !roles.contains(&role) {
                return Err(error::Error::forbidden("No permission").into());
            }
            next.call(req).await
        }
        .boxed_local()
    }
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct RequireBody {
    pub recipient_id: Option<Uuid>,
    pub member_ids: Option<Vec<Uuid>>,
}

pub async fn require_friend(
    mut req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    // req body require_friend must have recipient_id and member_ids (and any)
    let (http_req, payload) = req.parts_mut();

    let body_bytes = web::Bytes::from_request(http_req, payload)
        .await
        .map_err(|e| error::Error::bad_request(format!("Failed to read request body: {}", e)))?;

    let parsed = serde_json::from_slice::<RequireBody>(&body_bytes)
        .map_err(|e| error::Error::bad_request(format!("Invalid Body: {}", e)))?;

    if parsed.recipient_id.is_none() && parsed.member_ids.is_none() {
        return Err(error::Error::bad_request(
            "Either recipient_id or member_ids must be provided",
        )
        .into());
    }

    parsed.validate().map_err(|e| error::Error::BadRequest(e.to_string().into()))?;

    let user_id = get_extensions::<Claims>(req.request())?.sub;

    let friend_svc = req.app_data::<web::Data<FriendSvc>>().ok_or(error::Error::InternalServer)?;

    if let Some(recipient_id) = parsed.recipient_id {
        let (user_a, user_b) =
            if user_id < recipient_id { (user_id, recipient_id) } else { (recipient_id, user_id) };

        if !friend_svc.is_friend(user_a, user_b).await.map_err(|_| error::Error::InternalServer)? {
            return Err(error::Error::forbidden("You are not friends with the recipient").into());
        }
    }

    if let Some(member_ids) = parsed.member_ids {
        let futures = member_ids.into_iter().map(|id| {
            let service = friend_svc.clone();
            async move {
                let (a, b) = if user_id < id { (user_id, id) } else { (id, user_id) };

                service.is_friend(a, b).await
            }
        });

        let results = futures_util::future::try_join_all(futures)
            .await
            .map_err(|_| error::Error::InternalServer)?;

        if !results.into_iter().all(|v| v) {
            return Err(error::Error::forbidden("You are not friends with all members").into());
        }
    }

    req.set_payload(body_bytes.into());

    next.call(req).await
}

#[derive(Deserialize)]
pub struct RequireGroupMemberParams {
    pub conversation_id: Uuid,
}

pub async fn require_group_member(
    mut req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let (http_req, payload) = req.parts_mut();

    let body_bytes = web::Bytes::from_request(http_req, payload)
        .await
        .map_err(|_| error::Error::bad_request("Invalid Body"))?;

    let parsed = serde_json::from_slice::<RequireGroupMemberParams>(&body_bytes)
        .map_err(|_| error::Error::bad_request("Invalid Body"))?;

    let user_id = get_extensions::<Claims>(req.request())?.sub;

    let conv_svc =
        req.app_data::<web::Data<ConversationSvc>>().ok_or(error::Error::InternalServer)?;

    let (conversation, is_member) = conv_svc
        .get_conversation_and_check_membership(parsed.conversation_id, user_id)
        .await
        .map_err(|_| error::Error::not_found("Conversation not found"))?;

    if !is_member {
        return Err(error::Error::forbidden("You are not a member of this conversation").into());
    }

    req.set_payload(body_bytes.into());

    req.extensions_mut().insert(conversation);

    next.call(req).await
}
