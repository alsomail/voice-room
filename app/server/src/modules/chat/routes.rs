//! 聊天历史查询路由（T-00043）

use axum::{routing::get, Router};

use crate::bootstrap::AppState;

use super::controller::list_room_messages_handler;

/// 注册聊天相关路由。
///
/// `GET /api/v1/rooms/:room_id/messages`
pub fn chat_routes() -> Router<AppState> {
    Router::new().route(
        "/api/v1/rooms/{room_id}/messages",
        get(list_room_messages_handler),
    )
}
