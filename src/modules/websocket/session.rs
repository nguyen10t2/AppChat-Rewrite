/// WebSocket Session Actor
///
/// Mỗi WebSocket connection có một Session actor riêng.
/// Session actor quản lý state (auth, user_id) và gửi messages tới client
/// thông qua mpsc channel được bridge từ handler.rs.
///
/// Presence tracking:
/// - Khi auth thành công: load friend list, set Redis presence, notify friends
/// - Heartbeat: refresh Redis TTL mỗi 15s
/// - Khi disconnect: set Redis offline + last_seen, notify friends
///
/// Async operations (DB calls) sử dụng `ctx.spawn()` + `into_actor()`.
use actix::prelude::*;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::modules::conversation::repository_pg::{
    ConversationPgRepository, LastMessagePgRepository, ParticipantPgRepository,
};
use crate::modules::friend::repository_pg::FriendRepositoryPg;
use crate::modules::message::repository_pg::MessageRepositoryPg;
use crate::modules::message::service::MessageService;
use crate::utils::{Claims, TypeClaims};
use crate::ENV;

use super::events::*;
use super::message::{ClientMessage, LastMessageInfo, SenderInfo, ServerMessage};
use super::presence::PresenceService;
use super::server::WebSocketServer;

/// Type alias cho MessageService với concrete repository types
pub type MessageSvc = MessageService<
    MessageRepositoryPg,
    ConversationPgRepository,
    ParticipantPgRepository,
    LastMessagePgRepository,
>;

/// Heartbeat ping interval (server gửi ping mỗi 15s)
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
/// Client timeout - nếu không nhận được pong sau 30s, disconnect
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

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

    /// Presence service cho Redis presence tracking
    pub presence_service: Option<actix_web::web::Data<PresenceService>>,

    /// Friend repository cho loading friend IDs
    pub friend_repo: Option<actix_web::web::Data<FriendRepositoryPg>>,

    /// Cached friend IDs - loaded sau khi auth, dùng cho presence notifications
    pub friend_ids: Vec<Uuid>,

    /// Thời điểm nhận heartbeat cuối cùng từ client
    pub last_heartbeat: Instant,
}

impl WebSocketSession {
    /// Tạo session mới với outbound channel và dependencies
    pub fn new(
        server: Addr<WebSocketServer>,
        tx: mpsc::UnboundedSender<String>,
        message_service: actix_web::web::Data<MessageSvc>,
        presence_service: actix_web::web::Data<PresenceService>,
        friend_repo: actix_web::web::Data<FriendRepositoryPg>,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            user_id: None,
            server,
            tx,
            message_service: Some(message_service),
            presence_service: Some(presence_service),
            friend_repo: Some(friend_repo),
            friend_ids: Vec::new(),
            last_heartbeat: Instant::now(),
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
                self.handle_auth(token, ctx);
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
                // Cập nhật heartbeat timestamp và gửi pong response
                self.last_heartbeat = Instant::now();
                self.send_to_client(&ServerMessage::Pong);
            }
        }
    }

    /// Xử lý authentication - verify JWT, load friends, set presence
    ///
    /// Flow (inspired by Messenger/Instagram):
    /// 1. Verify JWT token
    /// 2. Register session với server (sync)
    /// 3. Spawn async task:
    ///    a. Load friend IDs từ DB (for targeted notifications)
    ///    b. Set presence key trong Redis với TTL
    ///    c. Thông báo online friends về user mới online
    ///    d. Gửi initial online friends list cho user
    fn handle_auth(&mut self, token: &str, ctx: &mut Context<Self>) {
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

        // Thông báo server về user đã authenticate (đăng ký vào users map)
        self.server.do_send(Authenticate { session_id: self.id, user_id });

        // Gửi success response về client
        self.send_to_client(&ServerMessage::AuthSuccess { user_id });

        tracing::info!("User {} đã authenticate thành công trên session {}", user_id, self.id);

        // === Presence flow (async) ===
        let friend_repo = self.friend_repo.clone();
        let presence_service = self.presence_service.clone();
        let server = self.server.clone();

        ctx.spawn(
            async move {
                // 1. Load friend IDs từ DB
                let friend_ids = if let Some(repo) = &friend_repo {
                    match repo.find_friend_ids(&user_id).await {
                        Ok(ids) => ids,
                        Err(e) => {
                            tracing::error!(
                                "Lỗi load friend IDs cho user {}: {}",
                                user_id,
                                e
                            );
                            vec![]
                        }
                    }
                } else {
                    vec![]
                };

                // 2. Set online trong Redis
                if let Some(presence) = &presence_service {
                    if let Err(e) = presence.set_online(user_id).await {
                        tracing::error!("Lỗi set Redis presence cho user {}: {}", user_id, e);
                    }
                }

                // 3. Notify online friends (friend-scoped, not broadcast)
                if !friend_ids.is_empty() {
                    server.do_send(UserPresenceChanged {
                        user_id,
                        is_online: true,
                        friend_ids: friend_ids.clone(),
                        last_seen: None,
                    });

                    // 4. Send initial presence (online friends) to this user
                    server.do_send(SendInitialPresence {
                        user_id,
                        friend_ids: friend_ids.clone(),
                    });
                }

                friend_ids
            }
            .into_actor(self)
            .map(|friend_ids, act, _ctx| {
                // Cache friend IDs in session for disconnect notification
                act.friend_ids = friend_ids;
            }),
        );
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
        // Sử dụng send_group_message vì WS luôn có conversation_id (đã tồn tại).
        // send_group_message hoạt động đúng cho cả direct và group conversations:
        // - increment_unread_count_for_others tăng unread cho tất cả participants trừ sender
        // - Đây là hành vi chính xác cho cả 1-1 và group chat
        ctx.spawn(
            async move {
                // Lưu message vào database
                match service.send_group_message(user_id, content, conversation_id).await {
                    Ok(msg_entity) => {
                        // Serialize MessageEntity thành JSON value cho broadcast
                        let message_value = serde_json::to_value(&msg_entity).unwrap_or_default();

                        // Tạo last message info cho new-message event
                        let last_message = LastMessageInfo {
                            _id: msg_entity.id,
                            content: msg_entity.content.clone(),
                            created_at: msg_entity.created_at.to_rfc3339(),
                            sender: SenderInfo {
                                _id: msg_entity.sender_id,
                                display_name: String::new(), // Will be populated by client
                                avatar_url: None,
                            },
                        };

                        // Broadcast tin nhắn mới với format tương thích Socket.IO
                        let new_msg_event = ServerMessage::new_message(
                            message_value,
                            conversation_id,
                            last_message,
                            msg_entity.created_at.to_rfc3339(),
                            serde_json::json!({}), // unread_counts will be handled by client
                        );

                        server.do_send(BroadcastToRoom {
                            conversation_id,
                            message: new_msg_event,
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

        // Bắt đầu heartbeat check định kỳ
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // Nếu client không phản hồi trong CLIENT_TIMEOUT, disconnect
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                tracing::warn!(
                    "WebSocket session {} heartbeat timeout, disconnecting",
                    act.id
                );
                ctx.stop();
                return;
            }

            // Gửi ping tới client để kiểm tra connection
            act.send_to_client(&ServerMessage::Pong);

            // Refresh Redis presence TTL (piggyback on heartbeat interval)
            if let (Some(user_id), Some(presence)) =
                (act.user_id, act.presence_service.clone())
            {
                actix_web::rt::spawn(async move {
                    if let Err(e) = presence.refresh_presence(user_id).await {
                        tracing::warn!("Lỗi refresh Redis presence cho user {}: {}", user_id, e);
                    }
                });
            }
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::debug!("WebSocket session stopped: {}", self.id);

        // Notify server về disconnect
        self.server.do_send(Disconnect { id: self.id });

        // Presence cleanup: notify friends + set Redis offline
        if let Some(user_id) = self.user_id {
            let friend_ids = self.friend_ids.clone();
            let server = self.server.clone();
            let presence_service = self.presence_service.clone();

            // Spawn async task cho Redis cleanup
            actix_web::rt::spawn(async move {
                // Set offline + last_seen in Redis
                if let Some(presence) = &presence_service {
                    if let Err(e) = presence.set_offline(user_id).await {
                        tracing::error!("Lỗi set Redis offline cho user {}: {}", user_id, e);
                    }
                }

                // Notify friends about offline (with last_seen)
                if !friend_ids.is_empty() {
                    let last_seen = Some(chrono::Utc::now().to_rfc3339());
                    server.do_send(UserPresenceChanged {
                        user_id,
                        is_online: false,
                        friend_ids,
                        last_seen,
                    });
                }
            });
        }
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
