use crate::modules::message::model::{InsertMessage, MessageQuery};
use crate::{api::error, modules::message::schema::MessageEntity};

#[async_trait::async_trait]
pub trait MessageRepository {
    fn get_pool(&self) -> &sqlx::PgPool;
    async fn create<'e, E>(
        &self,
        message: &InsertMessage,
        tx: E,
    ) -> Result<MessageEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_by_query<'e, E>(
        &self,
        query: &MessageQuery,
        limit: i32,
        tx: E,
    ) -> Result<Vec<MessageEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;
}
