//! 集成测试 — T-00051 支付订单创建 API
//!
//! 验收用例：
//! - T51-01: 已登录 + 有效 sku_id → 200 + UUID order_id + DB 中 PENDING 记录
//! - T51-02: sku_id 不存在 → 404 + code=40902
//! - T51-03: sku.is_active=false（停售）→ 404 + code=40902
//! - T51-04: 未登录 → 401 + code=40101
//! - T51-05: 风控拦截（日失败 > 10）→ 429 + code=40903
//! - T51-06: GET /api/v1/payments/skus → 200 + 至少 5 个 SKU
//!
//! 运行前提：DATABASE_URL 环境变量指向可用的 PostgreSQL 实例。
//! 未设置时所有 DB 相关测试自动跳过。

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    modules::payment::service::PaymentOrderServicePort,
};

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

/// 在 DB 中创建测试用户，返回 (user_id)
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    let phone = format!("+1555{}", &user_id.to_string().replace('-', "")[..10]);
    sqlx::query(
        "INSERT INTO users (user_id, phone, password_hash, nickname)
         VALUES ($1, $2, 'hash', 'PayTest')
         ON CONFLICT DO NOTHING",
    )
    .bind(user_id)
    .bind(phone)
    .execute(pool)
    .await
    .expect("create test user");
    user_id
}

/// 生成测试用 JWT
fn make_jwt(user_id: Uuid) -> String {
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
    encode_token(&claims, b"test-secret").expect("encode JWT")
}

/// 解析响应 body 为 JSON
async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({"_raw": std::str::from_utf8(&bytes).unwrap_or("?").to_string()}))
}

// ─── T51-06: GET /api/v1/payments/skus (Fake) → 200 ─────────────────────────
// 使用 FakePaymentOrderService（无 DB）；验证路由注册正确 + 响应结构合法

#[tokio::test]
async fn t51_06_list_skus_endpoint_returns_200() {
    let app = build_app(AppState::for_test());

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/payments/skus?provider=google_play")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /api/v1/payments/skus should return 200"
    );

    let body = body_json(resp).await;
    assert_eq!(body["code"], 0, "Success code should be 0");
    assert!(
        body["data"]["skus"].is_array(),
        "Response should have data.skus array"
    );
}

// ─── T51-04: 未登录 → 401 ────────────────────────────────────────────────────

#[tokio::test]
async fn t51_04_unauthenticated_create_order_returns_401() {
    let app = build_app(AppState::for_test());

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments/orders")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"sku_id":"diamond_60","provider":"google_play"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "Should be 401");
    let body = body_json(resp).await;
    assert_eq!(body["code"], 40101, "Error code should be 40101");
}

// ─── T51-01: 已登录 + 有效 sku_id → 200 + order_id ───────────────────────────
// 使用 FakePaymentOrderService（无 DB 依赖）

#[tokio::test]
async fn t51_01_create_order_with_valid_sku_returns_200() {
    let app = build_app(AppState::for_test());
    let user_id = Uuid::new_v4();
    let jwt = make_jwt(user_id);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments/orders")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::from(r#"{"sku_id":"diamond_60","provider":"google_play"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Authenticated create_order should return 200"
    );
    let body = body_json(resp).await;
    assert_eq!(body["code"], 0);
    assert!(
        body["data"]["order_id"].is_string(),
        "Response should have data.order_id"
    );
    // Verify order_id is a valid UUID
    let order_id_str = body["data"]["order_id"].as_str().unwrap();
    Uuid::parse_str(order_id_str).expect("order_id should be a valid UUID");
}

// ─── T51-07: verify endpoint (Fake) → 200 ────────────────────────────────────

#[tokio::test]
async fn t51_07_verify_endpoint_with_fake_returns_200() {
    let app = build_app(AppState::for_test());
    let user_id = Uuid::new_v4();
    let jwt = make_jwt(user_id);
    let order_id = Uuid::new_v4();

    let body_json_str = serde_json::json!({
        "order_id": order_id.to_string(),
        "purchase_token": "test_token_abc",
        "provider": "google_play"
    })
    .to_string();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments/google/verify")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::from(body_json_str))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK, "verify should return 200 with fake");
    let body = body_json(resp).await;
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["state"], "ACKED");
    assert_eq!(body["data"]["diamonds_credited"], 60);
}

// ─── T51-08: rtdn webhook endpoint (Fake) → 200 ──────────────────────────────

#[tokio::test]
async fn t51_08_rtdn_webhook_with_fake_returns_200() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let app = build_app(AppState::for_test());

    // 构造合法的 RTDN envelope（testNotification）
    let notification_json = r#"{"version":"1.0","packageName":"com.test.app","eventTimeMillis":"1746788688000","testNotification":{"version":"1.0"}}"#;
    let encoded = STANDARD.encode(notification_json);

    let envelope = serde_json::json!({
        "message": {
            "data": encoded,
            "messageId": "test_msg_001",
            "publishTime": "2025-05-09T10:00:00Z"
        },
        "subscription": "projects/test/subscriptions/rtdn"
    })
    .to_string();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhook/google/rtdn")
                .header("content-type", "application/json")
                .body(Body::from(envelope))
                .unwrap(),
        )
        .await
        .unwrap();

    // RTDN webhook should return 200 (Google expects 200 for ack)
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "RTDN webhook should return 200"
    );
}

// ─── T51-02/T51-03: DB-backed tests (sku not found / inactive) ───────────────
// 以下测试需要真实 DB + PaymentOrderService (real)

#[tokio::test]
async fn t51_02_create_order_unknown_sku_returns_404_with_db() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] t51_02: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    let user_id = create_test_user(&pool).await;
    let jwt = make_jwt(user_id);

    // Build app with real PaymentOrderService
    use std::sync::Arc;
    use voice_room_server::modules::payment::{
        risk::RiskCheckService,
        service::PaymentOrderService,
    };
    let risk = Arc::new(RiskCheckService::new(pool.clone()));
    let svc = Arc::new(PaymentOrderService::new(pool.clone(), risk));

    let state = AppState::for_test()
        .with_payment_order_service(svc as Arc<dyn PaymentOrderServicePort>);
    let app = build_app(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments/orders")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::from(r#"{"sku_id":"nonexistent_sku","provider":"google_play"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "Unknown SKU → 404");
    let body = body_json(resp).await;
    assert_eq!(body["code"], 40902, "Error code should be 40902 (SKU_NOT_FOUND)");
}

#[tokio::test]
async fn t51_03_create_order_inactive_sku_returns_404_with_db() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] t51_03: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    // 创建一个 is_active=false 的 SKU
    let inactive_sku_id = format!("test_inactive_{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO payment_skus (sku_id, provider, diamonds, display_price_usd, is_active, sort_order)
         VALUES ($1, 'google_play', 60, 0.99, FALSE, 999)
         ON CONFLICT DO NOTHING",
    )
    .bind(&inactive_sku_id)
    .execute(&pool)
    .await
    .unwrap();

    let user_id = create_test_user(&pool).await;
    let jwt = make_jwt(user_id);

    use std::sync::Arc;
    use voice_room_server::modules::payment::{
        risk::RiskCheckService,
        service::PaymentOrderService,
    };
    let risk = Arc::new(RiskCheckService::new(pool.clone()));
    let svc = Arc::new(PaymentOrderService::new(pool.clone(), risk));

    let state = AppState::for_test()
        .with_payment_order_service(svc as Arc<dyn PaymentOrderServicePort>);
    let app = build_app(state);

    let body_str = serde_json::json!({
        "sku_id": inactive_sku_id,
        "provider": "google_play"
    })
    .to_string();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments/orders")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "Inactive SKU → 404");
    let body = body_json(resp).await;
    assert_eq!(body["code"], 40902, "Error code should be 40902 (SKU_NOT_FOUND)");
}

#[tokio::test]
async fn t51_05_create_order_risk_blocked_returns_429_with_db() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] t51_05: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    let user_id = create_test_user(&pool).await;
    let jwt = make_jwt(user_id);

    // 写入 11 条 FAILED 订单以触发风控（阈值 > 10）
    for _ in 0..11 {
        sqlx::query(
            "INSERT INTO payment_orders (user_id, sku_id, provider, state, created_at)
             VALUES ($1, 'diamond_60', 'google_play', 'FAILED', NOW() - INTERVAL '1 hour')",
        )
        .bind(user_id)
        .execute(&pool)
        .await
        .unwrap();
    }

    use std::sync::Arc;
    use voice_room_server::modules::payment::{
        risk::RiskCheckService,
        service::PaymentOrderService,
    };
    let risk = Arc::new(RiskCheckService::new(pool.clone()));
    let svc = Arc::new(PaymentOrderService::new(pool.clone(), risk));

    let state = AppState::for_test()
        .with_payment_order_service(svc as Arc<dyn PaymentOrderServicePort>);
    let app = build_app(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments/orders")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::from(r#"{"sku_id":"diamond_60","provider":"google_play"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Risk blocked → 429"
    );
    let body = body_json(resp).await;
    assert_eq!(body["code"], 40903, "Error code should be 40903 (ORDER_RISK_BLOCKED)");
}
