//! 聊天消息持久化与历史查询模块（T-00043）
//!
//! - `repository`：`ChatRepository` trait + `RealChatRepository`（PgPool）+ `FakeChatRepository`
//! - `dto`：REST 响应/查询 DTO
//! - `controller`：`GET /api/v1/rooms/:room_id/messages` handler
//! - `routes`：路由注册
//!
//! WS 落库由 `room::handler::chat::handle_send_message` 调用 `ChatRepository::insert_message`。

pub mod controller;
pub mod dto;
pub mod repository;
pub mod routes;

pub use repository::{ChatHistoryRow, ChatRepository, RealChatRepository};
pub use routes::chat_routes;

#[cfg(any(test, feature = "test-utils"))]
pub use repository::FakeChatRepository;
