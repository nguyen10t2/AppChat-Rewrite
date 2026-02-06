use actix_web::{HttpRequest, delete, get, post, web};
use uuid::Uuid;

use crate::{
    api::{error, success},
    middlewares::get_claims,
    modules::friend::{
        model::{FriendRequestBody, FriendRequestResponse, FriendResponse},
        schema::FriendRequestEntity,
        service::FriendService,
    },
};

#[post("/requests")]
pub async fn send_friend_request(
    friend_service: web::Data<FriendService>,
    body: web::Json<FriendRequestBody>,
    req: HttpRequest,
) -> Result<success::Success<FriendRequestEntity>, error::Error> {
    let sender_id = get_claims(&req)?.sub;
    let request = friend_service
        .send_friend_request(sender_id, body.recipient_id, body.message.clone())
        .await?;

    Ok(success::Success::created(Some(request)).message("Friend request sent successfully"))
}

#[post("/requests/{request_id}/accept")]
pub async fn accept_friend_request(
    friend_service: web::Data<FriendService>,
    request_id: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<success::Success<FriendResponse>, error::Error> {
    let receiver_id = get_claims(&req)?.sub;
    let response = friend_service.accept_friend_request(receiver_id, *request_id).await?;

    Ok(success::Success::ok(Some(response)).message("Friend request accepted successfully"))
}

#[post("/requests/{request_id}/decline")]
pub async fn decline_friend_request(
    friend_service: web::Data<FriendService>,
    request_id: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<success::Success<()>, error::Error> {
    let receiver_id = get_claims(&req)?.sub;
    friend_service.decline_friend_request(receiver_id, *request_id).await?;
    Ok(success::Success::no_content())
}

#[get("/")]
pub async fn list_friends(
    friend_service: web::Data<FriendService>,
    req: HttpRequest,
) -> Result<success::Success<Vec<FriendResponse>>, error::Error> {
    let user_id = get_claims(&req)?.sub;
    let friends = friend_service.get_friends(user_id).await?;

    Ok(success::Success::ok(Some(friends)).message("Friends retrieved successfully"))
}

#[get("/requests")]
pub async fn list_friend_requests(
    friend_service: web::Data<FriendService>,
    req: HttpRequest,
) -> Result<success::Success<Vec<FriendRequestResponse>>, error::Error> {
    let user_id = get_claims(&req)?.sub;
    let requests = friend_service.get_friend_requests(user_id).await?;

    Ok(success::Success::ok(Some(requests)).message("Friend requests retrieved successfully"))
}

#[delete("/{friend_id}")]
pub async fn remove_friend(
    friend_service: web::Data<FriendService>,
    friend_id: web::Path<Uuid>,
    req: HttpRequest,
) -> Result<success::Success<()>, error::Error> {
    let user_id = get_claims(&req)?.sub;
    friend_service.remove_friend(user_id, *friend_id).await?;
    Ok(success::Success::no_content())
}
