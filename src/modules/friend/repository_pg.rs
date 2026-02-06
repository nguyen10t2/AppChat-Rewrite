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
    async fn find_friendship(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
    ) -> Result<Option<FriendEntity>, error::SystemError> {
        let (user_a, user_b) =
            if user_id_a <= user_id_b { (user_id_a, user_id_b) } else { (user_id_b, user_id_a) };

        let friendship = sqlx::query_as::<_, FriendEntity>(
            "SELECT * FROM friends WHERE user_a = $1 AND user_b = $2",
        )
        .bind(user_a)
        .bind(user_b)
        .fetch_optional(&self.pool)
        .await?;

        Ok(friendship)
    }

    async fn find_friends(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<FriendResponse>, error::SystemError> {
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
        .fetch_all(&self.pool)
        .await?;

        Ok(friends)
    }

    async fn create_friendship(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
    ) -> Result<(), error::SystemError> {
        let (user_a, user_b) =
            if user_id_a <= user_id_b { (user_id_a, user_id_b) } else { (user_id_b, user_id_a) };

        sqlx::query(
            "INSERT INTO friends (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(user_a)
        .bind(user_b)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_friendship(
        &self,
        user_id_a: &Uuid,
        user_id_b: &Uuid,
    ) -> Result<(), error::SystemError> {
        let (user_a, user_b) =
            if user_id_a <= user_id_b { (user_id_a, user_id_b) } else { (user_id_b, user_id_a) };

        sqlx::query("DELETE FROM friends WHERE user_a = $1 AND user_b = $2")
            .bind(user_a)
            .bind(user_b)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl FriendRequestRepository for FriendRepositoryPg {
    async fn find_friend_request(
        &self,
        sender_id: &Uuid,
        receiver_id: &Uuid,
    ) -> Result<Option<FriendRequestEntity>, error::SystemError> {
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
        .fetch_optional(&self.pool)
        .await?;

        Ok(request)
    }

    async fn find_friend_request_by_id(
        &self,
        request_id: &Uuid,
    ) -> Result<Option<FriendRequestEntity>, error::SystemError> {
        let request =
            sqlx::query_as::<_, FriendRequestEntity>("SELECT * FROM friend_requests WHERE id = $1")
                .bind(request_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(request)
    }

    async fn find_friend_request_from_user(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError> {
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
        .fetch_all(&self.pool)
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

    async fn find_friend_request_to_user(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<FriendRequestResponse>, error::SystemError> {
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
        .fetch_all(&self.pool)
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

    async fn create_friend_request(
        &self,
        sender_id: &Uuid,
        receiver_id: &Uuid,
        message: &Option<String>,
    ) -> Result<FriendRequestEntity, error::SystemError> {
        let request = sqlx::query_as::<_, FriendRequestEntity>(
            r#"
            INSERT INTO friend_requests (from_user_id, to_user_id, message)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(sender_id)
        .bind(receiver_id)
        .bind(message)
        .fetch_one(&self.pool)
        .await?;

        Ok(request)
    }

    async fn delete_friend_request(&self, request_id: &Uuid) -> Result<(), error::SystemError> {
        {
            sqlx::query("DELETE FROM friend_requests WHERE id = $1")
                .bind(request_id)
                .execute(&self.pool)
                .await?;

            Ok(())
        }
    }
}

#[async_trait::async_trait]
impl FriendRepo for FriendRepositoryPg {
    async fn accept_friend_request_atomic(
        &self,
        request_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<Uuid, error::SystemError> {
        let mut tx = self.pool.begin().await?;

        let request = sqlx::query_as::<_, FriendRequestEntity>(
            "SELECT * FROM friend_requests WHERE id = $1 FOR UPDATE",
        )
        .bind(request_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| error::SystemError::not_found("Friend request not found"))?;

        if request.to_user_id != *user_id {
            tx.rollback().await?;
            return Err(error::SystemError::forbidden(
                "You are not allowed to accept this friend request",
            ));
        }

        let (u1, u2) = if request.from_user_id <= request.to_user_id {
            (request.from_user_id, request.to_user_id)
        } else {
            (request.to_user_id, request.from_user_id)
        };

        sqlx::query(
            "INSERT INTO friends (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(u1)
        .bind(u2)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM friend_requests WHERE id = $1")
            .bind(request_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(request.from_user_id)
    }
}
