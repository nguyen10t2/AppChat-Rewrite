use actix_web::{
    middleware::from_fn,
    web::{scope, ServiceConfig},
};

use crate::{
    middlewares::{require_friend, require_group_member},
    modules::message::handle::*,
};

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/messages")
            .service(scope("/direct").wrap(from_fn(require_friend)).service(send_direct_message))
            .service(
                scope("/group").wrap(from_fn(require_group_member)).service(send_group_message),
            )
            .service(delete_message)
            .service(edit_message),
    );
}
