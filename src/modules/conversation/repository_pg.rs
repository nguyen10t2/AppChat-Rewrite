use sqlx::{FromRow, Row};
use uuid::Uuid;

use crate::modules::conversation::model::{
    ConversationDetail, ConversationRaw, ConversationRow, GroupInfo, LastMessageRow,
    NewLastMessage, NewParticipant, ParticipantDetailWithConversation, ParticipantRow,
};
use crate::modules::conversation::repository::{
    ConversationRepository, LastMessageRepository, ParticipantRepository,
};
use crate::modules::conversation::schema::{
    ConversationType, LastMessageEntity, PartacipantEntity,
};
use crate::{api::error, modules::conversation::schema::ConversationEntity};

#[derive(Clone)]
pub struct ConversationPgRepository {
    pool: sqlx::PgPool,
    participant_repo: ParticipantPgRepository,
}

impl ConversationPgRepository {
    pub fn new(pool: sqlx::PgPool, participant_repo: ParticipantPgRepository) -> Self {
        Self { pool, participant_repo }
    }
}

#[async_trait::async_trait]
impl ConversationRepository for ConversationPgRepository {
    fn get_pool(&self) -> &sqlx::Pool<sqlx::Postgres> {
        &self.pool
    }

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

    async fn create_direct_conversation<'e>(
        &self,
        user_a: &Uuid,
        user_b: &Uuid,
        tx: &mut sqlx::Transaction<'e, sqlx::Postgres>,
    ) -> Result<ConversationEntity, error::SystemError> {
        let conversation = self.create(&ConversationType::Direct, tx.as_mut()).await?;

        self.participant_repo
            .create_participant(
                &NewParticipant {
                    conversation_id: conversation.id,
                    user_id: *user_a,
                    unread_count: 0,
                },
                tx.as_mut(),
            )
            .await?;

        self.participant_repo
            .create_participant(
                &NewParticipant {
                    conversation_id: conversation.id,
                    user_id: *user_b,
                    unread_count: 0,
                },
                tx.as_mut(),
            )
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
            WHERE c.type = 'direct'
            AND EXISTS (
                SELECT 1
                FROM participants p1
                WHERE p1.conversation_id = c.id
                AND p1.user_id = $1
                AND p1.deleted_at IS NULL
            )
            AND EXISTS (
                SELECT 1
                FROM participants p2
                WHERE p2.conversation_id = c.id
                AND p2.user_id = $2
                AND p2.deleted_at IS NULL
            )
            LIMIT 1;
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

    async fn get_conversation_and_check_membership<'e, E>(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        tx: E,
    ) -> Result<(Option<ConversationEntity>, bool), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let row = sqlx::query(
            r#"
            SELECT c.*,
                EXISTS(
                SELECT 1
                FROM participants p
                WHERE p.conversation_id = c.id
                AND p.user_id = $2
                ) as is_member
            FROM conversations c
            WHERE c.id = $1
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_optional(tx)
        .await?;

        if let Some(row) = row {
            let is_member: bool = row.get("is_member");
            let conversation = ConversationEntity::from_row(&row)?;
            Ok((Some(conversation), is_member))
        } else {
            Ok((None, false))
        }
    }
}

#[derive(Clone, Default)]
pub struct ParticipantPgRepository {}

#[async_trait::async_trait]
impl ParticipantRepository for ParticipantPgRepository {
    async fn create_participant<'e, E>(
        &self,
        participant: &NewParticipant,
        tx: E,
    ) -> Result<PartacipantEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let entity = sqlx::query_as::<_, PartacipantEntity>(
            r#"
            INSERT INTO participants (conversation_id, user_id, unread_count)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(participant.conversation_id)
        .bind(participant.user_id)
        .bind(participant.unread_count)
        .fetch_one(tx)
        .await?;

        Ok(entity)
    }

    async fn increment_unread_count<'e, E>(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE participants
            SET unread_count = unread_count + 1
            WHERE conversation_id = $1
            AND user_id = $2
            AND deleted_at IS NULL
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .execute(tx)
        .await?;

        Ok(())
    }

    async fn reset_unread_count<'e, E>(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            r#"
            UPDATE participants
            SET unread_count = 0
            WHERE conversation_id = $1
            AND user_id = $2
            AND deleted_at IS NULL
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .execute(tx)
        .await?;

        Ok(())
    }

    async fn find_participants_by_conversation_id<'e, E>(
        &self,
        conversation_ids: &[Uuid],
        tx: E,
    ) -> Result<Vec<ParticipantDetailWithConversation>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let participants = sqlx::query_as::<_, ParticipantDetailWithConversation>(
            r#"
            SELECT
                p.conversation_id,
                p.user_id,
                u.display_name,
                u.avatar_url,
                p.unread_count,
                p.joined_at
            FROM participants p
            JOIN users u ON u.id = p.user_id
            WHERE p.conversation_id = ANY($1)
            AND p.deleted_at IS NULL
            "#,
        )
        .bind(conversation_ids)
        .fetch_all(tx)
        .await?;

        Ok(participants)
    }
}

#[allow(unused)]
#[derive(Clone, Default)]
pub struct LastMessagePgRepository {}

#[async_trait::async_trait]
impl LastMessageRepository for LastMessagePgRepository {
    async fn upsert_last_message<'e, E>(
        &self,
        last_message: &NewLastMessage,
        tx: E,
    ) -> Result<LastMessageEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let id = Uuid::now_v7();
        let res = sqlx::query_as::<_, LastMessageEntity>(
            r#"
            INSERT INTO last_messages (id, content, conversation_id, sender_id, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (conversation_id) DO UPDATE
            SET content = EXCLUDED.content,
                sender_id = EXCLUDED.sender_id,
                created_at = NOW()
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&last_message.content)
        .bind(last_message.conversation_id)
        .bind(last_message.sender_id)
        .bind(last_message.created_at)
        .fetch_one(tx)
        .await?;

        Ok(res)
    }
}
