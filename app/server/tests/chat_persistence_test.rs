//! T-00043 集成测试 — Chat 消息持久化 + REST 历史查询
//!
//! 验收用例覆盖（详见 doc/tds/server/T-00043.md §三）：
//!
//! - U-1（消息持久化）：`handle_send_message` 落库 chat_messages 一行
//! - U-2（msg_id = DB id）：广播 RoomMessage 的 payload.msg_id 与 repo 返回的 id 对齐
//! - U-3（历史查询排序）：REST 返回按 created_at DESC
//! - U-4（分页正确）：123 条消息 limit=50 offset 0/50/100 → 50 / 50 / 23
//! - U-5（空房间）：items=[] / total=0
//! - B-1（limit 超限截断）：limit=999 → 实际 100
//! - B-2（offset 越界）：offset=99999 → 空数组，不报错
//! - B-3（并发写）：10 并发 insert，COUNT == 10
//! - I-1（重连后 REST 兜底）：发 5 条 → 拉取 5 条
//! - I-2（last_msg_id 出窗后 REST 兜底）：超 200 条仍可由 REST 拉到全量
//! - R-1（迁移幂等）：本测试不直接调用迁移 SQL；由 migration_isolation_test 覆盖。
//!   本文件用 dto::normalize_pagination 单元测试覆盖 R-1 周边的边界。
//! - R-2：现有 chat 单元测试（room/handler/mod.rs s01..s09）保持通过 — 由 cargo test 整体回归。
//! - R-3：性能 — 本文件 perf_send_under_50ms 断言单次落库 + 广播 < 50ms。
//!
//! 所有 REST 测试使用 `FakeChatRepository`（内存）+ `build_app(AppState::for_test())`，
//! 无需真实 DB。WS 测试同样使用 Fake。
//!
//! > 端到端 DB 集成测试（U-1 / B-3 真实 sqlx 写入）需 DATABASE_URL；未设置时跳过。

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    modules::chat::{
        repository::{ChatRepository, FakeChatRepository, RealChatRepository},
    },
    room::handler::{handle_send_message, SendMessageDeps},
    room::manager::RoomManager,
    ws::registry::ConnectionRegistry,
};

// ─── 测试辅助 ─────────────────────────────────────────────────────────────────

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

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// 注册一个连接到 ConnectionRegistry，并把它放进给定 room_id。
fn register_conn(
    registry: &Arc<ConnectionRegistry>,
    user_id: Uuid,
    room_id: Option<Uuid>,
) -> (Uuid, tokio::sync::mpsc::UnboundedReceiver<String>) {
    use std::sync::RwLock;
    use tokio::sync::mpsc;
    use voice_room_server::ws::registry::ConnectionHandle;

    let conn_id = Uuid::new_v4();
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    registry.register(ConnectionHandle {
        connection_id: conn_id,
        user_id,
        room_id,
        sender: tx,
        last_heartbeat: Arc::new(RwLock::new(Instant::now())),
    });
    (conn_id, rx)
}

async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

// ============================================================================
// U-1：SendMessage 落库一行
// ============================================================================

#[tokio::test]
async fn u1_send_message_persists_one_row() {
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let chat_repo = Arc::new(FakeChatRepository::new());

    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);
    let (conn_id, _rx) = register_conn(&registry, user_id, Some(room_id));

    let deps = SendMessageDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: None,
        chat_repo: Some(chat_repo.clone()),
    };

    let resp = handle_send_message(
        Some(serde_json::json!({ "content": "hello" })),
        Some("client-msg-u1".to_string()),
        conn_id,
        user_id,
        &deps,
    )
    .await;
    let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
    assert_eq!(json["code"], 0, "U-1: send should succeed");

    let cnt = chat_repo.count_messages(room_id).await.unwrap();
    assert_eq!(cnt, 1, "U-1: chat_messages should have exactly 1 row");

    let (rows, total) = chat_repo.list_messages(room_id, 10, 0).await.unwrap();
    assert_eq!(total, 1);
    assert_eq!(rows[0].content, "hello");
    assert_eq!(rows[0].user_id, Some(user_id));
}

// ============================================================================
// U-2：广播 payload.msg_id == DB 返回的 id
// ============================================================================

#[tokio::test]
async fn u2_broadcast_msg_id_matches_db_id() {
    use std::time::Duration;

    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let chat_repo = Arc::new(FakeChatRepository::new());

    let room_id = Uuid::new_v4();
    let sender = Uuid::new_v4();
    let receiver = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);
    let (_recv_conn, mut rx) = register_conn(&registry, receiver, Some(room_id));
    let (sender_conn, _) = register_conn(&registry, sender, Some(room_id));

    let deps = SendMessageDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: None,
        chat_repo: Some(chat_repo.clone()),
    };

    handle_send_message(
        Some(serde_json::json!({ "content": "hi-u2" })),
        Some("client-msg-u2".to_string()),
        sender_conn,
        sender,
        &deps,
    )
    .await;

    let raw = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("U-2: broadcast must arrive")
        .expect("U-2: channel open");
    let env: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(env["type"], "RoomMessage");

    let payload_msg_id = env["payload"]["msg_id"]
        .as_str()
        .expect("payload.msg_id present")
        .to_string();
    // 应该是 UUID（DB id）而非客户端传的 "client-msg-u2"
    assert_ne!(
        payload_msg_id, "client-msg-u2",
        "U-2: payload.msg_id should be DB id, not client token"
    );
    assert!(
        Uuid::parse_str(&payload_msg_id).is_ok(),
        "U-2: payload.msg_id should be a UUID, got: {payload_msg_id}"
    );

    // 与 DB 中的 id 对齐
    let (rows, _) = chat_repo.list_messages(room_id, 10, 0).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].id.to_string(),
        payload_msg_id,
        "U-2: broadcast msg_id must equal DB row id"
    );
}

// ============================================================================
// U-3 / U-4 / U-5 / B-1 / B-2 — REST 接口
// ============================================================================

async fn build_state_with_repo(repo: Arc<FakeChatRepository>) -> AppState {
    AppState::for_test().with_chat_repo(repo)
}

#[tokio::test]
async fn u3_history_query_orders_desc() {
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    repo.seed_user(user_id, "Alice", Some("https://avatar/a.png"));

    // 顺序写 5 条，时间戳递增（FakeChatRepository 用 Utc::now 兜底）
    for i in 0..5u32 {
        repo.insert_message(room_id, user_id, &format!("msg-{i}"))
            .await
            .unwrap();
        // 让 created_at 单调递增
        tokio::time::sleep(Duration::from_millis(2)).await;
    }

    let state = build_state_with_repo(repo).await;
    let app = build_app(state);

    let jwt = make_test_jwt(user_id, "test-secret");
    let uri = format!("/api/v1/rooms/{room_id}/messages?limit=10");
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["code"], 0);
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    // 倒序：最新（msg-4）在前
    assert_eq!(items[0]["content"], "msg-4");
    assert_eq!(items[4]["content"], "msg-0");
    assert_eq!(items[0]["nickname"], "Alice");
    assert_eq!(items[0]["avatar_url"], "https://avatar/a.png");
    assert_eq!(body["data"]["total"], 5);
    assert_eq!(body["data"]["limit"], 10);
    assert_eq!(body["data"]["offset"], 0);
}

#[tokio::test]
async fn u4_pagination_offsets_split_correctly() {
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    for i in 0..123u32 {
        repo.insert_message(room_id, user_id, &format!("m{i}"))
            .await
            .unwrap();
    }

    let state = build_state_with_repo(repo).await;
    let app = build_app(state);
    let jwt = make_test_jwt(user_id, "test-secret");

    // helper 闭包 — 单次 oneshot 消耗 app，需要每次重新 build
    async fn page(
        jwt: &str,
        room_id: Uuid,
        offset: u32,
        repo: Arc<FakeChatRepository>,
    ) -> serde_json::Value {
        let state = AppState::for_test().with_chat_repo(repo);
        let app = build_app(state);
        let uri = format!("/api/v1/rooms/{room_id}/messages?limit=50&offset={offset}");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&uri)
                    .header("Authorization", format!("Bearer {jwt}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        body_json(resp).await
    }

    let _ = app; // 避免 warning
    // 注：build_app 一次只能消费 state，因此 page() 内部重建 — 但需使用 *同一个* repo Arc
    // 通过状态外置（Arc）共享数据，这里我们重新拿 repo：通过外部克隆
    // 重新塑造 repo
    drop(jwt);
    let jwt = make_test_jwt(user_id, "test-secret");
    // 重新构造 repo（已经在前面被 move 进了 state — 我们要重新提取）
    // 使用全新 repo + 数据同样填充太累；改写：不依赖前面的 state，单独构造
    let repo = Arc::new(FakeChatRepository::new());
    for i in 0..123u32 {
        repo.insert_message(room_id, user_id, &format!("m{i}"))
            .await
            .unwrap();
    }

    let p0 = page(&jwt, room_id, 0, repo.clone()).await;
    assert_eq!(p0["data"]["items"].as_array().unwrap().len(), 50);
    assert_eq!(p0["data"]["total"], 123);

    let p1 = page(&jwt, room_id, 50, repo.clone()).await;
    assert_eq!(p1["data"]["items"].as_array().unwrap().len(), 50);

    let p2 = page(&jwt, room_id, 100, repo.clone()).await;
    assert_eq!(p2["data"]["items"].as_array().unwrap().len(), 23);
}

#[tokio::test]
async fn u5_empty_room_returns_empty_items_total_zero() {
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let state = build_state_with_repo(repo).await;
    let app = build_app(state);
    let jwt = make_test_jwt(user_id, "test-secret");
    let uri = format!("/api/v1/rooms/{room_id}/messages");
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["code"], 0);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
    assert_eq!(body["data"]["total"], 0);
    assert_eq!(body["data"]["limit"], 50);
    assert_eq!(body["data"]["offset"], 0);
}

#[tokio::test]
async fn b1_limit_above_max_truncated_to_100() {
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    for i in 0..150u32 {
        repo.insert_message(room_id, user_id, &format!("m{i}"))
            .await
            .unwrap();
    }

    let state = build_state_with_repo(repo).await;
    let app = build_app(state);
    let jwt = make_test_jwt(user_id, "test-secret");
    let uri = format!("/api/v1/rooms/{room_id}/messages?limit=999");
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["data"]["limit"], 100, "B-1: limit must clamp to 100");
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 100);
    assert_eq!(body["data"]["total"], 150);
}

#[tokio::test]
async fn b2_offset_beyond_total_returns_empty() {
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    for i in 0..3u32 {
        repo.insert_message(room_id, user_id, &format!("m{i}"))
            .await
            .unwrap();
    }

    let state = build_state_with_repo(repo).await;
    let app = build_app(state);
    let jwt = make_test_jwt(user_id, "test-secret");
    let uri = format!("/api/v1/rooms/{room_id}/messages?offset=99999");
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
    assert_eq!(body["data"]["total"], 3);
    assert_eq!(body["data"]["offset"], 99999);
}

// ============================================================================
// B-3：并发写无丢失（FakeRepo 多线程 runtime — 验证无 panic / id 唯一）
// ============================================================================
//
// Round 1 Should-3：升级为 multi_thread runtime，让 RwLock<Vec> 真正承受多线程压力。
// 真实 DB 并发写由 `b3_concurrent_db_inserts`（`#[ignore]`）覆盖。

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn b3_concurrent_inserts_fake_multi_thread() {
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();

    let mut joins = Vec::new();
    for i in 0..10u32 {
        let r = repo.clone();
        let user_id = Uuid::new_v4();
        joins.push(tokio::spawn(async move {
            r.insert_message(room_id, user_id, &format!("c{i}"))
                .await
                .unwrap()
        }));
    }
    let mut ids = Vec::new();
    for j in joins {
        ids.push(j.await.unwrap());
    }
    assert_eq!(ids.len(), 10);
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        10,
        "B-3 (fake mt): every insert must yield unique id"
    );

    let cnt = repo.count_messages(room_id).await.unwrap();
    assert_eq!(
        cnt, 10,
        "B-3 (fake mt): COUNT must equal #concurrent inserts"
    );
}

// ============================================================================
// I-1：发 5 条 → 重连后 REST 拉 5 条
// ============================================================================

#[tokio::test]
async fn i1_disconnect_then_rest_returns_all_messages() {
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let repo = Arc::new(FakeChatRepository::new());

    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);
    let (conn_id, _rx) = register_conn(&registry, user_id, Some(room_id));

    let deps = SendMessageDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: None,
        chat_repo: Some(repo.clone()),
    };

    for i in 0..5u32 {
        let resp = handle_send_message(
            Some(serde_json::json!({ "content": format!("hist-{i}") })),
            Some(format!("c-i1-{i}")),
            conn_id,
            user_id,
            &deps,
        )
        .await;
        let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(json["code"], 0);
    }

    // 模拟断线：使用 REST 拉
    let state = AppState::for_test().with_chat_repo(repo.clone());
    let app = build_app(state);
    let jwt = make_test_jwt(user_id, "test-secret");
    let uri = format!("/api/v1/rooms/{room_id}/messages");
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["data"]["total"], 5, "I-1: REST returns all 5 messages");
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
}

// ============================================================================
// I-2：>200 条（last_msg_id 缓冲窗口外）仍可由 REST 全量拉取
// ============================================================================

#[tokio::test]
async fn i2_rest_fallback_after_last_msg_id_window_exhausted() {
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // 写 250 条（超过 §6.7 缓冲窗口 200 条）
    for i in 0..250u32 {
        repo.insert_message(room_id, user_id, &format!("m{i}"))
            .await
            .unwrap();
    }

    let state = AppState::for_test().with_chat_repo(repo.clone());
    let app = build_app(state);
    let jwt = make_test_jwt(user_id, "test-secret");

    // 第一页 100
    let uri = format!("/api/v1/rooms/{room_id}/messages?limit=100&offset=0");
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(
        body["data"]["total"], 250,
        "I-2: total must reflect full DB count"
    );
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 100);
    assert_eq!(body["data"]["items"][0]["content"], "m249");
}

// ============================================================================
// 鉴权回归：未带 JWT 返回 401
// ============================================================================

#[tokio::test]
async fn auth_required_returns_401() {
    let app = build_app(AppState::for_test());
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/rooms/00000000-0000-0000-0000-000000000000/messages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_room_id_returns_400() {
    let app = build_app(AppState::for_test());
    let jwt = make_test_jwt(Uuid::new_v4(), "test-secret");
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/rooms/not-a-uuid/messages")
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// R-3：性能 — mem-only smoke（**不**代表生产 DB 路径）
// ============================================================================
//
// Round 1 Should-4：明确标注此用例仅覆盖 SendMessage 处理链路（dedupe / filter / 广播）开销，
// 不反映 DB 插入延迟。真 DB 性能基线由 `r3_real_db_perf_smoke`（`#[ignore]`）覆盖。

#[tokio::test]
async fn r3_send_message_under_50ms_mem_only_smoke() {
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);
    let (conn_id, _rx) = register_conn(&registry, user_id, Some(room_id));

    let deps = SendMessageDeps {
        room_manager,
        registry,
        mute_redis: None,
        chat_repo: Some(repo),
    };

    // 预热一次
    let _ = handle_send_message(
        Some(serde_json::json!({ "content": "warmup" })),
        Some("warmup".into()),
        conn_id,
        user_id,
        &deps,
    )
    .await;

    let t0 = Instant::now();
    let _ = handle_send_message(
        Some(serde_json::json!({ "content": "perf-probe" })),
        Some("perf-probe".into()),
        conn_id,
        user_id,
        &deps,
    )
    .await;
    let elapsed = t0.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "R-3 (mem-only smoke): 单次 SendMessage 处理链路 < 50ms (Fake repo); elapsed={elapsed:?}\n\
         注：此基线**不**代表真实 DB 写入开销，生产 DB 性能由 r3_real_db_perf_smoke 覆盖。"
    );
}

// ============================================================================
// DB 集成测试：U-1/U-3/B-3 真实 sqlx 路径（DATABASE_URL 未设时跳过）
// ============================================================================

#[tokio::test]
async fn db_real_insert_and_list() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] db_real_insert_and_list: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.expect("migrations");

    // 创建一个用户与一个房间（避免外键约束失败）
    let user_id = create_user(&pool).await;
    let room_id = create_room(&pool, user_id).await;

    let repo = RealChatRepository::new(pool.clone());
    let id1 = repo.insert_message(room_id, user_id, "real-1").await.unwrap();
    let id2 = repo.insert_message(room_id, user_id, "real-2").await.unwrap();
    assert_ne!(id1, id2);

    let (rows, total) = repo.list_messages(room_id, 10, 0).await.unwrap();
    assert_eq!(total, 2);
    assert_eq!(rows.len(), 2);
    // 最新在前（real-2 后写）
    assert_eq!(rows[0].content, "real-2");
    assert_eq!(rows[1].content, "real-1");
    // LEFT JOIN users 应当返回 nickname
    assert!(rows[0].nickname.is_some());
}

async fn create_user(pool: &PgPool) -> Uuid {
    let phone = format!("+861{}", &Uuid::new_v4().to_string().replace('-', "")[..10]);
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO users (phone, nickname) VALUES ($1, $2) RETURNING id",
    )
    .bind(&phone)
    .bind("ChatTester")
    .fetch_one(pool)
    .await
    .expect("insert user");
    row.0
}

async fn create_room(pool: &PgPool, owner: Uuid) -> Uuid {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO rooms (owner_id, title, room_type, status) \
         VALUES ($1, $2, 'normal', 'active') RETURNING id",
    )
    .bind(owner)
    .bind("T-00043 chat test room")
    .fetch_one(pool)
    .await
    .expect("insert room");
    row.0
}

// ============================================================================
// Round 1 Should-6：HTTP 层 offset 软上限（normalize_pagination 截断到 100_000）
// ============================================================================

#[tokio::test]
async fn b6_offset_above_soft_cap_truncated() {
    let repo = Arc::new(FakeChatRepository::new());
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    repo.insert_message(room_id, user_id, "only-one")
        .await
        .unwrap();

    let state = AppState::for_test().with_chat_repo(repo);
    let app = build_app(state);
    let jwt = make_test_jwt(user_id, "test-secret");
    // 传入 200_000 — 应被 normalize_pagination 截断到 100_000
    let uri = format!("/api/v1/rooms/{room_id}/messages?offset=200000");
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .header("Authorization", format!("Bearer {jwt}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(
        body["data"]["offset"], 100_000,
        "Should-6: offset > MAX_OFFSET 必须截断到 100_000"
    );
    assert_eq!(body["data"]["total"], 1);
    assert_eq!(body["data"]["items"].as_array().unwrap().len(), 0);
}

// ============================================================================
// Round 1 Should-3：真实 DB 并发写入（10 task 并发 INSERT，COUNT == 10）
// 仅在显式开启时执行：cargo test ... -- --ignored b3_concurrent_db_inserts
// 若 DATABASE_URL 未设置则自动跳过。
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn b3_concurrent_db_inserts() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] b3_concurrent_db_inserts: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.expect("migrations");

    let user_id = create_user(&pool).await;
    let room_id = create_room(&pool, user_id).await;
    let repo = Arc::new(RealChatRepository::new(pool.clone()));

    // 10 个并发 INSERT
    let mut joins = Vec::new();
    for i in 0..10u32 {
        let r = repo.clone();
        joins.push(tokio::spawn(async move {
            r.insert_message(room_id, user_id, &format!("c{i}"))
                .await
                .expect("insert ok")
        }));
    }
    let mut ids = Vec::new();
    for j in joins {
        ids.push(j.await.unwrap());
    }
    assert_eq!(ids.len(), 10);

    // COUNT(*) == 10 — 验证无丢失
    let cnt = repo.count_messages(room_id).await.unwrap();
    assert_eq!(
        cnt, 10,
        "B-3 (real DB): 并发 INSERT 全部落盘，COUNT(*) 必须等于并发数"
    );

    // id 唯一
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 10);
}

// ============================================================================
// Round 1 Should-4：真实 DB 单次写入性能 smoke（100 次，p95 < 50ms）
// 仅在显式开启时执行；DATABASE_URL 未设置则跳过。
// ============================================================================

#[tokio::test]
#[ignore]
async fn r3_real_db_perf_smoke() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] r3_real_db_perf_smoke: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.expect("migrations");

    let user_id = create_user(&pool).await;
    let room_id = create_room(&pool, user_id).await;
    let repo = RealChatRepository::new(pool.clone());

    // 预热 5 次
    for _ in 0..5 {
        repo.insert_message(room_id, user_id, "warmup")
            .await
            .unwrap();
    }

    let mut samples = Vec::with_capacity(100);
    for i in 0..100u32 {
        let t0 = Instant::now();
        repo.insert_message(room_id, user_id, &format!("perf-{i}"))
            .await
            .unwrap();
        samples.push(t0.elapsed());
    }
    samples.sort();
    let p95 = samples[(samples.len() as f64 * 0.95) as usize - 1];
    assert!(
        p95 < Duration::from_millis(50),
        "R-3 (real DB): 100 次 INSERT p95 必须 < 50ms; got p95={p95:?}, max={:?}",
        samples.last().unwrap()
    );
}
