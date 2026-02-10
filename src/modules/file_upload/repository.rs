use uuid::Uuid;

use crate::{
    api::error,
    modules::file_upload::{model::NewFile, schema::FileEntity},
};

#[async_trait::async_trait]
pub trait FileRepository {
    fn get_pool(&self) -> &sqlx::Pool<sqlx::Postgres>;

    async fn create<'e, E>(&self, file: &NewFile, tx: E) -> Result<FileEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_by_id(&self, file_id: &Uuid) -> Result<Option<FileEntity>, error::SystemError>;

    async fn delete<'e, E>(&self, file_id: &Uuid, tx: E) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;
}
