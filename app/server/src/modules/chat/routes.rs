//! 聊天历史查询路由（T-00043）+ REST 写入路由（T-00045）

use axum::{
    routing::{get, post},
    Router,
};

use crate::bootstrap::AppState;

use super::controller::{list_room_messages_handler, send_chat_message_handler};

/// 注册聊天相关路由。
///
/// - `GET /api/v1/rooms/:room_id/messages` — 历史分页查询（T-00043）
/// - `POST /api/v1/chat-messages` — 写入聊天消息并广播至房间 WS（T-00045）
pub fn chat_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/rooms/{room_id}/messages",
            get(list_room_messages_handler),
        )
        .route(
            "/api/v1/chat-messages",
            post(send_chat_message_handler),
        )
}
