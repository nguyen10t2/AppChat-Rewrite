use actix::Actor;
use actix_cors::Cors;
use actix_web::{
    self,
    middleware::{from_fn, Logger},
    web, App, HttpServer,
};
use std::sync::{Arc, LazyLock};

use crate::{
    configs::{connect_database, RedisCache},
    middlewares::{authentication, authorization},
    modules::{
        conversation::{
            repository_pg::{
                ConversationPgRepository, LastMessagePgRepository, ParticipantPgRepository,
            },
            service::ConversationService,
        },
        file_upload::{repository_pg::FilePgRepository, service::FileUploadService},
        friend::{repository_pg::FriendRepositoryPg, service::FriendService},
        message::{repository_pg::MessageRepositoryPg, service::MessageService},
        user::{repository_pg::UserRepositoryPg, schema::UserRole, service::UserService},
        websocket::{handler::websocket_handler, server::WebSocketServer},
    },
};

mod api;
mod configs;
mod constants;
mod middlewares;
mod modules;
mod test;
mod utils;

pub static ENV: LazyLock<constants::Env> = LazyLock::new(|| {
    dotenvy::dotenv().ok();

    // Setup tracing subscriber cho logging
    tracing_subscriber::fmt().with_target(false).with_thread_ids(true).init();

    tracing::info!("Tracing initialized");
    tracing::info!("Environment variables loaded from .env file");

    constants::Env::default()
});

#[actix_web::get("/")]
async fn health_check(_db_pool: web::Data<sqlx::PgPool>) -> &'static str {
    "Server is running"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db_pool =
        connect_database().await.map_err(|_| std::io::Error::other("Database connection error"))?;

    let redis_pool =
        RedisCache::new().await.map_err(|_| std::io::Error::other("Redis connection error"))?;

    let _user_repo = UserRepositoryPg::new(db_pool.clone());
    let _friend_repo = FriendRepositoryPg::new(db_pool.clone());
    let _participant_repo = ParticipantPgRepository::default();
    let _message_repo = MessageRepositoryPg::new(db_pool.clone());
    let _conversation_repo =
        ConversationPgRepository::new(db_pool.clone(), _participant_repo.clone());
    let _last_message_repo = LastMessagePgRepository::default();
    let _file_repo = FilePgRepository::new(db_pool.clone());
    let ws_server = WebSocketServer::new().start();
    let user_service =
        UserService::with_dependencies(Arc::new(_user_repo.clone()), Arc::new(redis_pool.clone()));
    let friend_service =
        FriendService::with_dependencies(Arc::new(_friend_repo), Arc::new(_user_repo.clone()));
    let file_upload_service = FileUploadService::with_defaults(Arc::new(_file_repo));
    let conversation_service = ConversationService::with_dependencies(
        Arc::new(_conversation_repo.clone()),
        Arc::new(_participant_repo.clone()),
        Arc::new(_message_repo.clone()),
        Arc::new(ws_server.clone()),
    );
    let message_service = MessageService::with_dependencies(
        Arc::new(_conversation_repo.clone()),
        Arc::new(_message_repo),
        Arc::new(_participant_repo),
        Arc::new(_last_message_repo),
        Arc::new(redis_pool),
        Arc::new(ws_server.clone()),
    );

    println!("Starting server at http://{}:{}", ENV.ip.as_str(), ENV.port);
    tracing::info!("Starting HTTP server at http://{}:{}", ENV.ip.as_str(), ENV.port);

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(web::Data::new(user_service.clone()))
            .app_data(web::Data::new(friend_service.clone()))
            .app_data(web::Data::new(file_upload_service.clone()))
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(conversation_service.clone()))
            .app_data(web::Data::new(message_service.clone()))
            .app_data(web::Data::new(ws_server.clone())) // WebSocket server
            .service(health_check)
            // WebSocket endpoint (không cần authentication - auth trong WS handshake)
            .route("/ws", web::get().to(websocket_handler))
            .service(
                web::scope("/api")
                    .default_service(
                        web::route()
                            .guard(actix_web::guard::Method(actix_web::http::Method::OPTIONS))
                            .to(|| async { actix_web::HttpResponse::Ok().finish() }),
                    )
                    .configure(modules::user::route::public_api_configure)
                    .service(
                        web::scope("")
                            .wrap(from_fn(authorization(vec![UserRole::User])))
                            .wrap(from_fn(authentication))
                            .configure(modules::user::route::configure)
                            .configure(modules::friend::route::configure)
                            .configure(modules::conversation::route::configure)
                            .configure(modules::message::route::configure)
                            .configure(modules::file_upload::route::configure::<FilePgRepository>),
                    ),
            )
    })
    .bind((ENV.ip.as_str(), ENV.port))?
    .workers(2)
    .run()
    .await
}
