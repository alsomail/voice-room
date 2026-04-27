//! 集成测试 — T-00044 HTTP 礼物发送 POST /api/v1/gifts/send
//!
//! 测试用例验证以下内容：
//! - SH01 (U-1): HTTP 送礼成功 → 返回 gift_record_id, sender_balance, receiver_charm
//! - SH02 (U-3): 余额不足 → 400 + 40290 INSUFFICIENT_BALANCE
//! - SH03: count=0 → 400 + 40001 INVALID_COUNT
//! - SH04: count=10000 → 400 + 40001 INVALID_COUNT
//! - SH05: 未鉴权 → 401
//! - SH06: 礼物下架 → 404 + 40402 GIFT_NOT_AVAILABLE
//! - SH07: 接收者不在麦上 → 404 + 40403 RECEIVER_UNAVAILABLE

mod common;

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    modules::gift::send_gift::{GiftSendService},
    modules::wallet::broadcaster::BalanceEvent,
    room::{manager::RoomManager, state::MemberInfo},
    ws::registry::ConnectionRegistry,
};

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 获取测试用数据库连接池；未配置 DATABASE_URL 或连接失败时返回 None（测试跳过）
async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

/// 获取 Redis URL（可选，缺失时相关 Redis 断言被跳过）
fn redis_url() -> Option<String> {
    std::env::var("REDIS_URL").ok()
}

/// 在数据库中插入测试用户，设置初始 diamond_balance
async fn insert_test_user(pool: &PgPool, balance: i64) -> Uuid {
    let user_id = Uuid::new_v4();
    let phone = format!("+86{}", &user_id.to_string().replace('-', "")[..11]);
    sqlx::query(
        "INSERT INTO users (id, phone, nickname, diamond_balance) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(&phone)
    .bind(format!("TestUser_{}", &user_id.to_string()[..8]))
    .bind(balance)
    .execute(pool)
    .await
    .expect("insert test user");
    user_id
}

/// 在数据库中插入测试房间
async fn insert_test_room(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let room_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO rooms (id, owner_id, title, status) \
         VALUES ($1, $2, $3, 'active')",
    )
    .bind(room_id)
    .bind(owner_id)
    .bind("Test Room")
    .execute(pool)
    .await
    .expect("insert test room");
    room_id
}

/// 获取 gifts 表中最便宜的活跃礼物（迁移后有种子数据）
async fn get_active_gift(pool: &PgPool) -> (Uuid, i64) {
    let row = sqlx::query(
        "SELECT id, price FROM gifts WHERE is_active = true ORDER BY price ASC LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("get active gift");
    (row.get("id"), row.get("price"))
}

/// 查询用户当前 diamond_balance
async fn get_diamond_balance(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT diamond_balance FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("get diamond balance")
}

/// 查询用户当前 charm_balance
async fn get_charm_balance(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT charm_balance FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("get charm balance")
}

/// 生成测试用 JWT（简化版，使用硬编码 secret）
fn generate_test_jwt(user_id: Uuid) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    struct Claims {
        sub: String,
        iss: String,
        exp: u64,
        iat: u64,
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let claims = Claims {
        sub: user_id.to_string(),
        iss: "voiceroom".to_string(),
        exp: now + 3600,
        iat: now,
    };
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "test-secret".to_string());
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

/// 插入一个下架的测试礼物
async fn insert_inactive_gift(pool: &PgPool) -> Uuid {
    let gift_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO gifts \
         (id, code, name_en, name_ar, icon_url, price, tier, effect_level, sort_order, is_active, is_deleted) \
         VALUES ($1, $2, 'Inactive Gift', 'هدية غير نشطة', '/test.png', 10, 1, 1, 999, false, false)",
    )
    .bind(gift_id)
    .bind(format!("inactive_{}", &gift_id.to_string()[..8]))
    .execute(pool)
    .await
    .expect("insert inactive gift");
    gift_id
}

// ─── 测试用例 ──────────────────────────────────────────────────────────────────

/// SH01 (U-1): HTTP 送礼成功 → 返回 gift_record_id, sender_balance, receiver_charm
#[tokio::test]
async fn sh01_http_send_gift_success() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("SH01: DATABASE_URL not configured, skipping");
            return;
        }
    };
    let redis_url = match redis_url() {
        Some(url) => url,
        None => {
            eprintln!("SH01: REDIS_URL not configured, skipping");
            return;
        }
    };

    common::run_migrations(&pool).await.expect("migrate");

    // 1. 准备测试数据
    let sender = insert_test_user(&pool, 1000).await;
    let receiver = insert_test_user(&pool, 0).await;
    let owner = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner).await;
    let (gift_id, price) = get_active_gift(&pool).await;

    // 2. 设置房间状态
    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);
    room_state.members.insert(
        sender,
        MemberInfo::new(sender, format!("Sender_{}", &sender.to_string()[..8]), None),
    );
    room_state.members.insert(
        receiver,
        MemberInfo::new(receiver, format!("Receiver_{}", &receiver.to_string()[..8]), None),
    );
    // 将 receiver 放到 0 号麦位
    room_state
        .take_mic_slot(0, receiver)
        .expect("take mic slot");

    // 3. 构建服务
    let registry = Arc::new(ConnectionRegistry::new());
    let (balance_tx, _balance_rx) = tokio::sync::mpsc::channel::<BalanceEvent>(100);
    let send_gift_service = Arc::new(
        GiftSendService::new(
            pool.clone(),
            registry.clone(),
            room_manager.clone(),
            balance_tx,
            redis_url,
        )
        .expect("create GiftSendService"),
    );

    // 4. 创建 AppState
    let mut state = AppState::for_test();
    state.send_gift_service = send_gift_service;
    state.room_manager = room_manager;
    state.ws_registry = registry;

    // 5. 构建应用
    let app = build_app(state);

    // 6. 发送 HTTP 请求
    let jwt = generate_test_jwt(sender);
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/gifts/send")
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "room_id": room_id.to_string(),
                "gift_id": gift_id.to_string(),
                "receiver_id": receiver.to_string(),
                "count": 1
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // 7. 验证响应
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "SH01: should return 200 OK"
    );

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    
    eprintln!("SH01: response body = {}", serde_json::to_string_pretty(&body).unwrap());
    
    assert_eq!(body["code"], 0, "SH01: code should be 0");
    assert!(
        body["data"]["gift_record_id"].is_string(),
        "SH01: should return gift_record_id"
    );
    assert_eq!(
        body["data"]["sender_balance"], 1000 - price,
        "SH01: sender_balance should be reduced"
    );
    assert_eq!(
        body["data"]["receiver_charm"], price,
        "SH01: receiver_charm should increase"
    );

    // 8. 验证数据库状态
    let sender_balance = get_diamond_balance(&pool, sender).await;
    assert_eq!(
        sender_balance,
        1000 - price,
        "SH01: DB sender balance should match"
    );
    let receiver_charm = get_charm_balance(&pool, receiver).await;
    assert_eq!(
        receiver_charm, price,
        "SH01: DB receiver charm should match"
    );
}

/// SH02 (U-3): 余额不足 → 400 + 40290 INSUFFICIENT_BALANCE
#[tokio::test]
async fn sh02_insufficient_balance() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("SH02: DATABASE_URL not configured, skipping");
            return;
        }
    };
    let redis_url = match redis_url() {
        Some(url) => url,
        None => {
            eprintln!("SH02: REDIS_URL not configured, skipping");
            return;
        }
    };

    common::run_migrations(&pool).await.expect("migrate");

    let sender = insert_test_user(&pool, 5).await;
    let receiver = insert_test_user(&pool, 0).await;
    let owner = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner).await;
    let (gift_id, _price) = get_active_gift(&pool).await;

    let sender_balance_before = get_diamond_balance(&pool, sender).await;

    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);
    room_state.members.insert(
        sender,
        MemberInfo::new(sender, "Sender".to_string(), None),
    );
    room_state.members.insert(
        receiver,
        MemberInfo::new(receiver, "Receiver".to_string(), None),
    );
    room_state.take_mic_slot(0, receiver).expect("take mic");

    let registry = Arc::new(ConnectionRegistry::new());
    let (balance_tx, _balance_rx) = tokio::sync::mpsc::channel::<BalanceEvent>(100);
    let send_gift_service = Arc::new(
        GiftSendService::new(pool.clone(), registry.clone(), room_manager.clone(), balance_tx, redis_url)
            .expect("create service"),
    );

    let mut state = AppState::for_test();
    state.send_gift_service = send_gift_service;
    state.room_manager = room_manager;
    state.ws_registry = registry;

    let app = build_app(state);

    let jwt = generate_test_jwt(sender);
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/gifts/send")
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "room_id": room_id.to_string(),
                "gift_id": gift_id.to_string(),
                "receiver_id": receiver.to_string(),
                "count": 100
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "SH02: should return 400"
    );
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], 40290, "SH02: code should be 40290 INSUFFICIENT_BALANCE");

    let sender_balance_after = get_diamond_balance(&pool, sender).await;
    assert_eq!(
        sender_balance_after, sender_balance_before,
        "SH02: balance should not change"
    );
}

/// SH03: count=0 → 400 + 40001 INVALID_COUNT
#[tokio::test]
async fn sh03_invalid_count_zero() {
    let state = AppState::for_test();
    let app = build_app(state);

    let jwt = generate_test_jwt(Uuid::new_v4());
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/gifts/send")
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "room_id": Uuid::new_v4().to_string(),
                "gift_id": Uuid::new_v4().to_string(),
                "receiver_id": Uuid::new_v4().to_string(),
                "count": 0
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "SH03: should return 400"
    );
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], 40001, "SH03: code should be 40001 INVALID_COUNT");
}

/// SH04: count=10000 → 400 + 40001 INVALID_COUNT
#[tokio::test]
async fn sh04_invalid_count_overflow() {
    let state = AppState::for_test();
    let app = build_app(state);

    let jwt = generate_test_jwt(Uuid::new_v4());
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/gifts/send")
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "room_id": Uuid::new_v4().to_string(),
                "gift_id": Uuid::new_v4().to_string(),
                "receiver_id": Uuid::new_v4().to_string(),
                "count": 10000
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "SH04: should return 400"
    );
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], 40001, "SH04: code should be 40001 INVALID_COUNT");
}

/// SH05: 未鉴权 → 401
#[tokio::test]
async fn sh05_unauthorized() {
    let state = AppState::for_test();
    let app = build_app(state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/gifts/send")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "room_id": Uuid::new_v4().to_string(),
                "gift_id": Uuid::new_v4().to_string(),
                "receiver_id": Uuid::new_v4().to_string(),
                "count": 1
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "SH05: should return 401"
    );
}

/// SH06: 礼物下架 → 404 + 40402 GIFT_NOT_AVAILABLE
#[tokio::test]
async fn sh06_gift_not_available() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("SH06: DATABASE_URL not configured, skipping");
            return;
        }
    };
    let redis_url = match redis_url() {
        Some(url) => url,
        None => {
            eprintln!("SH06: REDIS_URL not configured, skipping");
            return;
        }
    };

    common::run_migrations(&pool).await.expect("migrate");

    let sender = insert_test_user(&pool, 1000).await;
    let receiver = insert_test_user(&pool, 0).await;
    let owner = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner).await;
    let inactive_gift_id = insert_inactive_gift(&pool).await;

    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);
    room_state.members.insert(sender, MemberInfo::new(sender, "Sender".to_string(), None));
    room_state.members.insert(receiver, MemberInfo::new(receiver, "Receiver".to_string(), None));
    room_state.take_mic_slot(0, receiver).expect("take mic");

    let registry = Arc::new(ConnectionRegistry::new());
    let (balance_tx, _) = tokio::sync::mpsc::channel::<BalanceEvent>(100);
    let send_gift_service = Arc::new(
        GiftSendService::new(pool.clone(), registry.clone(), room_manager.clone(), balance_tx, redis_url)
            .expect("create service"),
    );

    let mut state = AppState::for_test();
    state.send_gift_service = send_gift_service;
    state.room_manager = room_manager;
    state.ws_registry = registry;

    let app = build_app(state);

    let jwt = generate_test_jwt(sender);
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/gifts/send")
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "room_id": room_id.to_string(),
                "gift_id": inactive_gift_id.to_string(),
                "receiver_id": receiver.to_string(),
                "count": 1
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND, "SH06: should return 404");
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], 40402, "SH06: code should be 40402 GIFT_NOT_AVAILABLE");
}

/// SH07: 接收者不在麦上 → 404 + 40403 RECEIVER_UNAVAILABLE
#[tokio::test]
async fn sh07_receiver_not_on_mic() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("SH07: DATABASE_URL not configured, skipping");
            return;
        }
    };
    let redis_url = match redis_url() {
        Some(url) => url,
        None => {
            eprintln!("SH07: REDIS_URL not configured, skipping");
            return;
        }
    };

    common::run_migrations(&pool).await.expect("migrate");

    let sender = insert_test_user(&pool, 1000).await;
    let receiver = insert_test_user(&pool, 0).await;
    let owner = insert_test_user(&pool, 0).await;
    let room_id = insert_test_room(&pool, owner).await;
    let (gift_id, _) = get_active_gift(&pool).await;

    let room_manager = Arc::new(RoomManager::new());
    let room_state = room_manager.get_or_create_room(room_id);
    room_state.members.insert(sender, MemberInfo::new(sender, "Sender".to_string(), None));
    room_state.members.insert(receiver, MemberInfo::new(receiver, "Receiver".to_string(), None));
    // 注意：receiver 不在麦上

    let registry = Arc::new(ConnectionRegistry::new());
    let (balance_tx, _) = tokio::sync::mpsc::channel::<BalanceEvent>(100);
    let send_gift_service = Arc::new(
        GiftSendService::new(pool.clone(), registry.clone(), room_manager.clone(), balance_tx, redis_url)
            .expect("create service"),
    );

    let mut state = AppState::for_test();
    state.send_gift_service = send_gift_service;
    state.room_manager = room_manager;
    state.ws_registry = registry;

    let app = build_app(state);

    let jwt = generate_test_jwt(sender);
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/gifts/send")
        .header(header::AUTHORIZATION, format!("Bearer {}", jwt))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "room_id": room_id.to_string(),
                "gift_id": gift_id.to_string(),
                "receiver_id": receiver.to_string(),
                "count": 1
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND, "SH07: should return 404");
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], 40403, "SH07: code should be 40403 RECEIVER_UNAVAILABLE");
}
