use uuid::Uuid;

use crate::{
    api::error,
    modules::file_upload::{model::NewFile, repository::FileRepository, schema::FileEntity},
};

#[derive(Clone)]
pub struct FilePgRepository {
    pool: sqlx::PgPool,
}

impl FilePgRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl FileRepository for FilePgRepository {
    fn get_pool(&self) -> &sqlx::Pool<sqlx::Postgres> {
        &self.pool
    }

    async fn create<'e, E>(&self, file: &NewFile, tx: E) -> Result<FileEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let entity = sqlx::query_as::<_, FileEntity>(
            r#"
            INSERT INTO files (filename, original_filename, mime_type, file_size, storage_path, uploaded_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&file.filename)
        .bind(&file.original_filename)
        .bind(&file.mime_type)
        .bind(file.file_size)
        .bind(&file.storage_path)
        .bind(file.uploaded_by)
        .fetch_one(tx)
        .await?;

        Ok(entity)
    }

    async fn find_by_id(&self, file_id: &Uuid) -> Result<Option<FileEntity>, error::SystemError> {
        let file = sqlx::query_as::<_, FileEntity>(
            r#"
            SELECT * FROM files WHERE id = $1
            "#,
        )
        .bind(file_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(file)
    }

    async fn delete<'e, E>(&self, file_id: &Uuid, tx: E) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            r#"
            DELETE FROM files WHERE id = $1
            "#,
        )
        .bind(file_id)
        .execute(tx)
        .await?;

        Ok(())
    }
}
