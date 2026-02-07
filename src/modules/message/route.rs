use actix_web::web::{scope, ServiceConfig};

use crate::modules::message::handle::*;

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/messages")
            .service(scope("/direct").service(send_direct_message))
            .service(scope("/group").service(send_group_message)),
    );
}
