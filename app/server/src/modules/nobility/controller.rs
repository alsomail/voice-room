//! 贵族模块 HTTP handler（T-00066 / T-00067）
//!
//! - `list_tiers_handler`  — GET /api/v1/nobles/tiers（无需鉴权）
//! - `get_me_handler`      — GET /api/v1/nobles/me（JWT 鉴权）
//! - `purchase_handler`    — POST /api/v1/nobles/purchase（JWT 鉴权）
//! - `set_auto_renew_handler` — PATCH /api/v1/nobles/me/auto_renew（JWT 鉴权）

use axum::{
    extract::{Json, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension,
};

use crate::{
    bootstrap::AppState,
    common::{
        auth::AuthContext,
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
};

use super::dto::{AutoRenewRequest, AutoRenewResponse, PurchaseRequest};

/// 解析 Accept-Language header → 语言代码（"ar-SA" → "ar"，默认 "en"）
fn parse_lang(headers: &HeaderMap) -> String {
    headers
        .get("accept-language")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            // 取第一个语言标记
            let first = s.split(',').next().unwrap_or("en").trim();
            let lang_part = first.split('-').next().unwrap_or("en");
            Some(lang_part.to_string())
        })
        .unwrap_or_else(|| "en".to_string())
}

// ─── GET /api/v1/nobles/tiers ─────────────────────────────────────────────────

/// GET /api/v1/nobles/tiers — 无需鉴权；返回所有上架 tier（本地化）
pub async fn list_tiers_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
) -> axum::response::Response {
    let lang = parse_lang(&headers);
    match state.nobility_service.list_tiers(&lang).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── GET /api/v1/nobles/me ────────────────────────────────────────────────────

/// GET /api/v1/nobles/me — JWT 鉴权；返回当前用户贵族状态
pub async fn get_me_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    headers: HeaderMap,
) -> axum::response::Response {
    let lang = parse_lang(&headers);
    match state.nobility_service.get_my_noble(ctx.user_id, &lang).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── POST /api/v1/nobles/purchase ─────────────────────────────────────────────

/// POST /api/v1/nobles/purchase — JWT 鉴权；钻石购买/续费/升级
pub async fn purchase_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Json(req): Json<PurchaseRequest>,
) -> axum::response::Response {
    match state.nobility_service.purchase(ctx.user_id, req).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── PATCH /api/v1/nobles/me/auto_renew ──────────────────────────────────────

/// PATCH /api/v1/nobles/me/auto_renew — JWT 鉴权；切换自动续费
pub async fn set_auto_renew_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Json(req): Json<AutoRenewRequest>,
) -> axum::response::Response {
    match state
        .nobility_service
        .set_auto_renew(ctx.user_id, req.enabled)
        .await
    {
        Ok(enabled) => Json(ApiResponse::ok(
            AutoRenewResponse { auto_renew: enabled },
            rc.request_id(),
        ))
        .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::bootstrap::{build_app, AppState};

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // NC01: GET /nobles/tiers 无需鉴权 → 200
    #[tokio::test]
    async fn nc01_list_tiers_no_auth_returns_200() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/tiers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["code"], 0);
        assert!(body["data"]["tiers"].is_array());
    }

    // NC02: GET /nobles/tiers 返回 6 个 tier
    #[tokio::test]
    async fn nc02_list_tiers_returns_six_tiers() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/tiers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = body_json(response).await;
        let tiers = body["data"]["tiers"].as_array().unwrap();
        assert_eq!(tiers.len(), 6);
    }

    // NC03: GET /nobles/me 无 token → 401
    #[tokio::test]
    async fn nc03_get_me_no_token_returns_401() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // NC04: GET /nobles/me 有效 token → 200 + tier_id=null（FakeNobilityService）
    #[tokio::test]
    async fn nc04_get_me_valid_token_returns_null_tier() {
        use std::time::{SystemTime, UNIX_EPOCH};
        use voice_room_shared::jwt::token::{encode_token, AppClaims};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = AppClaims {
            sub: uuid::Uuid::new_v4().to_string(),
            iss: "voiceroom".to_string(),
            exp: now + 3600,
            iat: now,
        };
        let token = encode_token(&claims, b"test-secret").unwrap();

        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/me")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["code"], 0);
        assert!(body["data"]["tier_id"].is_null());
    }

    // NC05: Accept-Language: ar-SA → tiers 里有阿拉伯文 name（FakeNobilityService 暂时返回英文）
    // 真实实现需要从 DB 读取 name_ar，Fake 返回英文，此测试验证 200 成功即可
    #[tokio::test]
    async fn nc05_accept_language_ar_returns_200() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nobles/tiers")
                    .header("Accept-Language", "ar-SA")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // NC06: POST /nobles/purchase 无 token → 401
    #[tokio::test]
    async fn nc06_purchase_no_token_returns_401() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/nobles/purchase")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"tier_id":"duke","msg_id":"abc"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // NC07: parse_lang 解析正确
    #[test]
    fn nc07_parse_lang_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("accept-language", "ar-SA,ar;q=0.9".parse().unwrap());
        assert_eq!(parse_lang(&headers), "ar");

        let mut headers2 = HeaderMap::new();
        headers2.insert("accept-language", "en-US".parse().unwrap());
        assert_eq!(parse_lang(&headers2), "en");

        // 无 header → 默认 en
        let headers3 = HeaderMap::new();
        assert_eq!(parse_lang(&headers3), "en");
    }
}
