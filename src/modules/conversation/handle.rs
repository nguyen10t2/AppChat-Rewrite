use actix_web::{get, post, web, HttpRequest};
use uuid::Uuid;

use crate::{
    api::{error, success},
    middlewares::get_claims,
    modules::{
        conversation::{
            model::{ConversationDetail, MessageQueryRequest, NewConversation},
            repository_pg::{ConversationPgRepository, ParticipantPgRepository},
            service::ConversationService,
        },
        message::{model::GetMessageResponse, repository_pg::MessageRepositoryPg},
    },
};

type ConversationSvc =
    ConversationService<ConversationPgRepository, ParticipantPgRepository, MessageRepositoryPg>;

#[get("/")]
pub async fn get_conversations(
    conversation_svc: web::Data<ConversationSvc>,
    req: HttpRequest,
) -> Result<success::Success<Vec<ConversationDetail>>, error::Error> {
    let user_id = get_claims(&req)?.sub;

    let conversations = conversation_svc.get_by_user_id(user_id).await?;

    Ok(success::Success::ok(Some(conversations)).message("Successfully retrieved conversations"))
}

#[get("/{conversation_id}/messages")]
pub async fn get_messages(
    conversation_svc: web::Data<ConversationSvc>,
    conversation_id: web::Path<Uuid>,
    query: web::Query<MessageQueryRequest>,
) -> Result<success::Success<GetMessageResponse>, error::Error> {
    let (messages, cursor) =
        conversation_svc.get_message(*conversation_id, query.limit, query.cursor.clone()).await?;
    Ok(success::Success::ok(Some(GetMessageResponse { messages, cursor }))
        .message("Successfully retrieved messages"))
}

#[post("/")]
pub async fn create_conversation(
    conversation_svc: web::Data<ConversationSvc>,
    body: web::Json<NewConversation>,
    req: HttpRequest,
) -> Result<success::Success<Option<ConversationDetail>>, error::Error> {
    let user_id = get_claims(&req)?.sub;

    let body = body.into_inner();

    let conversation = conversation_svc
        .create_conversation(body._type, body.name, body.member_ids, user_id)
        .await?;

    Ok(success::Success::ok(Some(conversation)).message("Successfully created conversation"))
}
