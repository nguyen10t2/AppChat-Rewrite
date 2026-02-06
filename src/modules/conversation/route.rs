use actix_web::web::{scope, ServiceConfig};

use crate::modules::conversation::handle::*;

pub fn configure(cfg: &mut ServiceConfig) {
    cfg.service(
        scope("/conversations")
            .service(get_conversations)
            .service(get_messages)
            .service(create_conversation),
    );
}
