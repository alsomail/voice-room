//! HTTP Handler — POST /api/v1/events/batch（T-00022）
//!
//! ## 功能
//! - 批量接收来自客户端的埋点事件（最多 100 条/次）
//! - JWT 可选（兼容未登录 Splash 阶段）
//! - device_id 必填校验（返回 40002）
//! - properties 超 8KB 截断（EventWriter 内处理）
//! - JWT user_id 与请求体 user_id 不一致时覆盖（EventWriter 内处理）
//!
//! ## 请求格式
//! ```json
//! {
//!   "events": [
//!     {
//!       "event_name": "gift_send_success",
//!       "device_id": "uuid",
//!       "user_id": "uuid|null",
//!       "session_id": "uuid",
//!       "client_ts": 1720000000000,
//!       "properties": {},
//!       "app_version": "1.2.0",
//!       "os_version": "Android 14",
//!       "locale": "ar-SA",
//!       "network_type": "wifi"
//!     }
//!   ]
//! }
//! ```
//!
//! ## 响应格式
//! ```json
//! { "code": 0, "data": { "received": 100, "rejected_indices": [] } }
//! ```

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    bootstrap::AppState,
    common::{error::err_response, response::ApiResponse, RequestContext},
    core::analytics::writer::EventInput,
};

// ─── 请求/响应数据结构 ─────────────────────────────────────────────────────────

/// POST /api/v1/events/batch 请求体
///
/// `#[serde(deny_unknown_fields)]` — 拒绝含未知字段的请求体，防止字段注入。
/// PROTO-BINDING: doc/protocol/HTTP POST /api/v1/events/batch
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchEventsRequest {
    pub events: Vec<EventInput>,
}

/// 成功响应的 data 字段
#[derive(Debug, Serialize)]
pub struct BatchEventsData {
    /// 成功写入的事件数量
    pub received: usize,
    /// 被拒绝的事件索引（0-based）
    pub rejected_indices: Vec<usize>,
}

// ─── Handler ──────────────────────────────────────────────────────────────────

/// POST /api/v1/events/batch — 批量接收埋点事件（JWT 可选）
pub async fn batch_events(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Json(body): Json<BatchEventsRequest>,
) -> axum::response::Response {
    // R1 修复（缺陷 10）：fast-path — 超过 200 条直接拒绝（>100 软限制由 EventWriter 处理为
    // 写前 100 + rejected_indices；>200 视为恶意，立即 40003）
    if body.events.len() > 200 {
        return err_response(
            crate::common::error::AppError::ValidationError(format!(
                "events count {} exceeds hard limit 200",
                body.events.len()
            )),
            rc.request_id(),
        );
    }

    // 可选 JWT：有 token 则解析 user_id，失败则当作未登录
    let jwt_user_id = try_extract_jwt_user_id(&headers, &state.jwt_secret);

    match state.event_writer.persist(body.events, jwt_user_id).await {
        Ok(result) => (
            StatusCode::OK,
            Json(ApiResponse::ok(
                BatchEventsData {
                    received: result.received,
                    rejected_indices: result.rejected_indices,
                },
                rc.request_id(),
            )),
        )
            .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── JWT 可选提取 ─────────────────────────────────────────────────────────────

/// 尝试从 Authorization 头提取 JWT user_id；失败或缺失时返回 None（不报错）
fn try_extract_jwt_user_id(headers: &HeaderMap, jwt_secret: &str) -> Option<Uuid> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;

    let token = auth_header.strip_prefix("Bearer ")?;

    let claims: voice_room_shared::jwt::token::AppClaims =
        voice_room_shared::jwt::token::decode_token(token, jwt_secret.as_bytes(), "voiceroom")
            .ok()?;

    Uuid::parse_str(&claims.sub).ok()
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn make_jwt_header(user_id: &str, secret: &[u8]) -> HeaderMap {
        use std::time::{SystemTime, UNIX_EPOCH};
        use voice_room_shared::jwt::token::{encode_token, AppClaims};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = AppClaims {
            sub: user_id.to_string(),
            iss: "voiceroom".to_string(),
            exp: now + 3600,
            iat: now,
        };
        let token = encode_token(&claims, secret).unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers
    }

    // H-01: 有效 JWT → 提取 user_id
    #[test]
    fn h01_valid_jwt_extracts_user_id() {
        let uid = "550e8400-e29b-41d4-a716-446655440000";
        let headers = make_jwt_header(uid, b"test-secret");
        let result = try_extract_jwt_user_id(&headers, "test-secret");
        assert_eq!(result, Some(Uuid::parse_str(uid).unwrap()));
    }

    // H-02: 无 JWT → 返回 None（不报错）
    #[test]
    fn h02_no_jwt_returns_none() {
        let headers = HeaderMap::new();
        let result = try_extract_jwt_user_id(&headers, "test-secret");
        assert_eq!(result, None);
    }

    // H-03: 无效 JWT → 返回 None（不报错）
    #[test]
    fn h03_invalid_jwt_returns_none() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer invalid-token-string"),
        );
        let result = try_extract_jwt_user_id(&headers, "test-secret");
        assert_eq!(result, None);
    }

    // H-04: 错误签名 JWT → 返回 None
    #[test]
    fn h04_wrong_secret_jwt_returns_none() {
        let uid = "550e8400-e29b-41d4-a716-446655440000";
        let headers = make_jwt_header(uid, b"correct-secret");
        let result = try_extract_jwt_user_id(&headers, "wrong-secret");
        assert_eq!(result, None);
    }

    // H-05: 非 Bearer 格式 → 返回 None
    #[test]
    fn h05_non_bearer_format_returns_none() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Token abc123"),
        );
        let result = try_extract_jwt_user_id(&headers, "test-secret");
        assert_eq!(result, None);
    }
}
