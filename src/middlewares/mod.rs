use std::rc::Rc;

use crate::{ENV, api::error, modules::user::schema::UserRole, utils::Claims};
use actix_web::{
    Error, HttpMessage, HttpRequest, body::MessageBody, dev::{ServiceRequest, ServiceResponse}, middleware::Next
};
use futures_util::{FutureExt, future::LocalBoxFuture};
use log::info;

pub async fn authentication<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<B>, Error>
where
    B: MessageBody + 'static,
{
    info!("Authenticating request: {}", req.path());
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

pub fn authorization<B>(
    allowd_roles: Vec<UserRole>,
) -> impl Fn(
    ServiceRequest,
    Next<B>,
) -> LocalBoxFuture<'static, Result<ServiceResponse<B>, actix_web::Error>>
where
    B: MessageBody + 'static,
{
    let allowd_roles = Rc::new(allowd_roles);
    move |req: ServiceRequest, next: Next<B>| {
        let roles = allowd_roles.clone();
        async move {
            let role = {
                get_claims(&req.request())?.role
            };

            info!("Authorizing request for role: {:?}", role);

            if !roles.contains(&role) {
                return Err(error::Error::forbidden("Forbidden").into());
            }
            next.call(req).await
        }
        .boxed_local()
    }
}
