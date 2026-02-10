/// WebSocket Module
///
/// Module này cung cấp real-time communication capability cho chat application
/// thông qua WebSocket protocol. Nó bao gồm:
///
/// - Message protocol (ClientMessage & ServerMessage)
/// - WebSocket Server actor (quản lý connections và rooms)
/// - WebSocket Session actor (xử lý từng connection)
/// - HTTP handler (upgrade HTTP thành WebSocket)
pub mod events;
pub mod handler;
pub mod message;
pub mod server;
pub mod session;
