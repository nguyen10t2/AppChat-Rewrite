use crate::modules::message::model::{InsertMessage, MessageQuery};
use crate::{api::error, modules::message::schema::MessageEntity};

#[async_trait::async_trait]
pub trait MessageRepository {
    async fn create(
        &self,
        message: &InsertMessage,
    ) -> Result<Option<MessageEntity>, error::SystemError>;

    async fn find_by_query(
        &self,
        query: &MessageQuery,
        limit: usize,
    ) -> Result<Vec<MessageEntity>, error::SystemError>;
}
