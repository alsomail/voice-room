//! Gift 路由注册

use axum::{routing::get, Router};

use crate::bootstrap::AppState;

use super::handler::list_gifts;

/// 注册礼物 API 路由
///
/// - `GET /api/v1/gifts/list` — 礼物列表（无鉴权，支持 Accept-Language）
pub fn gift_routes() -> Router<AppState> {
    Router::new().route("/api/v1/gifts/list", get(list_gifts))
}
