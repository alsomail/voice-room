//! Gift HTTP handler
//!
//! - `list_gifts` — GET /api/v1/gifts/list
//!   - Header: `Accept-Language: ar|en`（默认 ar）
//!   - 响应: `{ code:0, data:{ items:[...], version:"..." } }`
//!   - 鉴权: 可选（登录/未登录皆可读取）
//! - `send_gift_http` — POST /api/v1/gifts/send（T-00044）
//!   - 需要 JWT 鉴权
//!   - 响应: `{ code:0, data:{ gift_record_id, sender_balance, receiver_charm } }`

use axum::{extract::State, http::HeaderMap, response::IntoResponse, Extension, Json};

use crate::{
    bootstrap::AppState,
    common::{auth::AuthContext, error::err_response, response::ApiResponse, RequestContext},
};

use super::dto::{SendGiftRequest, SendGiftResponse};
use super::send_gift::{SendGiftError, SendGiftPayload};

/// GET /api/v1/gifts/list
///
/// 返回所有上架礼物列表（按 tier + sort_order 排序），支持多语言名称。
///
/// - `Accept-Language: en`  → name 字段为英文
/// - `Accept-Language: ar`  → name 字段为阿拉伯语（默认）
/// - 其他/缺省              → 阿拉伯语（默认）
pub async fn list_gifts(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
) -> axum::response::Response {
    // 解析 Accept-Language（仅识别 "en"，其余默认 "ar"）
    let lang = parse_lang_header(&headers);

    match state.gift_service.list_active(&lang).await {
        Ok(data) => Json(ApiResponse::ok(data, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// POST /api/v1/gifts/send（T-00044）
///
/// HTTP 礼物发送端点，复用 T-00020 SendGift 核心事务逻辑。
///
/// - **鉴权**: 需要 JWT（通过 AuthContext 注入）
/// - **请求**: `{ room_id, gift_id, receiver_id, count }`
/// - **响应**: `{ code:0, data:{ gift_record_id, sender_balance, receiver_charm } }`
/// - **错误码**:
///   - 40001: INVALID_COUNT（count ≤ 0 或 > 9999）
///   - 40290: INSUFFICIENT_BALANCE（余额不足）
///   - 40402: GIFT_NOT_AVAILABLE（礼物不存在或已下架）
///   - 40403: RECEIVER_UNAVAILABLE（接收者不在麦上）
///
/// **异步广播**: HTTP 200 成功不阻塞广播；广播失败不回滚事务。
pub async fn send_gift_http(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Json(req): Json<SendGiftRequest>,
) -> axum::response::Response {
    // 构建 msg_id（HTTP 场景下使用 UUID）
    let msg_id = uuid::Uuid::new_v4().to_string();

    let payload = SendGiftPayload {
        gift_id: req.gift_id,
        receiver_id: req.receiver_id,
        count: req.count,
        msg_id,
    };

    // 调用核心送礼服务（复用 T-00020 逻辑）
    match state
        .send_gift_service
        .send(ctx.user_id, req.room_id, payload)
        .await
    {
        Ok(result) => {
            let response = SendGiftResponse {
                gift_record_id: result.gift_record_id,
                sender_balance: result.sender_new_balance,
                receiver_charm: result.receiver_new_charm,
            };

            Json(ApiResponse::ok(response, rc.request_id())).into_response()
        }
        Err(e) => {
            use crate::common::error::AppError;

            let app_error = match e {
                SendGiftError::InvalidCount => AppError::InvalidCount("must be 1-9999".to_string()),
                SendGiftError::SenderNotInRoom => AppError::NotFound("sender not in room".to_string()),
                SendGiftError::GiftUnavailable => AppError::GiftNotAvailable,
                SendGiftError::ReceiverUnavailable => AppError::ReceiverUnavailable,
                SendGiftError::InsufficientBalance => AppError::InsufficientBalance,
                SendGiftError::Internal(msg) => AppError::Internal(msg),
            };

            err_response(app_error, rc.request_id())
        }
    }
}

/// 从 Accept-Language 请求头中解析语言代码。
///
/// 规则：
/// - 包含 "en"（如 `en`, `en-US`, `en-GB`）→ 返回 `"en"`
/// - 其他（含缺省/`ar`/`ar-SA` 等）→ 返回 `"ar"`（阿拉伯语为默认）
fn parse_lang_header(headers: &HeaderMap) -> String {
    headers
        .get(axum::http::header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            if s.to_lowercase().starts_with("en") {
                "en".to_string()
            } else {
                "ar".to_string()
            }
        })
        .unwrap_or_else(|| "ar".to_string())
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_headers(lang: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::ACCEPT_LANGUAGE, lang.parse().unwrap());
        headers
    }

    // HC01: 无 Accept-Language 时默认返回 ar
    #[test]
    fn hc01_no_accept_language_defaults_to_ar() {
        let lang = parse_lang_header(&HeaderMap::new());
        assert_eq!(lang, "ar");
    }

    // HC02: Accept-Language: ar → ar
    #[test]
    fn hc02_ar_returns_ar() {
        let lang = parse_lang_header(&make_headers("ar"));
        assert_eq!(lang, "ar");
    }

    // HC03: Accept-Language: en → en
    #[test]
    fn hc03_en_returns_en() {
        let lang = parse_lang_header(&make_headers("en"));
        assert_eq!(lang, "en");
    }

    // HC04: Accept-Language: en-US → en
    #[test]
    fn hc04_en_us_returns_en() {
        let lang = parse_lang_header(&make_headers("en-US"));
        assert_eq!(lang, "en");
    }

    // HC05: Accept-Language: en-GB → en
    #[test]
    fn hc05_en_gb_returns_en() {
        let lang = parse_lang_header(&make_headers("en-GB"));
        assert_eq!(lang, "en");
    }

    // HC06: Accept-Language: zh → ar（非 en 均返回 ar）
    #[test]
    fn hc06_zh_returns_ar() {
        let lang = parse_lang_header(&make_headers("zh-CN"));
        assert_eq!(lang, "ar");
    }

    // HC07: Accept-Language: fr → ar
    #[test]
    fn hc07_fr_returns_ar() {
        let lang = parse_lang_header(&make_headers("fr"));
        assert_eq!(lang, "ar");
    }

    // HC08: Accept-Language 大小写不敏感（EN → en）
    #[test]
    fn hc08_uppercase_en_returns_en() {
        let lang = parse_lang_header(&make_headers("EN"));
        assert_eq!(lang, "en");
    }

    // HC09: HTTP 端到端 — GET /api/v1/gifts/list 返回 200
    #[tokio::test]
    async fn hc09_get_gifts_list_returns_200() {
        use crate::bootstrap::{build_app, AppState};
        use axum::{
            body::Body,
            http::{Request, StatusCode},
        };
        use tower::ServiceExt;

        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/gifts/list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "HC09: GET /api/v1/gifts/list should return 200"
        );
    }
}
