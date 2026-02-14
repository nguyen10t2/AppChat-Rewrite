/// WebSocket Message Protocol
///
/// Module này định nghĩa các message types được trao đổi giữa client và server
/// thông qua WebSocket connection. Format được giữ tương thích với Socket.IO client
/// để dễ dàng migrate từ Node.js sang Rust.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages được gửi từ client đến server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Xác thực WebSocket connection với JWT token
    Auth { token: String },

    /// Gửi tin nhắn đến conversation
    SendMessage { conversation_id: Uuid, content: String },

    /// Tham gia vào conversation room để nhận real-time updates
    JoinConversation { conversation_id: Uuid },

    /// Rời khỏi conversation room
    LeaveConversation { conversation_id: Uuid },

    /// Bắt đầu typing trong conversation
    TypingStart { conversation_id: Uuid },

    /// Dừng typing trong conversation
    TypingStop { conversation_id: Uuid },

    /// Ping để giữ connection alive
    Ping,
}

/// Thông tin last message gọn nhẹ để gửi trong events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastMessageInfo {
    /// Message ID
    pub _id: Uuid,
    /// Nội dung tin nhắn
    pub content: Option<String>,
    /// Thời gian tạo
    pub created_at: String,
    /// Thông tin sender
    pub sender: SenderInfo,
}

/// Thông tin sender gọn nhẹ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderInfo {
    /// User ID
    pub _id: Uuid,
    /// Display name (có thể empty trong một số context)
    #[serde(default)]
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
}

/// Thông tin conversation gọn nhẹ để gửi trong new-message event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationInfo {
    /// Conversation ID
    pub _id: Uuid,
    /// Last message info
    pub last_message: LastMessageInfo,
    /// Thời gian last message
    pub last_message_at: String,
}

/// Payload cho event new-message (format tương thích Socket.IO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMessagePayload {
    /// Message đầy đủ
    pub message: serde_json::Value,
    /// Thông tin conversation đã cập nhật
    pub conversation: ConversationInfo,
    /// Unread counts theo user ID
    pub unread_counts: serde_json::Value,
}

/// Payload cho event read-message (format tương thích Socket.IO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadMessagePayload {
    /// Conversation đã cập nhật
    pub conversation: serde_json::Value,
    /// Last message info
    pub last_message: LastMessageInfo,
}

/// Messages được gửi từ server đến client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerMessage {
    /// Xác thực thành công
    AuthSuccess { user_id: Uuid },

    /// Xác thực thất bại
    AuthFailed { reason: String },

    /// Event: new-message với đầy đủ thông tin (tương thích Socket.IO)
    /// Đây là format chính được sử dụng
    NewMessage(NewMessagePayload),

    /// Tin nhắn đã được chỉnh sửa
    MessageEdited { conversation_id: Uuid, message_id: Uuid, new_content: String },

    /// Tin nhắn đã bị xóa
    MessageDeleted { conversation_id: Uuid, message_id: Uuid },

    /// User đã đọc messages (read receipt) - format tương thích Socket.IO
    ReadMessage(ReadMessagePayload),

    /// Legacy format - giữ để backward compatibility
    MessagesRead { conversation_id: Uuid, user_id: Uuid, last_read_message_id: Uuid },

    /// Danh sách users đang online
    OnlineUsers { user_ids: Vec<Uuid> },

    /// Một user vừa online (incremental update)
    UserOnline { user_id: Uuid },

    /// Một user vừa offline (incremental update)
    UserOffline { user_id: Uuid, last_seen: Option<String> },

    /// Group chat mới được tạo
    NewGroup { conversation: serde_json::Value },

    /// User bắt đầu typing
    UserTyping { conversation_id: Uuid, user_id: Uuid },

    /// User ngừng typing
    UserStoppedTyping { conversation_id: Uuid, user_id: Uuid },

    /// Pong response cho Ping
    Pong,

    /// Lỗi xảy ra
    Error { message: String },
}

impl ServerMessage {
    /// Tạo new-message event với format tương thích Socket.IO
    #[must_use]
    pub fn new_message(
        message: serde_json::Value,
        conversation_id: Uuid,
        last_message: LastMessageInfo,
        last_message_at: String,
        unread_counts: serde_json::Value,
    ) -> Self {
        Self::NewMessage(NewMessagePayload {
            message,
            conversation: ConversationInfo {
                _id: conversation_id,
                last_message,
                last_message_at,
            },
            unread_counts,
        })
    }

    /// Tạo read-message event với format tương thích Socket.IO
    #[must_use]
    pub fn read_message(conversation: serde_json::Value, last_message: LastMessageInfo) -> Self {
        Self::ReadMessage(ReadMessagePayload { conversation, last_message })
    }
}
