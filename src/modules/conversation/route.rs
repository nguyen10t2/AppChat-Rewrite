use actix_web::{
    middleware::from_fn,
    web::{scope, ServiceConfig},
};

use crate::{middlewares::require_friend, modules::conversation::handle::*};

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(scope("/conversations").service(get_conversations).service(get_messages).service(
        scope("").wrap(from_fn(require_friend)).service(create_conversation).service(mark_as_seen),
    ));
}
