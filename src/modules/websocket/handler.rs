/// WebSocket HTTP Handler
///
/// Module này xử lý HTTP upgrade request và quản lý bidirectional message flow:
/// - Inbound:  Client → WebSocket → parse ClientMessage → Session Actor
/// - Outbound: Server Actor → Session Actor → mpsc channel → WebSocket → Client
use actix::Addr;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use tokio::sync::mpsc;

use super::message::ClientMessage;
use super::presence::PresenceService;
use super::server::WebSocketServer;
use super::session::{MessageSvc, WebSocketSession};
use crate::modules::friend::repository_pg::FriendRepositoryPg;

/// HTTP handler để upgrade connection thành WebSocket
///
/// Endpoint: GET /ws
///
/// Flow:
/// 1. HTTP handshake → WebSocket connection
/// 2. Tạo mpsc channel (session actor → client)
/// 3. Start WebSocketSession actor
/// 4. Spawn async task xử lý bidirectional messages
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    server: web::Data<Addr<WebSocketServer>>,
    message_service: web::Data<MessageSvc>,
    presence_service: web::Data<PresenceService>,
    friend_repo: web::Data<FriendRepositoryPg>,
) -> Result<HttpResponse, Error> {
    tracing::debug!("WebSocket upgrade request từ {:?}", req.peer_addr());

    // Thực hiện WebSocket handshake
    let (response, mut ws_session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    // Tạo mpsc channel: session actor gửi JSON → spawned task → WebSocket → client
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Tạo session actor với outbound channel và dependencies
    let ws_actor = WebSocketSession::new(
        server.get_ref().clone(),
        tx,
        message_service,
        presence_service,
        friend_repo,
    );

    use actix::Actor;
    let addr = ws_actor.start();

    // Spawn async task xử lý bidirectional message flow
    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
                // === INBOUND: Client → Server ===
                msg = msg_stream.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            let text_str = text.to_string();

                            // Parse và forward tới session actor
                            match serde_json::from_str::<ClientMessage>(&text_str) {
                                Ok(client_msg) => {
                                    addr.do_send(client_msg);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Không thể parse client message: {} - raw: {}",
                                        e,
                                        &text_str[..100.min(text_str.len())]
                                    );
                                }
                            }
                        }

                        Some(Ok(Message::Ping(data))) => {
                            // Tự động trả lời pong cho WebSocket-level ping
                            if let Err(e) = ws_session.pong(&data).await {
                                tracing::error!("Không thể gửi pong: {}", e);
                                break;
                            }
                        }

                        Some(Ok(Message::Pong(_))) => {
                            // Heartbeat response - bỏ qua
                        }

                        Some(Ok(Message::Close(reason))) => {
                            tracing::info!("WebSocket close frame: {:?}", reason);
                            break;
                        }

                        Some(Ok(Message::Binary(_))) => {
                            tracing::warn!("Binary messages không được hỗ trợ");
                        }

                        Some(Ok(Message::Continuation(_) | Message::Nop)) => {}

                        Some(Err(e)) => {
                            tracing::error!("WebSocket protocol error: {}", e);
                            break;
                        }

                        // Stream kết thúc (client disconnect)
                        None => break,
                    }
                }

                // === OUTBOUND: Server → Client ===
                Some(json) = rx.recv() => {
                    if ws_session.text(json).await.is_err() {
                        tracing::error!("Không thể gửi message tới WebSocket client");
                        break;
                    }
                }
            }
        }

        // Cleanup: đóng WebSocket session
        let _ = ws_session.close(None).await;
        tracing::debug!("WebSocket message loop kết thúc");
    });

    tracing::info!("WebSocket connection established");
    Ok(response)
}
