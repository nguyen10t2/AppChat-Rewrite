use crate::{
    api::error,
    modules::message::{
        self, model::InsertMessage, repository::MessageRepository, schema::MessageEntity,
    },
};

#[derive(Clone)]
pub struct MessageRepositoryPg {
    pool: sqlx::PgPool,
}

impl MessageRepositoryPg {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl MessageRepository for MessageRepositoryPg {
    fn get_pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    async fn create<'e, E>(
        &self,
        message: &InsertMessage,
        tx: E,
    ) -> Result<MessageEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let message = sqlx::query_as::<_, MessageEntity>(
            "INSERT INTO messages (conversation_id, sender_id, content) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(message.conversation_id)
        .bind(message.sender_id)
        .bind(&message.content)
        .fetch_one(tx)
        .await?;

        Ok(message)
    }

    async fn find_by_query<'e, E>(
        &self,
        query: &message::model::MessageQuery,
        limit: i32,
        tx: E,
    ) -> Result<Vec<MessageEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        // has index on (conversation_id, created_at DESC NULLS LAST) where deleted_at IS NULL

        let messages = sqlx::query_as::<_, MessageEntity>(
            r#"
            SELECT id, content, conversation_id, sender_id, created_at
            FROM messages
            WHERE conversation_id = $1
              AND deleted_at IS NULL
              AND ($2::timestamptz IS NULL OR created_at < $2)
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(query.conversation_id)
        .bind(query.created_at)
        .bind(limit + 1)
        .fetch_all(tx)
        .await?;

        Ok(messages)
    }
}
