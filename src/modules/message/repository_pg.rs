use crate::{
    api::error,
    modules::message::{
        self, model::InsertMessage, repository::MessageRepository, schema::MessageEntity,
    },
};

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
    async fn create(
        &self,
        message: &InsertMessage,
    ) -> Result<Option<MessageEntity>, error::SystemError> {
        let message = sqlx::query_as::<_, MessageEntity>(
            "INSERT INTO messages (conversation_id, sender_id, reply_to_id, type, content, file_url, is_edited) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
        )
        .bind(message.conversation_id)
        .bind(message.sender_id)
        .bind(message.reply_to_id)
        .bind(&message._type)
        .bind(&message.content)
        .bind(&message.file_url)
        .bind(message.is_edited)
        .fetch_optional(&self.pool)
        .await?;

        Ok(message)
    }

    async fn find_by_query(
        &self,
        query: &message::model::MessageQuery,
        limit: usize,
    ) -> Result<Vec<MessageEntity>, error::SystemError> {
        // has index on (conversation_id, created_at DESC NULLS LAST) where deleted_at IS NULL

        let messages = if let Some(created_at) = query.created_at {
            sqlx::query_as::<_, MessageEntity>(
                "SELECT * FROM messages WHERE conversation_id = $1 AND created_at < $2 AND deleted_at IS NULL ORDER BY created_at DESC NULLS LAST LIMIT $3",
            )
            .bind(query.conversation_id)
            .bind(created_at)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, MessageEntity>(
                "SELECT * FROM messages WHERE conversation_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC NULLS LAST LIMIT $2",
            )
            .bind(query.conversation_id)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(messages)
    }
}
