use actix_web::{delete, patch, post, web, HttpRequest};
use uuid::Uuid;

use crate::{
    api::{error, success},
    middlewares::get_extensions,
    modules::{
        conversation::{
            repository_pg::{
                ConversationPgRepository, LastMessagePgRepository, ParticipantPgRepository,
            },
            schema::ConversationEntity,
        },
        message::{
            model::{EditMessageRequest, SendDirectMessage, SendGroupMessage},
            repository_pg::MessageRepositoryPg,
            schema::MessageEntity,
            service::MessageService,
        },
    },
    utils::{Claims, ValidatedJson},
};

type MessageSvc = MessageService<
    MessageRepositoryPg,
    ConversationPgRepository,
    ParticipantPgRepository,
    LastMessagePgRepository,
>;

#[post("/")]
pub async fn send_direct_message(
    message_service: web::Data<MessageSvc>,
    body: web::Json<SendDirectMessage>,
    req: HttpRequest,
) -> Result<success::Success<MessageEntity>, error::Error> {
    let user_id = get_extensions::<Claims>(&req)?.sub;
    let message = message_service
        .send_direct_message(
            user_id,
            body.recipient_id.ok_or(error::Error::bad_request("Recipient ID is required"))?,
            body.content.clone(),
            body.conversation_id,
        )
        .await?;

    Ok(success::Success::ok(Some(message)).message("Send direct message successfully"))
}

#[post("/")]
pub async fn send_group_message(
    message_service: web::Data<MessageSvc>,
    body: web::Json<SendGroupMessage>,
    req: HttpRequest,
) -> Result<success::Success<MessageEntity>, error::Error> {
    let user_id = get_extensions::<Claims>(&req)?.sub;
    let conversation = get_extensions::<ConversationEntity>(&req)?;
    let message =
        message_service.send_group_message(user_id, body.content.clone(), conversation.id).await?;

    Ok(success::Success::ok(Some(message)).message("Send group message successfully"))
}

#[delete("/{message_id}")]
pub async fn delete_message(
    message_service: web::Data<MessageSvc>,
    message_id: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<success::Success<()>, error::Error> {
    let user_id = get_extensions::<Claims>(&req)?.sub;
    message_service.delete_message(*message_id, user_id).await?;
    Ok(success::Success::no_content())
}

#[patch("/{message_id}")]
pub async fn edit_message(
    message_service: web::Data<MessageSvc>,
    message_id: web::Path<Uuid>,
    ValidatedJson(body): ValidatedJson<EditMessageRequest>,
    req: HttpRequest,
) -> Result<success::Success<MessageEntity>, error::Error> {
    let user_id = get_extensions::<Claims>(&req)?.sub;

    let message = message_service.edit_message(*message_id, user_id, body.content).await?;
    Ok(success::Success::ok(Some(message)).message("Message edited successfully"))
}
