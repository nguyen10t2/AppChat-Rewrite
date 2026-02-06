use crate::modules::user::handle::*;
use actix_web::web::{ServiceConfig, scope};

pub fn public_api_configure(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/auth").service(sign_up).service(sign_in).service(sign_out).service(refresh),
    );
}

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/users")
            .service(update_user)
            .service(get_profile)
            .service(get_user)
            .service(delete_user),
    );
}
