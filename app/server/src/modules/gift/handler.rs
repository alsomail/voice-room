//! Gift HTTP handler
//!
//! - `list_gifts` — GET /api/v1/gifts/list
//!   - Header: `Accept-Language: ar|en`（默认 ar）
//!   - 响应: `{ code:0, data:{ items:[...], version:"..." } }`
//!   - 鉴权: 可选（登录/未登录皆可读取）

use axum::{extract::State, http::HeaderMap, response::IntoResponse, Extension, Json};

use crate::{
    bootstrap::AppState,
    common::{
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
};

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
        headers.insert(
            axum::http::header::ACCEPT_LANGUAGE,
            lang.parse().unwrap(),
        );
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
        use axum::{body::Body, http::{Request, StatusCode}};
        use tower::ServiceExt;
        use crate::bootstrap::{build_app, AppState};

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

        assert_eq!(response.status(), StatusCode::OK, "HC09: GET /api/v1/gifts/list should return 200");
    }
}
