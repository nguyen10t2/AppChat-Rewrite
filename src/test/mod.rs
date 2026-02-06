#![allow(dead_code)]

async fn test_find_conversation(pool: sqlx::PgPool) {
    use crate::modules::conversation::repository::ConversationRepository;
    use crate::modules::conversation::repository_pg::ConversationPgRepository;
    use uuid::Uuid;

    let repo = ConversationPgRepository::new(pool);

    let id = Uuid::parse_str("cf5160c2-27b7-4a50-9f8d-3e9ace1f86df").unwrap();

    let result = repo.find_one_conversation_detail(&id).await.unwrap();

    println!("{:#?}", result);

    assert!(result.is_some());
}
