use uuid::Uuid;

use crate::{
    api::error,
    modules::friend::{
        model::{FriendRequestResponse, FriendResponse, FriendUserRow, IdOrInfo},
        repository::{FriendRepo, FriendRepository, FriendRequestRepository},
        schema::{FriendEntity, FriendRequestEntity},
    },
};

#[derive(Clone)]
pub struct FriendRepositoryPg {
    pool: sqlx::PgPool,
}

impl FriendRepositoryPg {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl FriendRepository for FriendRepositoryPg {
    async fn find_friendship<'e, E>(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
        tx: E,
    ) -> Result<Option<FriendEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let (user_a, user_b) =
            if user_id_a <= user_id_b { (user_id_a, user_id_b) } else { (user_id_b, user_id_a) };

        let friendship = sqlx::query_as::<_, FriendEntity>(
            "SELECT * FROM friends WHERE user_a = $1 AND user_b = $2",
        )
        .bind(user_a)
        .bind(user_b)
        .fetch_optional(tx)
        .await?;

        Ok(friendship)
    }

    async fn find_friends<'e, E>(
        &self,
        user_id: &Uuid,
        tx: E,
    ) -> Result<Vec<FriendResponse>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let friends = sqlx::query_as::<_, FriendResponse>(
            r#"
        SELECT
            u.id,
            u.username,
            u.display_name,
            u.avatar_url,
            u.avatar_id
        FROM friends f
        JOIN users u
            ON u.id = CASE
                WHEN f.user_a = $1 THEN f.user_b
                ELSE f.user_a
            END
        WHERE f.user_a = $1
           OR f.user_b = $1
        "#,
        )
        .bind(user_id)
        .fetch_all(tx)
        .await?;

        Ok(friends)
    }

    async fn create_friendship<'e, E>(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let (user_a, user_b) =
            if user_id_a <= user_id_b { (user_id_a, user_id_b) } else { (user_id_b, user_id_a) };

        sqlx::query("INSERT INTO friends (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(user_a)
            .bind(user_b)
            .execute(tx)
            .await?;

        Ok(())
    }

    async fn delete_friendship<'e, E>(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let (user_a, user_b) =
            if user_id_a <= user_id_b { (user_id_a, user_id_b) } else { (user_id_b, user_id_a) };

        sqlx::query("DELETE FROM friends WHERE user_a = $1 AND user_b = $2")
            .bind(user_a)
            .bind(user_b)
            .execute(tx)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FriendRequestRepository for FriendRepositoryPg {
    async fn find_friend_request<'e, E>(
        &self,
        sender_id: &Uuid,
        receiver_id: &Uuid,
        tx: E,
    ) -> Result<Option<FriendRequestEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let request = sqlx::query_as::<_, FriendRequestEntity>(
            r#"
            SELECT *
            FROM friend_requests
            WHERE
                (from_user_id = $1 AND to_user_id = $2)
            OR (from_user_id = $2 AND to_user_id = $1)
            "#,
        )
        .bind(sender_id)
        .bind(receiver_id)
        .fetch_optional(tx)
        .await?;

        Ok(request)
    }

    async fn find_friend_request_by_id<'e, E>(
        &self,
        request_id: &Uuid,
        tx: E,
    ) -> Result<Option<FriendRequestEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let request =
            sqlx::query_as::<_, FriendRequestEntity>("SELECT * FROM friend_requests WHERE id = $1")
                .bind(request_id)
                .fetch_optional(tx)
                .await?;

        Ok(request)
    }

    async fn find_friend_request_from_user<'e, E>(
        &self,
        user_id: &Uuid,
        tx: E,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let rows = sqlx::query_as::<_, FriendUserRow>(
            r#"
            SELECT
                fr.id AS req_id,
                u.id AS user_id,
                u.username,
                u.display_name,
                u.avatar_url,
                u.avatar_id,
                fr.message,
                fr.created_at
            FROM friend_requests fr
            JOIN users u
                ON fr.to_user_id = u.id
            WHERE fr.from_user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(tx)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| FriendRequestResponse {
                id: r.req_id,
                from: IdOrInfo::Id(*user_id),
                to: IdOrInfo::Info(FriendResponse {
                    id: r.user_id,
                    username: r.username,
                    display_name: r.display_name,
                    avatar_url: r.avatar_url,
                }),
                message: r.message,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn find_friend_request_to_user<'e, E>(
        &self,
        user_id: &Uuid,
        tx: E,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let rows = sqlx::query_as::<_, FriendUserRow>(
            r#"
            SELECT
                fr.id AS req_id,
                u.id AS user_id,
                u.username,
                u.display_name,
                u.avatar_url,
                u.avatar_id,
                fr.message,
                fr.created_at
            FROM friend_requests fr
            JOIN users u
                ON fr.from_user_id = u.id
            WHERE fr.to_user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(tx)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| FriendRequestResponse {
                id: r.req_id,
                from: IdOrInfo::Info(FriendResponse {
                    id: r.user_id,
                    username: r.username,
                    display_name: r.display_name,
                    avatar_url: r.avatar_url,
                }),
                to: IdOrInfo::Id(*user_id),
                message: r.message,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn create_friend_request<'e, E>(
        &self,
        sender_id: &Uuid,
        receiver_id: &Uuid,
        message: &Option<String>,
        tx: E,
    ) -> Result<FriendRequestEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let id = Uuid::now_v7();
        let request = sqlx::query_as::<_, FriendRequestEntity>(
            r#"
            INSERT INTO friend_requests (id, from_user_id, to_user_id, message)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(sender_id)
        .bind(receiver_id)
        .bind(message)
        .fetch_one(tx)
        .await?;

        Ok(request)
    }

    async fn delete_friend_request<'e, E>(
        &self,
        request_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query("DELETE FROM friend_requests WHERE id = $1")
            .bind(request_id)
            .execute(tx)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FriendRepo for FriendRepositoryPg {
    fn get_pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}
