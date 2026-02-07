#![allow(dead_code)]

use crate::modules::conversation::repository_pg::ParticipantPgRepository;

async fn test_find_conversation(pool: sqlx::PgPool) {
    use crate::modules::conversation::repository::ConversationRepository;
    use crate::modules::conversation::repository_pg::ConversationPgRepository;
    use uuid::Uuid;

    let participant_repo = ParticipantPgRepository::default();
    let repo = ConversationPgRepository::new(pool, participant_repo);
    let id = Uuid::parse_str("cf5160c2-27b7-4a50-9f8d-3e9ace1f86df").unwrap();

    let result = repo.find_one_conversation_detail(&id).await.unwrap();

    println!("{:#?}", result);

    assert!(result.is_some());
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_chao_endpoint() {
        let app = test::init_service(App::new().service(crate::chao)).await;
        let req = test::TestRequest::get().uri("/chao").to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());

        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(json.get("message").is_some());
        assert!(json.get("welcome").is_some());
        assert_eq!(json["message"], "Chào mừng bạn đến với App Chat!");
        assert_eq!(json["welcome"], "Welcome to App Chat!");
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["status"], "online");
    }

    #[actix_web::test]
    async fn test_health_check_endpoint() {
        let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").unwrap_or_default())
            .await
            .ok();

        if let Some(pool) = pool {
            let app = test::init_service(
                App::new()
                    .app_data(actix_web::web::Data::new(pool))
                    .service(crate::health_check),
            )
            .await;
            let req = test::TestRequest::get().uri("/").to_request();
            let resp = test::call_service(&app, req).await;

            assert!(resp.status().is_success());

            let body = test::read_body(resp).await;
            let body_str = std::str::from_utf8(&body).unwrap();

            assert_eq!(body_str, "Server is running");
        }
    }
}

