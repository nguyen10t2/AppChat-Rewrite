use uuid::Uuid;

use crate::{
    api::error,
    modules::conversation::{
        model::{
            ConversationDetail, ConversationRow, NewLastMessage, NewParticipant,
            ParticipantDetailWithConversation,
        },
        schema::{ConversationEntity, ConversationType, LastMessageEntity, ParticipantEntity},
    },
};

#[async_trait::async_trait]
pub trait ConversationRepository {
    fn get_pool(&self) -> &sqlx::Pool<sqlx::Postgres>;

    async fn find_by_id<'e, E>(
        &self,
        conversation_id: &Uuid,
        tx: E,
    ) -> Result<Option<ConversationEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_one_conversation_detail(
        &self,
        conversation_id: &Uuid,
    ) -> Result<Option<ConversationDetail>, error::SystemError>;

    async fn create<'e, E>(
        &self,
        _type: &ConversationType,
        tx: E,
    ) -> Result<ConversationEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn create_direct_conversation<'e>(
        &self,
        user_a: &Uuid,
        user_b: &Uuid,
        tx: &mut sqlx::Transaction<'e, sqlx::Postgres>,
    ) -> Result<ConversationEntity, error::SystemError>;

    async fn create_group_conversation<'e>(
        &self,
        name: &str,
        unique_member_ids: &[Uuid],
        user_id: &Uuid,
        tx: &mut sqlx::Transaction<'e, sqlx::Postgres>,
    ) -> Result<ConversationEntity, error::SystemError>;

    async fn find_direct_between_users<'e, E>(
        &self,
        user_a: &Uuid,
        user_b: &Uuid,
        tx: E,
    ) -> Result<Option<ConversationEntity>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_all_conversation_with_details_by_user<'e, E>(
        &self,
        user_id: &Uuid,
        tx: E,
    ) -> Result<Vec<ConversationRow>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn get_conversation_and_check_membership<'e, E>(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        tx: E,
    ) -> Result<(Option<ConversationEntity>, bool), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    /// Update conversation's updated_at timestamp to current time
    async fn update_timestamp<'e, E>(
        &self,
        conversation_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;
}

#[async_trait::async_trait]
pub trait ParticipantRepository {
    async fn create_participant<'e, E>(
        &self,
        participant: &NewParticipant,
        tx: E,
    ) -> Result<ParticipantEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn increment_unread_count<'e, E>(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    /// Increment unread count for all participants in a conversation except the sender
    async fn increment_unread_count_for_others<'e, E>(
        &self,
        conversation_id: &Uuid,
        sender_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    #[allow(unused)]
    async fn reset_unread_count<'e, E>(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    /// Mark messages as seen by updating last_seen_message_id and resetting unread count
    async fn mark_as_seen<'e, E>(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
        last_seen_message_id: &Uuid,
        tx: E,
    ) -> Result<(), error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    async fn find_participants_by_conversation_id<'e, E>(
        &self,
        conversation_ids: &[Uuid],
        tx: E,
    ) -> Result<Vec<ParticipantDetailWithConversation>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;

    /// Get unread counts for all participants in a conversation
    /// Returns a map of user_id -> unread_count
    async fn get_unread_counts<'e, E>(
        &self,
        conversation_id: &Uuid,
        tx: E,
    ) -> Result<std::collections::HashMap<Uuid, i32>, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;
}

#[async_trait::async_trait]
pub trait LastMessageRepository {
    async fn upsert_last_message<'e, E>(
        &self,
        last_message: &NewLastMessage,
        tx: E,
    ) -> Result<LastMessageEntity, error::SystemError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>;
}
