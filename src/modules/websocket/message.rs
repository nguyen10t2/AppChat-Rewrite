/// WebSocket Message Protocol
///
/// Module này định nghĩa các message types được trao đổi giữa client và server
/// thông qua WebSocket connection.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages được gửi từ client đến server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    /// Xác thực WebSocket connection với JWT token
    #[serde(rename_all = "camelCase")]
    Auth { token: String },

    /// Gửi tin nhắn đến conversation
    #[serde(rename_all = "camelCase")]
    SendMessage { conversation_id: Uuid, content: String },

    /// Tham gia vào conversation room để nhận real-time updates
    #[serde(rename_all = "camelCase")]
    JoinConversation { conversation_id: Uuid },

    /// Rời khỏi conversation room
    #[serde(rename_all = "camelCase")]
    LeaveConversation { conversation_id: Uuid },

    /// Bắt đầu typing trong conversation
    #[serde(rename_all = "camelCase")]
    TypingStart { conversation_id: Uuid },

    /// Dừng typing trong conversation
    #[serde(rename_all = "camelCase")]
    TypingStop { conversation_id: Uuid },

    /// Ping để giữ connection alive
    Ping,
}

/// Messages được gửi từ server đến client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerMessage {
    /// Xác thực thành công
    #[serde(rename_all = "camelCase")]
    AuthSuccess { user_id: Uuid },

    /// Xác thực thất bại
    #[serde(rename_all = "camelCase")]
    AuthFailed { reason: String },

    /// Tin nhắn mới trong conversation
    #[serde(rename_all = "camelCase")]
    NewMessage {
        conversation_id: Uuid,
        message: serde_json::Value, // Full message object
    },

    /// Tin nhắn đã được chỉnh sửa
    #[serde(rename_all = "camelCase")]
    MessageEdited { conversation_id: Uuid, message_id: Uuid, new_content: String },

    /// Tin nhắn đã bị xóa
    #[serde(rename_all = "camelCase")]
    MessageDeleted { conversation_id: Uuid, message_id: Uuid },

    /// User đã đọc messages (read receipt)
    #[serde(rename_all = "camelCase")]
    MessagesRead { conversation_id: Uuid, user_id: Uuid, last_read_message_id: Uuid },

    /// Danh sách users đang online
    #[serde(rename_all = "camelCase")]
    OnlineUsers { user_ids: Vec<Uuid> },

    /// User bắt đầu typing
    #[serde(rename_all = "camelCase")]
    UserTyping { conversation_id: Uuid, user_id: Uuid },

    /// User ngừng typing
    #[serde(rename_all = "camelCase")]
    UserStoppedTyping { conversation_id: Uuid, user_id: Uuid },

    /// Pong response cho Ping
    Pong,

    /// Lỗi xảy ra
    #[serde(rename_all = "camelCase")]
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // === ClientMessage serialization/deserialization ===

    #[test]
    fn test_client_auth_deserialize() {
        let json = r#"{"type":"auth","token":"my-jwt-token"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Auth { token } if token == "my-jwt-token"));
    }

    #[test]
    fn test_client_send_message_deserialize() {
        let id = Uuid::now_v7();
        let json =
            format!(r#"{{"type":"sendMessage","conversationId":"{}","content":"Xin chào!"}}"#, id);
        let msg: ClientMessage = serde_json::from_str(&json).unwrap();
        match msg {
            ClientMessage::SendMessage { conversation_id, content } => {
                assert_eq!(conversation_id, id);
                assert_eq!(content, "Xin chào!");
            }
            _ => panic!("Expected SendMessage variant"),
        }
    }

    #[test]
    fn test_client_join_conversation_deserialize() {
        let id = Uuid::now_v7();
        let json = format!(r#"{{"type":"joinConversation","conversationId":"{}"}}"#, id);
        let msg: ClientMessage = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(msg, ClientMessage::JoinConversation { conversation_id } if conversation_id == id)
        );
    }

    #[test]
    fn test_client_leave_conversation_deserialize() {
        let id = Uuid::now_v7();
        let json = format!(r#"{{"type":"leaveConversation","conversationId":"{}"}}"#, id);
        let msg: ClientMessage = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(msg, ClientMessage::LeaveConversation { conversation_id } if conversation_id == id)
        );
    }

    #[test]
    fn test_client_typing_start_deserialize() {
        let id = Uuid::now_v7();
        let json = format!(r#"{{"type":"typingStart","conversationId":"{}"}}"#, id);
        let msg: ClientMessage = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(msg, ClientMessage::TypingStart { conversation_id } if conversation_id == id)
        );
    }

    #[test]
    fn test_client_typing_stop_deserialize() {
        let id = Uuid::now_v7();
        let json = format!(r#"{{"type":"typingStop","conversationId":"{}"}}"#, id);
        let msg: ClientMessage = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(msg, ClientMessage::TypingStop { conversation_id } if conversation_id == id)
        );
    }

    #[test]
    fn test_client_ping_deserialize() {
        let json = r#"{"type":"ping"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ClientMessage::Ping));
    }

    #[test]
    fn test_invalid_type_returns_error() {
        let json = r#"{"type":"unknownType"}"#;
        let result = serde_json::from_str::<ClientMessage>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_field_returns_error() {
        // sendMessage thiếu content
        let json =
            r#"{"type":"sendMessage","conversationId":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let result = serde_json::from_str::<ClientMessage>(json);
        assert!(result.is_err());
    }

    // === ServerMessage serialization ===

    #[test]
    fn test_server_auth_success_serialize() {
        let uid = Uuid::now_v7();
        let msg = ServerMessage::AuthSuccess { user_id: uid };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"authSuccess\""));
        assert!(json.contains(&uid.to_string()));
    }

    #[test]
    fn test_server_auth_failed_serialize() {
        let msg = ServerMessage::AuthFailed { reason: "Token hết hạn".to_string() };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"authFailed\""));
        assert!(json.contains("Token hết hạn"));
    }

    #[test]
    fn test_server_new_message_serialize() {
        let conv_id = Uuid::now_v7();
        let msg = ServerMessage::NewMessage {
            conversation_id: conv_id,
            message: serde_json::json!({"content": "Hello"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"newMessage\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_server_pong_serialize() {
        let msg = ServerMessage::Pong;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"pong"}"#);
    }

    #[test]
    fn test_server_error_serialize() {
        let msg = ServerMessage::Error { message: "Lỗi hệ thống".to_string() };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("Lỗi hệ thống"));
    }

    #[test]
    fn test_server_online_users_serialize() {
        let u1 = Uuid::now_v7();
        let u2 = Uuid::now_v7();
        let msg = ServerMessage::OnlineUsers { user_ids: vec![u1, u2] };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"onlineUsers\""));
        assert!(json.contains(&u1.to_string()));
        assert!(json.contains(&u2.to_string()));
    }

    #[test]
    fn test_server_user_typing_serialize() {
        let conv_id = Uuid::now_v7();
        let uid = Uuid::now_v7();
        let msg = ServerMessage::UserTyping { conversation_id: conv_id, user_id: uid };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"userTyping\""));
    }

    // === Roundtrip tests ===

    #[test]
    fn test_client_message_roundtrip() {
        let id = Uuid::now_v7();
        let original = ClientMessage::SendMessage {
            conversation_id: id,
            content: "Test message 🇻🇳".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            ClientMessage::SendMessage { conversation_id, content } => {
                assert_eq!(conversation_id, id);
                assert_eq!(content, "Test message 🇻🇳");
            }
            _ => panic!("Roundtrip failed"),
        }
    }

    #[test]
    fn test_server_message_roundtrip() {
        let uid = Uuid::now_v7();
        let original = ServerMessage::AuthSuccess { user_id: uid };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            ServerMessage::AuthSuccess { user_id } => assert_eq!(user_id, uid),
            _ => panic!("Roundtrip failed"),
        }
    }

    #[test]
    fn test_empty_content_allowed() {
        let id = Uuid::now_v7();
        let json = format!(r#"{{"type":"sendMessage","conversationId":"{}","content":""}}"#, id);
        let msg: ClientMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(msg, ClientMessage::SendMessage { content, .. } if content.is_empty()));
    }
}
