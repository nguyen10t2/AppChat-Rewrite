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
