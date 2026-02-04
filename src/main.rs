use actix_web::{
    self, App, HttpServer,
    middleware::{Logger, from_fn},
    web,
};
use std::sync::{Arc, LazyLock};

use crate::{
    configs::{RedisCache, connect_database},
    middlewares::{authentication, authorization},
    modules::user::{repository_pg::UserRepositoryPg, schema::UserRole, service::UserService},
};

mod api;
mod configs;
mod constants;
mod middlewares;
mod modules;
mod utils;

pub static ENV: LazyLock<constants::ENV> = LazyLock::new(|| {
    dotenvy::dotenv().ok();
    env_logger::init();
    log::info!("Environment variables loaded from .env file");
    constants::ENV::new()
});

#[actix_web::get("/")]
async fn health_check() -> &'static str {
    "Server is running"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db_pool = connect_database().await.map_err(|e| {
        log::error!("Failed to connect to the database: {:?}", e);
        std::io::Error::new(std::io::ErrorKind::Other, "Database connection error")
    })?;

    let redis_pool = RedisCache::new().await.map_err(|e| {
        log::error!("Failed to connect to Redis: {:?}", e);
        std::io::Error::new(std::io::ErrorKind::Other, "Redis connection error")
    })?;

    let _user_repo = UserRepositoryPg::new(db_pool);

    let user_service = UserService::with_dependencies(Arc::new(_user_repo), Arc::new(redis_pool));

    println!("Starting server at http://{}:{}", ENV.ip.as_str(), ENV.port);
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(user_service.clone()))
            .service(health_check)
            .service(
                web::scope("/api").configure(modules::user::route::public_api_configure).service(
                    web::scope("")
                        .wrap(from_fn(authorization(vec![UserRole::User])))
                        .wrap(from_fn(authentication))
                        .configure(modules::user::route::configure),
                ),
            )
    })
    .bind((ENV.ip.as_str(), ENV.port))?
    .workers(2)
    .run()
    .await
}
