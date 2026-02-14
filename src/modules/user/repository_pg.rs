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
            avatar_url   = CASE WHEN $5::boolean THEN $6 ELSE avatar_url END,
            bio          = CASE WHEN $7::boolean THEN $8 ELSE bio END,
            phone        = CASE WHEN $9::boolean THEN $10 ELSE phone END
        WHERE id = $1
        RETURNING *
        "#,
        )
        .bind(id)
        .bind(&user.username) // $2: Option<String>
        .bind(&user.email) // $3: Option<String>
        .bind(&user.display_name) // $4: Option<String>
        .bind(user.avatar_url.is_some()) // $5: bool - was avatar_url provided?
        .bind(user.avatar_url.as_ref().and_then(|v| v.as_ref())) // $6: Option<&String>
        .bind(user.bio.is_some()) // $7: bool - was bio provided?
        .bind(user.bio.as_ref().and_then(|v| v.as_ref())) // $8: Option<&String>
        .bind(user.phone.is_some()) // $9: bool - was phone provided?
        .bind(user.phone.as_ref().and_then(|v| v.as_ref())) // $10: Option<&String>
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

    async fn search_users(
        &self,
        query: &str,
        limit: i32,
    ) -> Result<Vec<UserEntity>, error::SystemError> {
        let search_pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
        let users = sqlx::query_as::<_, UserEntity>(
            r#"
            SELECT * FROM users
            WHERE deleted_at IS NULL
            AND (
                lower(username) LIKE lower($1)
                OR lower(display_name) LIKE lower($1)
            )
            ORDER BY display_name
            LIMIT $2
            "#,
        )
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }
}
