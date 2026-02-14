/// WebSocket Actor Events
///
/// Module này định nghĩa các messages được trao đổi giữa các actors
/// trong WebSocket system (giữa Session actors và Server actor).
use actix::prelude::*;
use uuid::Uuid;

use super::message::ServerMessage;
use super::session::WebSocketSession;

/// Event: User connected đến WebSocket server
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    /// Unique session ID
    pub id: Uuid,
    /// Address của session actor để có thể gửi messages
    pub addr: Addr<WebSocketSession>,
}

/// Event: User disconnected khỏi WebSocket server
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    /// Session ID cần disconnect
    pub id: Uuid,
}

/// Event: User đã xác thực thành công
#[derive(Message)]
#[rtype(result = "Result<Uuid, String>")]
pub struct Authenticate {
    /// Session ID đang authenticate
    pub session_id: Uuid,
    /// User ID sau khi authenticate
    pub user_id: Uuid,
}

/// Event: User tham gia vào conversation room
#[derive(Message)]
#[rtype(result = "()")]
pub struct JoinRoom {
    /// User ID muốn join room
    pub user_id: Uuid,
    /// Conversation ID (room ID)
    pub conversation_id: Uuid,
}

/// Event: User rời khỏi conversation room
#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaveRoom {
    /// User ID muốn leave room
    pub user_id: Uuid,
    /// Conversation ID (room ID)
    pub conversation_id: Uuid,
}

/// Event: Broadcast message tới tất cả users trong room
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastToRoom {
    /// Conversation ID (room ID) cần broadcast
    pub conversation_id: Uuid,
    /// Message cần broadcast
    pub message: ServerMessage,
    /// Optional: Không gửi đến user này (ví dụ: sender)
    pub skip_user_id: Option<Uuid>,
}

/// Event: Gửi message cho một user cụ thể
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendToUser {
    /// User ID cần nhận message
    pub user_id: Uuid,
    /// Message cần gửi
    pub message: ServerMessage,
}

/// Event: Lấy danh sách users đang online
#[derive(Message)]
#[rtype(result = "Vec<Uuid>")]
pub struct GetOnlineUsers;

/// Event: Broadcast tới tất cả users connected
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct BroadcastToAll {
    /// Message cần broadcast
    pub message: ServerMessage,
}

/// Event: Gửi message đến nhiều users (dùng cho new-group)
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct SendToUsers {
    /// Danh sách User IDs cần nhận message
    pub user_ids: Vec<Uuid>,
    /// Message cần gửi
    pub message: ServerMessage,
}

/// Event: User thay đổi trạng thái presence (online/offline)
/// Server sẽ chỉ gửi notification đến friends đang online (friend-scoped)
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct UserPresenceChanged {
    /// User ID thay đổi trạng thái
    pub user_id: Uuid,
    /// True = online, False = offline
    pub is_online: bool,
    /// Danh sách friend IDs để notify
    pub friend_ids: Vec<Uuid>,
    /// Last seen timestamp (chỉ có khi offline)
    pub last_seen: Option<String>,
}

/// Event: Gửi initial presence state cho user vừa connect
/// Server kiểm tra friends nào đang online và gửi danh sách
#[derive(Message)]
#[rtype(result = "()")]
pub struct SendInitialPresence {
    /// User ID vừa connect
    pub user_id: Uuid,
    /// Danh sách friend IDs để kiểm tra
    pub friend_ids: Vec<Uuid>,
}
