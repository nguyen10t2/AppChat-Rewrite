use uuid::Uuid;

use crate::{
    api::error,
    modules::user::{
        model::{InsertUser, UpdateUser},
        repository::UserRepository,
        schema::UserEntity,
    },
};

#[derive(Clone)]
pub struct UserRepositoryPg {
    pool: sqlx::PgPool,
}

impl UserRepositoryPg {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for UserRepositoryPg {
    async fn find_by_id(&self, id: &Uuid) -> Result<Option<UserEntity>, error::SystemError> {
        let user = sqlx::query_as::<_, UserEntity>(
            "SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn find_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserEntity>, error::SystemError> {
        let user = sqlx::query_as::<_, UserEntity>(
            "SELECT * FROM users WHERE lower(username) = lower($1) AND deleted_at IS NULL",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn create(&self, user: &InsertUser) -> Result<Uuid, error::SystemError> {
        let id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        sqlx::query(
            "INSERT INTO users (id, username, email, hash_password, display_name) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.hash_password)
        .bind(&user.display_name)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn update(&self, id: &Uuid, user: &UpdateUser) -> Result<UserEntity, error::SystemError> {
        let user = sqlx::query_as::<_, UserEntity>(
            r#"
        UPDATE users
        SET
            username     = COALESCE($2, username),
            email        = COALESCE($3, email),
            display_name = COALESCE($4, display_name),
            avatar_url   = $5,
            bio          = $6,
            phone        = $7
        WHERE id = $1
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(&user.username) // Option<String>
        .bind(&user.email) // Option<String>
        .bind(&user.display_name) // Option<String>
        .bind(&user.avatar_url) // Option<Option<String>>
        .bind(&user.bio) // Option<Option<String>>
        .bind(&user.phone) // Option<Option<String>>
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| error::SystemError::not_found("User not found"))?;

        Ok(user)
    }

    async fn delete(&self, id: &Uuid) -> Result<bool, error::SystemError> {
        let rows =
            sqlx::query("UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
                .bind(id)
                .execute(&self.pool)
                .await?
                .rows_affected();

        Ok(rows > 0)
    }
}
