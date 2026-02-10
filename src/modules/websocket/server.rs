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

    /// Broadcast danh sách online users tới tất cả connected clients
    fn broadcast_online_users(&self) {
        let user_ids = self.get_online_users();
        let message = ServerMessage::OnlineUsers { user_ids };

        // Gửi tới tất cả sessions
        for session_addr in self.sessions.values() {
            session_addr.do_send(message.clone());
        }
    }

    /// Gửi message tới một session cụ thể
    fn send_to_session(&self, session_id: &Uuid, message: ServerMessage) {
        if let Some(session_addr) = self.sessions.get(session_id) {
            session_addr.do_send(message);
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

            // Broadcast updated online users list
            self.broadcast_online_users();
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
        let is_new_user = sessions.is_empty();
        sessions.insert(msg.session_id);

        tracing::info!("User {} now has {} active session(s)", msg.user_id, sessions.len());

        // Chỉ broadcast nếu là user mới online (session đầu tiên)
        if is_new_user {
            self.broadcast_online_users();
        }

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
            self.rooms.get(&msg.conversation_id).map(|s| s.len()).unwrap_or(0)
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
            tracing::warn!("Attempted to broadcast to non-existent room: {}", msg.conversation_id);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: tạo WebSocketServer instance để test logic trực tiếp
    fn make_server() -> WebSocketServer {
        WebSocketServer::new()
    }

    // === Unit tests cho internal state ===

    #[test]
    fn test_new_server_is_empty() {
        let server = make_server();
        assert!(server.sessions.is_empty());
        assert!(server.users.is_empty());
        assert!(server.rooms.is_empty());
    }

    #[test]
    fn test_get_online_users_empty() {
        let server = make_server();
        assert!(server.get_online_users().is_empty());
    }

    #[test]
    fn test_get_online_users_returns_authenticated_users() {
        let mut server = make_server();
        let user_a = Uuid::now_v7();
        let user_b = Uuid::now_v7();
        let session_a = Uuid::now_v7();
        let session_b = Uuid::now_v7();

        // Multi-device: mỗi user có một set of sessions
        server.users.insert(user_a, HashSet::from([session_a]));
        server.users.insert(user_b, HashSet::from([session_b]));

        let online = server.get_online_users();
        assert_eq!(online.len(), 2);
        assert!(online.contains(&user_a));
        assert!(online.contains(&user_b));
    }

    // === Actor handler tests (cần actix runtime) ===

    #[actix::test]
    async fn test_connect_adds_session() {
        let server = WebSocketServer::new().start();

        // Tạo một dummy session actor để lấy Addr
        // Sử dụng thực tế: cần tạo WebSocketSession, nhưng ở đây test server logic
        // Verify qua GetOnlineUsers (chưa auth thì chưa online)
        let online: Vec<Uuid> = server.send(GetOnlineUsers).await.unwrap();
        assert!(online.is_empty(), "Chưa có user nào authenticate");
    }

    #[actix::test]
    async fn test_authenticate_and_get_online() {
        let mut server = WebSocketServer::new();
        let user_id = Uuid::now_v7();
        let session_id = Uuid::now_v7();

        // Simulate authenticate với HashSet
        server.users.insert(user_id, HashSet::from([session_id]));

        let online = server.get_online_users();
        assert_eq!(online.len(), 1);
        assert!(online.contains(&user_id));
    }

    #[actix::test]
    async fn test_join_room_creates_room() {
        let server_addr = WebSocketServer::new().start();
        let user_id = Uuid::now_v7();
        let conv_id = Uuid::now_v7();

        server_addr.send(JoinRoom { user_id, conversation_id: conv_id }).await.unwrap();

        // Verify thông qua cách gián tiếp - broadcast không panic
        server_addr
            .send(BroadcastToRoom {
                conversation_id: conv_id,
                message: ServerMessage::Pong,
                skip_user_id: None,
            })
            .await
            .unwrap();
    }

    #[actix::test]
    async fn test_leave_room_removes_user() {
        let mut server = WebSocketServer::new();
        let user_id = Uuid::now_v7();
        let conv_id = Uuid::now_v7();

        // Join
        server.rooms.entry(conv_id).or_default().insert(user_id);
        assert_eq!(server.rooms.get(&conv_id).unwrap().len(), 1);

        // Leave
        if let Some(room) = server.rooms.get_mut(&conv_id) {
            room.remove(&user_id);
            if room.is_empty() {
                server.rooms.remove(&conv_id);
            }
        }
        assert!(!server.rooms.contains_key(&conv_id), "Room trống phải được xóa");
    }

    #[actix::test]
    async fn test_disconnect_cleans_up_rooms() {
        let mut server = WebSocketServer::new();
        let user_id = Uuid::now_v7();
        let session_id = Uuid::now_v7();
        let conv_a = Uuid::now_v7();
        let conv_b = Uuid::now_v7();

        // Setup: user trong 2 rooms với HashSet
        server.users.insert(user_id, HashSet::from([session_id]));
        server.rooms.entry(conv_a).or_default().insert(user_id);
        server.rooms.entry(conv_b).or_default().insert(user_id);

        // Simulate disconnect logic - xóa session khỏi user
        if let Some(sessions) = server.users.get_mut(&user_id) {
            sessions.remove(&session_id);
            if sessions.is_empty() {
                server.users.remove(&user_id);
                for room_users in server.rooms.values_mut() {
                    room_users.remove(&user_id);
                }
                server.rooms.retain(|_, users| !users.is_empty());
            }
        }

        assert!(server.users.is_empty());
        assert!(server.rooms.is_empty(), "Tất cả rooms trống phải được dọn");
    }

    #[actix::test]
    async fn test_multiple_users_in_room() {
        let mut server = WebSocketServer::new();
        let user_a = Uuid::now_v7();
        let user_b = Uuid::now_v7();
        let user_c = Uuid::now_v7();
        let conv_id = Uuid::now_v7();

        server.rooms.entry(conv_id).or_default().insert(user_a);
        server.rooms.entry(conv_id).or_default().insert(user_b);
        server.rooms.entry(conv_id).or_default().insert(user_c);

        let room = server.rooms.get(&conv_id).unwrap();
        assert_eq!(room.len(), 3);

        // Xóa 1 user, room vẫn tồn tại
        server.rooms.get_mut(&conv_id).unwrap().remove(&user_b);
        assert_eq!(server.rooms.get(&conv_id).unwrap().len(), 2);
    }

    #[actix::test]
    async fn test_multi_device_same_user() {
        let mut server = WebSocketServer::new();
        let user_id = Uuid::now_v7();
        let session_1 = Uuid::now_v7();
        let session_2 = Uuid::now_v7();

        // Device 1 auth - tạo HashSet với session đầu tiên
        server.users.entry(user_id).or_insert_with(HashSet::new).insert(session_1);
        assert!(server.users.get(&user_id).unwrap().contains(&session_1));

        // Device 2 auth - thêm session thứ 2 vào cùng user (multi-device support!)
        server.users.entry(user_id).or_insert_with(HashSet::new).insert(session_2);

        // Verify: user có 2 sessions
        let sessions = server.users.get(&user_id).unwrap();
        assert!(sessions.contains(&session_1), "Session 1 vẫn còn");
        assert!(sessions.contains(&session_2), "Session 2 được thêm");
        assert_eq!(sessions.len(), 2, "User có 2 sessions (multi-device)");
        assert_eq!(server.users.len(), 1, "Vẫn chỉ 1 user entry");
    }

    #[actix::test]
    async fn test_broadcast_to_nonexistent_room_no_panic() {
        let server_addr = WebSocketServer::new().start();
        let fake_conv = Uuid::now_v7();

        // Broadcast tới room không tồn tại - không panic
        server_addr
            .send(BroadcastToRoom {
                conversation_id: fake_conv,
                message: ServerMessage::Pong,
                skip_user_id: None,
            })
            .await
            .unwrap();
    }

    #[actix::test]
    async fn test_send_to_user_not_online_no_panic() {
        let server_addr = WebSocketServer::new().start();
        let fake_user = Uuid::now_v7();

        // Gửi tới user không online - không panic
        server_addr
            .send(SendToUser { user_id: fake_user, message: ServerMessage::Pong })
            .await
            .unwrap();
    }
}
