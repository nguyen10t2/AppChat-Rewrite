use actix_web::{
    middleware::from_fn,
    web::{scope, ServiceConfig},
};

use crate::{middlewares::require_friend, modules::message::handle::*};

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/messages")
            .wrap(from_fn(require_friend))
            .service(scope("/direct").service(send_direct_message))
            .service(scope("/group").service(send_group_message)),
    );
}
