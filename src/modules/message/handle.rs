use actix_web::{post, web, HttpRequest};

use crate::{
    api::{error, success},
    middlewares::{get_claims, get_conversation},
    modules::{
        conversation::repository_pg::{
            ConversationPgRepository, LastMessagePgRepository, ParticipantPgRepository,
        },
        message::{
            model::{SendDirectMessage, SendGroupMessage},
            repository_pg::MessageRepositoryPg,
            schema::MessageEntity,
            service::MessageService,
        },
    },
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
    let user_id = get_claims(&req)?.sub;
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
    let user_id = get_claims(&req)?.sub;
    let conversation = get_conversation(&req)?;
    let message =
        message_service.send_group_message(user_id, body.content.clone(), conversation.id).await?;

    Ok(success::Success::ok(Some(message)).message("Send group message successfully"))
}
