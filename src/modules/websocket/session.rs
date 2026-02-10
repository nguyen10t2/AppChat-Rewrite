/// WebSocket Session Actor
///
/// Mỗi WebSocket connection có một Session actor riêng.
/// Session actor quản lý state (auth, user_id) và gửi messages tới client
/// thông qua mpsc channel được bridge từ handler.rs.
///
/// Async operations (DB calls) sử dụng `ctx.spawn()` + `into_actor()`.
use actix::prelude::*;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::modules::conversation::repository_pg::{
    ConversationPgRepository, LastMessagePgRepository, ParticipantPgRepository,
};
use crate::modules::message::repository_pg::MessageRepositoryPg;
use crate::modules::message::service::MessageService;
use crate::utils::{Claims, TypeClaims};
use crate::ENV;

use super::events::*;
use super::message::{ClientMessage, ServerMessage};
use super::server::WebSocketServer;

/// Type alias cho MessageService với concrete repository types
pub type MessageSvc = MessageService<
    MessageRepositoryPg,
    ConversationPgRepository,
    ParticipantPgRepository,
    LastMessagePgRepository,
>;

/// WebSocket session cho một client
pub struct WebSocketSession {
    /// Unique session ID
    pub id: Uuid,

    /// User ID sau khi authenticate (None nếu chưa auth)
    pub user_id: Option<Uuid>,

    /// Address của WebSocket server actor
    pub server: Addr<WebSocketServer>,

    /// Channel gửi JSON messages tới client (bridge → handler.rs → WebSocket)
    pub tx: mpsc::UnboundedSender<String>,

    /// Message service để persist messages vào DB (None trong test environment)
    pub message_service: Option<actix_web::web::Data<MessageSvc>>,
}

impl WebSocketSession {
    /// Tạo session mới với outbound channel và message service
    pub fn new(
        server: Addr<WebSocketServer>,
        tx: mpsc::UnboundedSender<String>,
        message_service: actix_web::web::Data<MessageSvc>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            user_id: None,
            server,
            tx,
            message_service: Some(message_service),
        }
    }

    /// Gửi ServerMessage tới client thông qua channel
    fn send_to_client(&self, msg: &ServerMessage) {
        match serde_json::to_string(msg) {
            Ok(json) => {
                if let Err(e) = self.tx.send(json) {
                    tracing::error!(
                        "Không thể gửi message tới client (session {}): {}",
                        self.id,
                        e
                    );
                }
            }
            Err(e) => {
                tracing::error!("Không thể serialize ServerMessage (session {}): {}", self.id, e);
            }
        }
    }

    /// Gửi error message tới client
    fn send_error(&self, message: &str) {
        self.send_to_client(&ServerMessage::Error { message: message.to_string() });
    }

    /// Kiểm tra user đã authenticate chưa, trả về user_id nếu có
    fn require_auth(&self) -> Option<Uuid> {
        if self.user_id.is_none() {
            self.send_error("Bạn cần xác thực trước khi thực hiện thao tác này");
            tracing::warn!("Session {} chưa authenticate, từ chối request", self.id);
        }
        self.user_id
    }

    /// Xử lý message từ client - dispatch tới handler tương ứng
    fn handle_client_message(&mut self, msg: &ClientMessage, ctx: &mut Context<Self>) {
        match msg {
            ClientMessage::Auth { token } => {
                self.handle_auth(token);
            }

            ClientMessage::SendMessage { conversation_id, content } => {
                self.handle_send_message(*conversation_id, content.clone(), ctx);
            }

            ClientMessage::JoinConversation { conversation_id } => {
                self.handle_join_conversation(*conversation_id);
            }

            ClientMessage::LeaveConversation { conversation_id } => {
                self.handle_leave_conversation(*conversation_id);
            }

            ClientMessage::TypingStart { conversation_id } => {
                self.handle_typing_start(*conversation_id);
            }

            ClientMessage::TypingStop { conversation_id } => {
                self.handle_typing_stop(*conversation_id);
            }

            ClientMessage::Ping => {
                // Gửi pong response về client
                self.send_to_client(&ServerMessage::Pong);
            }
        }
    }

    /// Xử lý authentication - verify JWT token và liên kết user với session
    fn handle_auth(&mut self, token: &str) {
        // Kiểm tra đã auth chưa (tránh auth lại)
        if self.user_id.is_some() {
            self.send_error("Session đã được xác thực");
            return;
        }

        // Decode và verify JWT token
        let claims = match Claims::decode(token, ENV.jwt_secret.as_ref()) {
            Ok(claims) => claims,
            Err(e) => {
                tracing::warn!("JWT verification thất bại (session {}): {}", self.id, e);
                self.send_to_client(&ServerMessage::AuthFailed {
                    reason: "Token không hợp lệ hoặc đã hết hạn".to_string(),
                });
                return;
            }
        };

        // Kiểm tra token type phải là AccessToken
        if claims._type.as_ref() != Some(&TypeClaims::AccessToken) {
            self.send_to_client(&ServerMessage::AuthFailed {
                reason: "Chỉ chấp nhận access token".to_string(),
            });
            return;
        }

        let user_id = claims.sub;

        // Cập nhật state session
        self.user_id = Some(user_id);

        // Thông báo server về user đã authenticate
        self.server.do_send(Authenticate { session_id: self.id, user_id });

        // Gửi success response về client
        self.send_to_client(&ServerMessage::AuthSuccess { user_id });

        tracing::info!("User {} đã authenticate thành công trên session {}", user_id, self.id);
    }

    /// Xử lý gửi tin nhắn - lưu vào DB rồi broadcast tới room
    fn handle_send_message(&self, conversation_id: Uuid, content: String, ctx: &mut Context<Self>) {
        let Some(user_id) = self.require_auth() else {
            return;
        };

        tracing::debug!("User {} gửi message tới conversation {}", user_id, conversation_id);

        // Kiểm tra message service khả dụng
        let Some(service) = self.message_service.clone() else {
            self.send_error("Message service không khả dụng");
            return;
        };

        // Clone các dependencies cần thiết cho async block
        let server = self.server.clone();
        let tx = self.tx.clone();
        let session_id = self.id;

        // Spawn async future trong actor context để gọi DB
        ctx.spawn(
            async move {
                // Lưu message vào database
                match service.send_group_message(user_id, content, conversation_id).await {
                    Ok(msg_entity) => {
                        // Serialize MessageEntity thành JSON value cho broadcast
                        let message_value = serde_json::to_value(&msg_entity).unwrap_or_default();

                        // Broadcast tin nhắn mới tới tất cả users trong room
                        server.do_send(BroadcastToRoom {
                            conversation_id,
                            message: ServerMessage::NewMessage {
                                conversation_id,
                                message: message_value,
                            },
                            skip_user_id: None, // Gửi cả cho sender (confirm message đã gửi)
                        });

                        tracing::info!(
                            "Message {} saved và broadcast tới conversation {}",
                            msg_entity.id,
                            conversation_id
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Lỗi lưu message (session {}, conversation {}): {}",
                            session_id,
                            conversation_id,
                            e
                        );

                        // Gửi error response về client
                        let err_msg = ServerMessage::Error {
                            message: "Không thể gửi tin nhắn. Vui lòng thử lại.".to_string(),
                        };
                        if let Ok(json) = serde_json::to_string(&err_msg) {
                            let _ = tx.send(json);
                        }
                    }
                }
            }
            .into_actor(self),
        );
    }

    /// Xử lý join conversation room
    fn handle_join_conversation(&self, conversation_id: Uuid) {
        let Some(user_id) = self.require_auth() else {
            return;
        };

        self.server.do_send(JoinRoom { user_id, conversation_id });
        tracing::debug!("User {} joined conversation {}", user_id, conversation_id);
    }

    /// Xử lý leave conversation room
    fn handle_leave_conversation(&self, conversation_id: Uuid) {
        let Some(user_id) = self.require_auth() else {
            return;
        };

        self.server.do_send(LeaveRoom { user_id, conversation_id });
        tracing::debug!("User {} left conversation {}", user_id, conversation_id);
    }

    /// Xử lý typing start - broadcast tới room (trừ sender)
    fn handle_typing_start(&self, conversation_id: Uuid) {
        let Some(user_id) = self.require_auth() else {
            return;
        };

        self.server.do_send(BroadcastToRoom {
            conversation_id,
            message: ServerMessage::UserTyping { conversation_id, user_id },
            skip_user_id: Some(user_id),
        });
    }

    /// Xử lý typing stop - broadcast tới room (trừ sender)
    fn handle_typing_stop(&self, conversation_id: Uuid) {
        let Some(user_id) = self.require_auth() else {
            return;
        };

        self.server.do_send(BroadcastToRoom {
            conversation_id,
            message: ServerMessage::UserStoppedTyping { conversation_id, user_id },
            skip_user_id: Some(user_id),
        });
    }
}

impl Actor for WebSocketSession {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::debug!("WebSocket session started: {}", self.id);

        // Notify server về connection mới
        self.server.do_send(Connect { id: self.id, addr: ctx.address() });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::debug!("WebSocket session stopped: {}", self.id);

        // Notify server về disconnect
        self.server.do_send(Disconnect { id: self.id });
    }
}

/// Implement Message trait cho ClientMessage để có thể send qua actors
impl Message for ClientMessage {
    type Result = ();
}

/// Handler: Nhận ClientMessage từ handler.rs
impl Handler<ClientMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, ctx: &mut Context<Self>) {
        self.handle_client_message(&msg, ctx);
    }
}

/// Handler: Nhận ServerMessage từ server actor → serialize → gửi tới client qua channel
impl Handler<ServerMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: ServerMessage, _ctx: &mut Context<Self>) {
        self.send_to_client(&msg);
    }
}
