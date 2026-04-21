//! WebSocket 升级处理器 — JWT 鉴权 + 连接升级入口
//!
//! 路由：`GET /ws?token=<JWT>`
//!
//! 流程：
//! 1. 从 query 参数提取 token
//! 2. 用 jwt_decode 验证（复用 T-00004 逻辑）
//! 3. 鉴权失败 → 401 UNAUTHORIZED
//! 4. 鉴权成功 → WebSocket 升级，进入 handle_socket 生命周期

use axum::{
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use jsonwebtoken::errors::ErrorKind;
use serde::Deserialize;
use voice_room_shared::jwt::token::{decode_token, AppClaims};

use crate::bootstrap::AppState;

use super::connection::handle_socket;

/// GET /ws?token=<JWT> 的查询参数
#[derive(Deserialize)]
pub struct WsQueryParams {
    pub token: Option<String>,
}

/// WebSocket 升级处理器
///
/// 从 `?token=` 提取 JWT，验证后升级为 WebSocket 连接。
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsQueryParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 1. 提取 token
    let token = match params.token {
        Some(t) if !t.is_empty() => t,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // 2. 验证 JWT（复用 T-00004 逻辑，iss = "voiceroom"）
    let claims: AppClaims = match decode_token(&token, state.jwt_secret.as_bytes(), "voiceroom") {
        Ok(c) => c,
        Err(e) => {
            if e.kind() == &ErrorKind::ExpiredSignature {
                tracing::warn!("ws upgrade rejected: token expired");
            } else {
                tracing::warn!("ws upgrade rejected: invalid token");
            }
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    // 3. 解析 user_id
    let user_id = match uuid::Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // 4. 升級 WebSocket
    let registry = state.ws_registry.clone();
    let stats = state.stats_service.clone();
    let room_manager = state.room_manager.clone();
    let room_service = state.room_service.clone();
    let auth_service = state.auth_service.clone();
    let send_gift_service = state.send_gift_service.clone();
    tracing::info!(%user_id, "websocket upgrade accepted");
    ws.on_upgrade(move |socket| {
        handle_socket(socket, user_id, registry, stats, room_manager, room_service, auth_service, send_gift_service)
    })
}
