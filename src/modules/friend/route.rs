use crate::modules::friend::handle::*;
use actix_web::web::{ServiceConfig, scope};

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/friends")
            .service(send_friend_request)
            .service(accept_friend_request)
            .service(decline_friend_request)
            .service(list_friends)
            .service(list_friend_requests)
            .service(remove_friend),
    );
}
