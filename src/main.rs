use actix_web::{
    self, App, HttpServer,
    middleware::{Logger, from_fn},
    web,
};
use std::sync::{Arc, LazyLock};

use crate::{
    configs::{RedisCache, connect_database},
    middlewares::{authentication, authorization},
    modules::{
        friend::{repository_pg::FriendRepositoryPg, service::FriendService},
        user::{repository_pg::UserRepositoryPg, schema::UserRole, service::UserService},
    },
    test::*,
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
    env_logger::init();
    log::info!("Environment variables loaded from .env file");
    constants::Env::default()
});

#[actix_web::get("/")]
async fn health_check(db_pool: web::Data<sqlx::PgPool>) -> &'static str {
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

    let user_service =
        UserService::with_dependencies(Arc::new(_user_repo.clone()), Arc::new(redis_pool.clone()));
    let friend_service =
        FriendService::with_dependencies(Arc::new(_friend_repo), Arc::new(_user_repo.clone()));

    println!("Starting server at http://{}:{}", ENV.ip.as_str(), ENV.port);
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(user_service.clone()))
            .app_data(web::Data::new(friend_service.clone()))
            .app_data(web::Data::new(db_pool.clone()))
            .service(health_check)
            .service(
                web::scope("/api").configure(modules::user::route::public_api_configure).service(
                    web::scope("")
                        .wrap(from_fn(authorization(vec![UserRole::User])))
                        .wrap(from_fn(authentication))
                        .configure(modules::user::route::configure)
                        .configure(modules::friend::route::configure),
                ),
            )
    })
    .bind((ENV.ip.as_str(), ENV.port))?
    .workers(2)
    .run()
    .await
}
