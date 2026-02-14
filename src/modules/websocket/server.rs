/// WebSocket Server Actor
///
/// Server actor chịu trách nhiệm quản lý tất cả WebSocket connections,
/// user sessions, và conversation rooms. Nó xử lý routing messages
/// giữa các clients và maintain state của hệ thống real-time.
use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::events::*;
use super::message::ServerMessage;
use super::session::WebSocketSession;

/// WebSocket server quản lý tất cả client sessions và conversation rooms
pub struct WebSocketServer {
    /// Map: session_id -> session actor address
    /// Lưu tất cả active WebSocket connections
    sessions: HashMap<Uuid, Addr<WebSocketSession>>,

    /// Map: user_id -> set of session_ids
    /// Hỗ trợ multi-device: một user có thể có nhiều sessions (phone, tablet, desktop)
    users: HashMap<Uuid, HashSet<Uuid>>,

    /// Map: conversation_id -> set of user_ids
    /// Track users nào đang ở trong room nào để broadcast messages
    rooms: HashMap<Uuid, HashSet<Uuid>>,
}

impl WebSocketServer {
    /// Tạo WebSocket server mới với state rỗng
    pub fn new() -> Self {
        Self { sessions: HashMap::new(), users: HashMap::new(), rooms: HashMap::new() }
    }

    /// Lấy danh sách user IDs đang online
    fn get_online_users(&self) -> Vec<Uuid> {
        self.users.keys().copied().collect()
    }

    /// Gửi message tới một session cụ thể
    fn send_to_session(&self, session_id: &Uuid, message: ServerMessage) {
        if let Some(session_addr) = self.sessions.get(session_id) {
            session_addr.do_send(message);
        }
    }

    /// Gửi message tới tất cả sessions của một user (multi-device)
    fn send_to_user(&self, user_id: &Uuid, message: ServerMessage) {
        if let Some(session_ids) = self.users.get(user_id) {
            for session_id in session_ids {
                self.send_to_session(session_id, message.clone());
            }
        }
    }
}

impl Actor for WebSocketServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("WebSocket server started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("WebSocket server stopped");
    }
}

/// Handler: User mới connected
impl Handler<Connect> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        tracing::debug!("New WebSocket session connected: {}", msg.id);

        // Lưu session vào map
        self.sessions.insert(msg.id, msg.addr);
    }
}

/// Handler: User disconnected
impl Handler<Disconnect> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        tracing::debug!("WebSocket session disconnected: {}", msg.id);

        // Xóa session
        self.sessions.remove(&msg.id);

        // Tìm user có session này và xóa session khỏi set
        let mut user_to_remove: Option<Uuid> = None;
        for (&user_id, sessions) in self.users.iter_mut() {
            if sessions.remove(&msg.id) {
                tracing::debug!("Removed session {} from user {}", msg.id, user_id);
                // Nếu user không còn session nào, đánh dấu để xóa
                if sessions.is_empty() {
                    user_to_remove = Some(user_id);
                }
                break;
            }
        }

        // Xóa user nếu không còn session nào
        if let Some(user_id) = user_to_remove {
            self.users.remove(&user_id);

            // Xóa user khỏi tất cả rooms
            for room_users in self.rooms.values_mut() {
                room_users.remove(&user_id);
            }

            // Clean up empty rooms
            self.rooms.retain(|_, users| !users.is_empty());

            tracing::info!(
                "User {} fully disconnected (no more sessions) and removed from all rooms",
                user_id
            );

            // NOTE: Presence notification được xử lý bởi UserPresenceChanged event
            // từ session actor (session có friend_ids và presence_service)
        }
    }
}

/// Handler: Authenticate user
impl Handler<Authenticate> for WebSocketServer {
    type Result = Result<Uuid, String>;

    fn handle(&mut self, msg: Authenticate, _: &mut Context<Self>) -> Self::Result {
        tracing::info!("User {} authenticated on session {}", msg.user_id, msg.session_id);

        // Thêm session vào set của user (hỗ trợ multi-device)
        let sessions = self.users.entry(msg.user_id).or_default();
        sessions.insert(msg.session_id);

        tracing::info!("User {} now has {} active session(s)", msg.user_id, sessions.len());

        // NOTE: Presence notification (online-users, user-online) được xử lý
        // bởi session actor sau khi load friend list và set Redis presence

        Ok(msg.user_id)
    }
}

/// Handler: Join conversation room
impl Handler<JoinRoom> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: JoinRoom, _: &mut Context<Self>) {
        tracing::debug!("User {} joining conversation {}", msg.user_id, msg.conversation_id);

        // Thêm user vào room
        self.rooms.entry(msg.conversation_id).or_default().insert(msg.user_id);

        tracing::info!(
            "User {} joined conversation {} ({} users in room)",
            msg.user_id,
            msg.conversation_id,
            self.rooms.get(&msg.conversation_id).map_or(0, HashSet::len)
        );
    }
}

/// Handler: Leave conversation room
impl Handler<LeaveRoom> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: LeaveRoom, _: &mut Context<Self>) {
        if let Some(room) = self.rooms.get_mut(&msg.conversation_id) {
            room.remove(&msg.user_id);

            tracing::debug!(
                "User {} left conversation {} ({} users remaining)",
                msg.user_id,
                msg.conversation_id,
                room.len()
            );

            // Clean up empty room
            if room.is_empty() {
                self.rooms.remove(&msg.conversation_id);
                tracing::debug!("Room {} empty, removed", msg.conversation_id);
            }
        }
    }
}

/// Handler: Broadcast message tới room
impl Handler<BroadcastToRoom> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastToRoom, _: &mut Context<Self>) {
        if let Some(room_users) = self.rooms.get(&msg.conversation_id) {
            let mut sent_count = 0;

            for &user_id in room_users {
                // Skip user nếu được chỉ định (ví dụ: sender không cần nhận lại)
                if let Some(skip_id) = msg.skip_user_id {
                    if user_id == skip_id {
                        continue;
                    }
                }

                // Lấy tất cả sessions của user và gửi message tới mỗi session (multi-device)
                if let Some(session_ids) = self.users.get(&user_id) {
                    for session_id in session_ids {
                        self.send_to_session(session_id, msg.message.clone());
                        sent_count += 1;
                    }
                }
            }

            tracing::debug!(
                "Broadcast to room {}: sent to {} sessions",
                msg.conversation_id,
                sent_count
            );
        } else {
            tracing::debug!("Attempted to broadcast to non-existent room: {}", msg.conversation_id);
        }
    }
}

/// Handler: Gửi message cho user cụ thể (tất cả devices)
impl Handler<SendToUser> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: SendToUser, _: &mut Context<Self>) {
        if let Some(session_ids) = self.users.get(&msg.user_id) {
            let session_count = session_ids.len();
            for session_id in session_ids {
                self.send_to_session(session_id, msg.message.clone());
            }
            tracing::debug!("Sent message to user {} ({} sessions)", msg.user_id, session_count);
        } else {
            tracing::debug!("User {} not online, message not sent", msg.user_id);
        }
    }
}

/// Handler: Gửi message đến nhiều users (dùng cho new-group notification)
impl Handler<SendToUsers> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: SendToUsers, _: &mut Context<Self>) {
        let mut sent_count = 0;

        for user_id in &msg.user_ids {
            if let Some(session_ids) = self.users.get(user_id) {
                for session_id in session_ids {
                    self.send_to_session(session_id, msg.message.clone());
                    sent_count += 1;
                }
            }
        }

        tracing::debug!("Sent message to {} users ({} total sessions)", msg.user_ids.len(), sent_count);
    }
}

/// Handler: Lấy online users
impl Handler<GetOnlineUsers> for WebSocketServer {
    type Result = Vec<Uuid>;

    fn handle(&mut self, _: GetOnlineUsers, _: &mut Context<Self>) -> Self::Result {
        self.get_online_users()
    }
}

/// Handler: Broadcast tới tất cả users
impl Handler<BroadcastToAll> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: BroadcastToAll, _: &mut Context<Self>) {
        for session_addr in self.sessions.values() {
            session_addr.do_send(msg.message.clone());
        }

        tracing::debug!("Broadcast to all: {} sessions", self.sessions.len());
    }
}

/// Implement Message trait cho ServerMessage để có thể send tới sessions
impl Message for ServerMessage {
    type Result = ();
}

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler: User thay đổi trạng thái presence
/// Chỉ gửi notification đến friends đang online (friend-scoped fan-out)
/// Giống cách Messenger/IG chỉ notify cho contacts, không phải all users
impl Handler<UserPresenceChanged> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: UserPresenceChanged, _: &mut Context<Self>) {
        let event = if msg.is_online {
            ServerMessage::UserOnline { user_id: msg.user_id }
        } else {
            ServerMessage::UserOffline {
                user_id: msg.user_id,
                last_seen: msg.last_seen,
            }
        };

        let mut notified_count = 0;
        for friend_id in &msg.friend_ids {
            if self.users.contains_key(friend_id) {
                self.send_to_user(friend_id, event.clone());
                notified_count += 1;
            }
        }

        tracing::info!(
            "Presence change: user {} {} → notified {}/{} friends",
            msg.user_id,
            if msg.is_online { "online" } else { "offline" },
            notified_count,
            msg.friend_ids.len()
        );
    }
}

/// Handler: Gửi initial presence state cho user vừa connect
/// Kiểm tra friends nào đang online trong server's users map
/// và gửi OnlineUsers list chỉ chứa friends (không phải tất cả users)
impl Handler<SendInitialPresence> for WebSocketServer {
    type Result = ();

    fn handle(&mut self, msg: SendInitialPresence, _: &mut Context<Self>) {
        // Lọc chỉ friends đang online
        let online_friend_ids: Vec<Uuid> = msg
            .friend_ids
            .iter()
            .filter(|fid| self.users.contains_key(fid))
            .copied()
            .collect();

        let message = ServerMessage::OnlineUsers {
            user_ids: online_friend_ids.clone(),
        };

        // Gửi đến tất cả sessions của user
        self.send_to_user(&msg.user_id, message);

        tracing::debug!(
            "Sent initial presence to user {}: {}/{} friends online",
            msg.user_id,
            online_friend_ids.len(),
            msg.friend_ids.len()
        );
    }
}
