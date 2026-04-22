use axum::{routing::{get, post}, Router};

use crate::bootstrap::AppState;

use super::controller::{close_room, create_room, get_room, list_rooms, patch_room};

/// 注册房间相关路由
pub fn room_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/rooms", post(create_room).get(list_rooms))
        .route(
            "/api/v1/rooms/{id}",
            get(get_room).delete(close_room).patch(patch_room),
        )
}
