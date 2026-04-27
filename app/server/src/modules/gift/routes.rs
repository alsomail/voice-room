//! Gift 路由注册

use axum::{routing::{get, post}, Router};

use crate::bootstrap::AppState;

use super::handler::{list_gifts, send_gift_http};

/// 注册礼物 API 路由
///
/// - `GET /api/v1/gifts/list` — 礼物列表（无鉴权，支持 Accept-Language）
/// - `POST /api/v1/gifts/send` — 发送礼物（需要 JWT 鉴权，T-00044）
pub fn gift_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/gifts/list", get(list_gifts))
        .route("/api/v1/gifts/send", post(send_gift_http))
}
