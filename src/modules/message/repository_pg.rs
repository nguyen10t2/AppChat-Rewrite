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

    async fn find_by_id<'e, E>(
        &self,
        message_id: &uuid::Uuid,
        tx: E,
    ) -> Result<Option<MessageEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let message = sqlx::query_as::<_, MessageEntity>(
            "SELECT * FROM messages WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(message_id)
        .fetch_optional(tx)
        .await?;
        Ok(message)
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
            SELECT *
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

    async fn delete_message<'e, E>(
        &self,
        message_id: &uuid::Uuid,
        user_id: &uuid::Uuid,
        tx: E,
    ) -> Result<bool, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        // Soft delete: chỉ cho phép xóa tin nhắn của chính mình
        let rows = sqlx::query(
            r#"
            UPDATE messages
            SET deleted_at = NOW()
            WHERE id = $1
              AND sender_id = $2
              AND deleted_at IS NULL
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .execute(tx)
        .await?
        .rows_affected();

        Ok(rows > 0)
    }

    async fn edit_message<'e, E>(
        &self,
        message_id: &uuid::Uuid,
        user_id: &uuid::Uuid,
        new_content: &str,
        tx: E,
    ) -> Result<Option<MessageEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        // Edit message: chỉ cho phép sửa tin nhắn của chính mình
        let message = sqlx::query_as::<_, MessageEntity>(
            r#"
            UPDATE messages
            SET content = $1,
                updated_at = NOW()
            WHERE id = $2
              AND sender_id = $3
              AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(new_content)
        .bind(message_id)
        .bind(user_id)
        .fetch_optional(tx)
        .await?;

        Ok(message)
    }

    async fn get_last_message_by_conversation<'e, E>(
        &self,
        conversation_id: &uuid::Uuid,
        tx: E,
    ) -> Result<Option<MessageEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let message = sqlx::query_as::<_, MessageEntity>(
            r#"
            SELECT *
            FROM messages
            WHERE conversation_id = $1
              AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(conversation_id)
        .fetch_optional(tx)
        .await?;

        Ok(message)
    }
}
