//! 集成测试 — T-00018 余额查询 API + WS 推送
//!
//! 测试用例 B01~B09 验证以下内容：
//! - B01: 未登录访问 /wallet/balance 返回 401
//! - B02: 已登录初始用户返回 diamond_balance=0
//! - B03: /wallet/transactions 空流水返回 total=0, items=[]
//! - B04: 按 type=gift_send 过滤只返回对应类型
//! - B05: apply_delta 成功后 500ms 内同会话收到 BalanceUpdated
//! - B06: 同一 user 多连接时全部收到推送
//! - B07: BalanceBroadcaster.broadcast_event 正确推送 BalanceUpdated 到用户 WS
//! - B08: apply_delta 使 balance < 0 时整体事务回滚，无流水写入，无 WS 推送
//! - B09: page=0 / size=200 返回 40003
//!
//! 运行前提：DATABASE_URL 环境变量指向可用的 PostgreSQL 实例（B02~B06、B08 需要）。
//! 若未设置 DATABASE_URL，DB 相关测试将被跳过。

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tokio::sync::mpsc;
use tower::ServiceExt;
use uuid::Uuid;
use voice_room_shared::models::wallet::WalletTxnType;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    modules::wallet::{
        broadcaster::{BalanceBroadcaster, BalanceEvent},
        service::WalletService,
    },
    ws::registry::{ConnectionHandle, ConnectionRegistry},
};

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

/// 获取测试用连接池；未配置 DATABASE_URL 或连接失败时返回 None（测试跳过）
async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

/// 在 DB 中创建测试用户，返回 user_id
async fn create_test_user(pool: &PgPool) -> Uuid {
    let phone = format!("+861{}", &Uuid::new_v4().to_string().replace('-', "")[..10]);
    let row = sqlx::query("INSERT INTO users (phone, nickname) VALUES ($1, $2) RETURNING id")
        .bind(&phone)
        .bind("TestWalletUser")
        .fetch_one(pool)
        .await
        .expect("create test user");
    row.get("id")
}

/// 生成测试用 JWT token
fn make_test_jwt(user_id: Uuid, secret: &str) -> String {
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
    encode_token(&claims, secret.as_bytes()).expect("encode JWT")
}

/// 将响应 body 读取为 JSON
async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// 创建用于测试的 WS 连接 handle，返回 (handle, rx)
fn make_ws_handle(user_id: Uuid) -> (ConnectionHandle, mpsc::UnboundedReceiver<String>) {
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let handle = ConnectionHandle {
        connection_id: Uuid::new_v4(),
        user_id,
        room_id: None,
        sender: tx,
        last_heartbeat: Arc::new(RwLock::new(Instant::now())),
    };
    (handle, rx)
}

// ─── B01: 未登录访问 /wallet/balance 返回 401 ────────────────────────────────

#[tokio::test]
async fn b01_unauthenticated_balance_returns_401() {
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

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Unauthenticated balance request should return 401"
    );

    let body = body_json(response).await;
    assert_eq!(body["code"], 40101, "Error code should be 40101 (Unauthorized)");
}

// ─── B02: 已登录初始用户返回 diamond_balance=0 ────────────────────────────────

#[tokio::test]
async fn b02_authenticated_user_initial_balance_is_zero() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] b02: DATABASE_URL not set");
        return;
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let user_id = create_test_user(&pool).await;
    let jwt = make_test_jwt(user_id, "test-secret");

    // 构建带真实 WalletService 的 App
    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(10);
    let wallet_service = Arc::new(WalletService::new(pool.clone(), balance_tx));
    let state = AppState::for_test_with_wallet(wallet_service);
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/wallet/balance")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["code"], 0);
    assert_eq!(
        body["data"]["diamond_balance"], 0,
        "New user should have diamond_balance=0"
    );
}

// ─── B03: /wallet/transactions 空流水返回 total=0, items=[] ──────────────────

#[tokio::test]
async fn b03_empty_transactions_returns_zero() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] b03: DATABASE_URL not set");
        return;
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let user_id = create_test_user(&pool).await;
    let jwt = make_test_jwt(user_id, "test-secret");

    let (balance_tx, _balance_rx) = mpsc::channel::<BalanceEvent>(10);
    let wallet_service = Arc::new(WalletService::new(pool.clone(), balance_tx));
    let state = AppState::for_test_with_wallet(wallet_service);
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/wallet/transactions?page=1&size=20")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["total"], 0, "Empty wallet should have total=0");
    assert_eq!(
        body["data"]["items"].as_array().unwrap().len(),
        0,
        "Empty wallet should have items=[]"
    );
}

// ─── B04: 按 type=gift_send 过滤只返回对应类型 ───────────────────────────────

#[tokio::test]
async fn b04_filter_by_type_returns_only_matching() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] b04: DATABASE_URL not set");
        return;
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let user_id = create_test_user(&pool).await;

    // 先充值 1000，再送礼 -100
    let (balance_tx, _rx) = mpsc::channel::<BalanceEvent>(10);
    let wallet_service = WalletService::new(pool.clone(), balance_tx);

    wallet_service
        .apply_delta(user_id, 1000, WalletTxnType::AdminAdjust, None, None, None)
        .await
        .expect("apply recharge delta");

    wallet_service
        .apply_delta(user_id, -100, WalletTxnType::GiftSend, None, None, None)
        .await
        .expect("apply gift_send delta");

    // 构建 App 查询
    let jwt = make_test_jwt(user_id, "test-secret");
    let (balance_tx2, _rx2) = mpsc::channel::<BalanceEvent>(10);
    let wallet_service2 = Arc::new(WalletService::new(pool.clone(), balance_tx2));
    let state = AppState::for_test_with_wallet(wallet_service2);
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/wallet/transactions?page=1&size=20&type=gift_send")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["total"], 1, "Should have 1 gift_send transaction");
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["type"], "gift_send", "Transaction type should be gift_send");
    assert_eq!(items[0]["amount"], -100, "Amount should be -100");
}

// ─── B05: apply_delta 成功后 500ms 内同会话收到 BalanceUpdated ────────────────

#[tokio::test]
async fn b05_apply_delta_triggers_ws_balance_updated_within_500ms() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] b05: DATABASE_URL not set");
        return;
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let user_id = create_test_user(&pool).await;

    // 创建 channel 和 WalletService
    let (balance_tx, balance_rx) = mpsc::channel::<BalanceEvent>(10);
    let wallet_service = WalletService::new(pool.clone(), balance_tx);

    // 创建 ConnectionRegistry 并注册用户连接
    let registry = Arc::new(ConnectionRegistry::new());
    let (handle, mut ws_rx) = make_ws_handle(user_id);
    registry.register(handle);

    // 启动 BalanceBroadcaster
    let broadcaster = BalanceBroadcaster::new(registry.clone());
    tokio::spawn(broadcaster.run(balance_rx));

    // 先给用户充值，确保有余额
    wallet_service
        .apply_delta(user_id, 1000, WalletTxnType::AdminAdjust, None, None, None)
        .await
        .expect("initial recharge");

    // 消耗掉充值的 WS 通知
    let _ = tokio::time::timeout(Duration::from_millis(200), ws_rx.recv()).await;

    // 执行扣款 delta
    wallet_service
        .apply_delta(user_id, -100, WalletTxnType::GiftSend, None, None, None)
        .await
        .expect("apply delta");

    // 期望在 500ms 内收到 BalanceUpdated
    let msg = tokio::time::timeout(Duration::from_millis(500), ws_rx.recv())
        .await
        .expect("Should receive BalanceUpdated within 500ms")
        .expect("WS channel should not be closed");

    let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(value["type"], "BalanceUpdated", "Message type should be BalanceUpdated");
    assert_eq!(
        value["payload"]["diamond_balance"], 900,
        "Balance after should be 900"
    );
    assert_eq!(value["payload"]["delta"], -100, "Delta should be -100");
    assert_eq!(
        value["payload"]["reason"], "gift_send",
        "Reason should be gift_send"
    );
}

// ─── B06: 同一 user 多连接时全部收到推送 ────────────────────────────────────

#[tokio::test]
async fn b06_multi_connection_same_user_all_receive_push() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] b06: DATABASE_URL not set");
        return;
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let user_id = create_test_user(&pool).await;

    let (balance_tx, balance_rx) = mpsc::channel::<BalanceEvent>(10);
    let wallet_service = WalletService::new(pool.clone(), balance_tx);

    // 注册同一用户的 2 个 WS 连接
    let registry = Arc::new(ConnectionRegistry::new());
    let (handle1, mut rx1) = make_ws_handle(user_id);
    let (handle2, mut rx2) = make_ws_handle(user_id);
    registry.register(handle1);
    registry.register(handle2);

    // 启动广播器
    let broadcaster = BalanceBroadcaster::new(registry.clone());
    tokio::spawn(broadcaster.run(balance_rx));

    // 充值
    wallet_service
        .apply_delta(user_id, 500, WalletTxnType::AdminAdjust, None, None, None)
        .await
        .expect("apply delta");

    // 两个连接都应该在 500ms 内收到通知
    let msg1 = tokio::time::timeout(Duration::from_millis(500), rx1.recv())
        .await
        .expect("conn1 should receive within 500ms")
        .expect("conn1 channel closed");

    let msg2 = tokio::time::timeout(Duration::from_millis(500), rx2.recv())
        .await
        .expect("conn2 should receive within 500ms")
        .expect("conn2 channel closed");

    let v1: serde_json::Value = serde_json::from_str(&msg1).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&msg2).unwrap();

    assert_eq!(v1["type"], "BalanceUpdated", "conn1 should receive BalanceUpdated");
    assert_eq!(v2["type"], "BalanceUpdated", "conn2 should receive BalanceUpdated");
    assert_eq!(v1["payload"]["diamond_balance"], 500, "conn1 balance should be 500");
    assert_eq!(v2["payload"]["diamond_balance"], 500, "conn2 balance should be 500");
}

// ─── B07: BalanceBroadcaster 直接推送（模拟 Redis balance_updated 事件路径）────

#[tokio::test]
async fn b07_balance_broadcaster_pushes_ws_on_event() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user_id = Uuid::new_v4();

    let (handle, mut rx) = make_ws_handle(user_id);
    registry.register(handle);

    let broadcaster = BalanceBroadcaster::new(registry);

    // 模拟 Redis balance_updated 事件触发 broadcaster
    broadcaster.broadcast_event(&BalanceEvent {
        user_id,
        balance_after: 1234,
        delta: 100,
        reason: "admin_adjust".to_string(),
        ref_id: None,
    });

    let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("Should receive BalanceUpdated immediately")
        .expect("Channel should not be closed");

    let value: serde_json::Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(value["type"], "BalanceUpdated");
    assert_eq!(value["payload"]["diamond_balance"], 1234);
    assert_eq!(value["payload"]["delta"], 100);
    assert_eq!(value["payload"]["reason"], "admin_adjust");
}

// ─── B08: apply_delta 使 balance < 0 时事务回滚，无流水写入，无 WS 推送 ─────

#[tokio::test]
async fn b08_apply_delta_negative_balance_rolls_back() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] b08: DATABASE_URL not set");
        return;
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    let user_id = create_test_user(&pool).await;

    let (balance_tx, balance_rx) = mpsc::channel::<BalanceEvent>(10);
    let wallet_service = WalletService::new(pool.clone(), balance_tx);

    // 注册 WS 连接以监听是否收到 push
    let registry = Arc::new(ConnectionRegistry::new());
    let (handle, mut ws_rx) = make_ws_handle(user_id);
    registry.register(handle);

    let broadcaster = BalanceBroadcaster::new(registry.clone());
    tokio::spawn(broadcaster.run(balance_rx));

    // 用户初始余额为 0，尝试扣款 -100 应失败
    let result = wallet_service
        .apply_delta(user_id, -100, WalletTxnType::GiftSend, None, None, None)
        .await;

    assert!(result.is_err(), "Applying negative delta should fail");

    // 验证余额未变
    let balance: i64 = sqlx::query_scalar(
        "SELECT diamond_balance FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(balance, 0, "Balance should remain 0 after failed delta");

    // 验证无流水记录
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM wallet_transactions WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0, "No transaction should be written after rollback");

    // 验证无 WS 推送（50ms 内不应收到任何消息）
    let no_push = tokio::time::timeout(Duration::from_millis(50), ws_rx.recv()).await;
    assert!(
        no_push.is_err(),
        "No WS BalanceUpdated should be sent after rollback"
    );
}

// ─── B09: page=0 / size=200 返回 40003 ──────────────────────────────────────

#[tokio::test]
async fn b09_invalid_pagination_returns_40003() {
    let jwt = make_test_jwt(Uuid::new_v4(), "test-secret");

    // page=0 应返回 40003
    {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/wallet/transactions?page=0&size=20")
                    .header("Authorization", format!("Bearer {jwt}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "page=0 should return 400");
        let body = body_json(response).await;
        assert_eq!(body["code"], 40003, "page=0 should return error code 40003");
    }

    // size=200 应返回 40003
    {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/wallet/transactions?page=1&size=200")
                    .header("Authorization", format!("Bearer {jwt}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "size=200 should return 400");
        let body = body_json(response).await;
        assert_eq!(body["code"], 40003, "size=200 should return error code 40003");
    }
}
