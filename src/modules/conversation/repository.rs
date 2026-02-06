use uuid::Uuid;

use crate::{
    api::error,
    modules::conversation::{
        model::{ConversationDetail, ConversationRow},
        schema::{ConversationEntity, ConversationType},
    },
};

#[async_trait::async_trait]
pub trait ConversationRepository {
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
}
