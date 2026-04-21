//! 钱包模块 HTTP handler
//!
//! - `get_balance`       — GET /api/v1/wallet/balance（需 JWT）
//! - `list_transactions` — GET /api/v1/wallet/transactions（需 JWT）

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};

use crate::{
    bootstrap::AppState,
    common::{
        auth::AuthContext,
        error::{err_response, AppError},
        response::ApiResponse,
        RequestContext,
    },
};

use super::dto::{BalanceResponse, Paginated, TransactionItem, TransactionQuery};

/// GET /api/v1/wallet/balance（需 JWT）
///
/// 返回当前用户最新 diamond_balance。
/// - 200: `{ code:0, data:{ diamond_balance: N } }`
/// - 401: 未登录
pub async fn get_balance(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
) -> axum::response::Response {
    match state.wallet_service.get_balance(ctx.user_id).await {
        Ok(balance) => Json(ApiResponse::ok(
            BalanceResponse {
                diamond_balance: balance,
            },
            rc.request_id(),
        ))
        .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/wallet/transactions?page=&size=&type=（需 JWT）
///
/// 分页查询用户流水（按时间倒序）。
/// - 200: `{ code:0, data:{ total, page, size, items:[...] } }`
/// - 400 / 40003: page < 1 或 size > 100
/// - 401: 未登录
pub async fn list_transactions(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Query(query): Query<TransactionQuery>,
) -> axum::response::Response {
    // ── 参数校验 ─────────────────────────────────────────────────────────────
    if query.page == 0 {
        return err_response(
            AppError::ValidationError("page must be >= 1".to_string()),
            rc.request_id(),
        );
    }
    if query.size == 0 || query.size > 100 {
        return err_response(
            AppError::ValidationError("size must be between 1 and 100".to_string()),
            rc.request_id(),
        );
    }

    // ── 查询服务 ─────────────────────────────────────────────────────────────
    match state
        .wallet_service
        .list_txns(ctx.user_id, query.page, query.size, query.txn_type)
        .await
    {
        Ok(page) => {
            let resp = Paginated {
                total: page.total,
                page: page.page,
                size: page.size,
                items: page.items.into_iter().map(TransactionItem::from).collect(),
            };
            Json(ApiResponse::ok(resp, rc.request_id())).into_response()
        }
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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

    // HC01: GET /wallet/balance 无 token → 401
    #[tokio::test]
    async fn hc01_get_balance_no_token_returns_401() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/wallet/balance")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // HC02: GET /wallet/transactions 无 token → 401
    #[tokio::test]
    async fn hc02_get_transactions_no_token_returns_401() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/wallet/transactions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // HC03: page=0 → 40003
    #[tokio::test]
    async fn hc03_page_zero_returns_40003() {
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
                    .uri("/api/v1/wallet/transactions?page=0&size=20")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_json(response).await;
        assert_eq!(body["code"], 40003);
    }

    // HC04: size=200 → 40003
    #[tokio::test]
    async fn hc04_size_200_returns_40003() {
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
                    .uri("/api/v1/wallet/transactions?page=1&size=200")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_json(response).await;
        assert_eq!(body["code"], 40003);
    }

    // HC05: 有效 token + FakeWalletService → balance=0
    #[tokio::test]
    async fn hc05_valid_token_returns_balance_zero() {
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
                    .uri("/api/v1/wallet/balance")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["code"], 0);
        assert_eq!(body["data"]["diamond_balance"], 0);
    }
}
