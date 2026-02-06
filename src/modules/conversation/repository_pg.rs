use uuid::Uuid;

use crate::modules::conversation::model::{
    ConversationDetail, ConversationRaw, ConversationRow, GroupInfo, LastMessageRow, ParticipantRow,
};
use crate::modules::conversation::repository::ConversationRepository;
use crate::modules::conversation::schema::ConversationType;
use crate::{api::error, modules::conversation::schema::ConversationEntity};

#[derive(Clone)]
pub struct ConversationPgRepository {
    pool: sqlx::PgPool,
}

impl ConversationPgRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub fn get_pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

#[async_trait::async_trait]
impl ConversationRepository for ConversationPgRepository {
    async fn find_by_id<'e, E>(
        &self,
        conversation_id: &Uuid,
        tx: E,
    ) -> Result<Option<ConversationEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let conversation =
            sqlx::query_as::<_, ConversationEntity>("SELECT * FROM conversations WHERE id = $1")
                .bind(conversation_id)
                .fetch_optional(tx)
                .await?;

        Ok(conversation)
    }

    async fn find_one_conversation_detail(
        &self,
        conversation_id: &Uuid,
    ) -> Result<Option<ConversationDetail>, error::SystemError> {
        let conv = sqlx::query_as::<_, ConversationRaw>(
            r#"
            SELECT
                c.id,
                c.type,
                c.created_at,
                c.updated_at,

                g.name AS group_name,
                g.created_by AS group_created_by,
                g.avatar_url AS group_avatar_url,

                m.content AS last_content,
                m.sender_id AS last_sender_id,
                m.created_at AS last_created_at
            FROM conversations c
            LEFT JOIN group_conversations g
                ON g.conversation_id = c.id
            LEFT JOIN LATERAL (
                SELECT content, sender_id, created_at
                FROM messages
                WHERE conversation_id = c.id
                ORDER BY created_at DESC
                LIMIT 1
            ) m ON true
            WHERE c.id = $1
            LIMIT 1
            "#,
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await?;

        let raw = match conv {
            Some(v) => v,
            None => return Ok(None),
        };

        let participants = sqlx::query_as::<_, ParticipantRow>(
            r#"
            SELECT
                p.user_id,
                u.display_name,
                u.avatar_url,
                u.avatar_id,
                p.unread_count,
                p.joined_at
            FROM participants p
            JOIN users u ON u.id = p.user_id
            WHERE p.conversation_id = $1
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let res = ConversationDetail {
            conversation_id: raw.id,
            _type: raw._type,
            created_at: raw.created_at,
            updated_at: raw.updated_at,

            group_info: match (raw.group_name, raw.group_created_by) {
                (Some(name), Some(created_by)) => {
                    Some(GroupInfo { name, avatar_url: raw.group_avatar_url, created_by })
                }
                _ => None,
            },

            last_message: match (raw.last_content, raw.last_sender_id, raw.last_created_at) {
                (content, Some(sender_id), Some(created_at)) => {
                    Some(LastMessageRow { content, sender_id, created_at })
                }
                _ => None,
            },

            participants,
        };

        Ok(Some(res))
    }

    async fn create<'e, E>(
        &self,
        _type: &ConversationType,
        tx: E,
    ) -> Result<ConversationEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let id = Uuid::now_v7();
        let conversation = sqlx::query_as::<_, ConversationEntity>(
            r#"
            INSERT INTO conversations (id, type)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(_type)
        .fetch_one(tx)
        .await?;

        Ok(conversation)
    }

    async fn create_group_conversation<'e>(
        &self,
        name: &str,
        unique_member_ids: &[Uuid],
        user_id: &Uuid,
        tx: &mut sqlx::Transaction<'e, sqlx::Postgres>,
    ) -> Result<ConversationEntity, error::SystemError> {
        let conversation = self.create(&ConversationType::Group, tx.as_mut()).await?;

        sqlx::query(
            r#"
            INSERT INTO group_conversations (conversation_id, name, created_by)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(conversation.id)
        .bind(name)
        .bind(user_id)
        .execute(tx.as_mut())
        .await?;

        sqlx::query(
            r#"
            INSERT INTO participants (conversation_id, user_id, unread_count, joined_at)
            SELECT $1, unnest($2::uuid[]), 0, NOW()
            "#,
        )
        .bind(conversation.id)
        .bind(unique_member_ids)
        .execute(tx.as_mut())
        .await?;

        Ok(conversation)
    }

    async fn find_direct_between_users<'e, E>(
        &self,
        user_a: &Uuid,
        user_b: &Uuid,
        tx: E,
    ) -> Result<Option<ConversationEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let conversation = sqlx::query_as::<_, ConversationEntity>(
            r#"
            SELECT c.*
            FROM conversations c
            JOIN participants p
            ON p.conversation_id = c.id
            WHERE c.type = 'direct'
            AND p.user_id IN ($1, $2)
            AND p.deleted_at IS NULL
            GROUP BY c.id
            HAVING COUNT(DISTINCT p.user_id) = 2
            LIMIT 1
            "#,
        )
        .bind(user_a)
        .bind(user_b)
        .fetch_optional(tx)
        .await?;

        Ok(conversation)
    }

    async fn find_all_conversation_with_details_by_user<'e, E>(
        &self,
        user_id: &Uuid,
        tx: E,
    ) -> Result<Vec<ConversationRow>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let rows = sqlx::query_as::<_, ConversationRaw>(
            r#"
            SELECT
                c.id,
                c.type,
                c.created_at,
                c.updated_at,

                g.name          AS group_name,
                g.avatar_url    AS group_avatar_url,
                g.avatar_id     AS group_avatar_id,
                g.created_by    AS group_created_by,

                lm.content      AS last_content,
                lm.sender_id    AS last_sender_id,
                lm.created_at   AS last_created_at

            FROM conversations c

            JOIN participants p
                ON p.conversation_id = c.id
            AND p.user_id = $1
            AND p.deleted_at IS NULL

            LEFT JOIN group_conversations g
                ON g.conversation_id = c.id

            LEFT JOIN LATERAL (
                SELECT content, sender_id, created_at
                FROM messages m
                WHERE m.conversation_id = c.id
                ORDER BY created_at DESC
                LIMIT 1
            ) lm ON TRUE

            ORDER BY
                COALESCE(lm.created_at, c.updated_at) DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(tx)
        .await?;

        let result = rows
            .into_iter()
            .map(|r| {
                let group_info = match (r.group_name, r.group_created_by) {
                    (Some(name), Some(created_by)) => {
                        Some(GroupInfo { name, avatar_url: r.group_avatar_url, created_by })
                    }
                    _ => None,
                };

                let last_message = match (r.last_content, r.last_sender_id, r.last_created_at) {
                    (content, Some(sender_id), Some(created_at)) => {
                        Some(LastMessageRow { content, sender_id, created_at })
                    }
                    _ => None,
                };

                ConversationRow {
                    conversation_id: r.id,
                    _type: r._type,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                    group_info,
                    last_message,
                }
            })
            .collect();

        Ok(result)
    }
}
