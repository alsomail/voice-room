//! T-00045 集成测试 — Chat REST POST 广播（BUG-CHAT-WS-BROADCAST）
//!
//! 验收用例覆盖（详见 doc/tds/server/T-00045.md §三）：
//!
//! - REST-01: POST /api/v1/chat-messages 成功 → 同房间 WS 收到 RoomMessage，
//!            payload.msg_id 为合法 UUID 且与 DB 落库 id 一致。
//! - REST-02: 广播 envelope 顶层包含 type=RoomMessage、msg_id（UUID v4）、timestamp。
//! - REST-03: 不同房间的 WS 连接不应收到该消息。
//! - REST-04: DB 中确实落库一行（content/room_id/user_id 均匹配）。
//! - REST-05: 广播失败容忍 — 一个断开的 sender + 一个健康连接 → REST 仍返回 200/0；
//!            健康连接收到广播。
//! - REST-06: content 为空 / 超长 → 400，且不广播；DB 不落库。
//! - REST-07: room_id 非法 UUID → 400。
//! - REST-08: 未登录（缺失 Authorization）→ 401。
//! - REST-09: 房间未在内存（无 RoomState）→ REST 仍返回 200/0（降级广播分支），DB 仍落库。

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use uuid::Uuid;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    modules::chat::repository::{ChatRepository, FakeChatRepository},
    room::manager::RoomManager,
    ws::registry::{ConnectionHandle, ConnectionRegistry},
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

fn register_conn(
    registry: &ConnectionRegistry,
    user_id: Uuid,
    room_id: Option<Uuid>,
) -> (Uuid, tokio::sync::mpsc::UnboundedReceiver<String>) {
    use std::sync::RwLock;
    use tokio::sync::mpsc;

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

/// 构造一个 AppState：注入指定的 chat_repo / room_manager / registry，
/// 这样测试可以直接观察广播是否进入受测连接。
fn build_state(
    chat_repo: Arc<FakeChatRepository>,
    room_manager: Arc<RoomManager>,
    registry: Arc<ConnectionRegistry>,
) -> AppState {
    let mut state = AppState::for_test().with_chat_repo(chat_repo);
    state.room_manager = room_manager;
    state.ws_registry = registry;
    state
}

fn post_chat_message_request(jwt: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/chat-messages")
        .header("Authorization", format!("Bearer {jwt}"))
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

// ─── REST-01: 房间内连接收到 RoomMessage ──────────────────────────────────────

#[tokio::test]
async fn rest01_post_broadcasts_to_room() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);
    let (_conn, mut rx) = register_conn(&registry, user_id, Some(room_id));

    let app = build_app(build_state(chat_repo.clone(), room_manager.clone(), registry.clone()));
    let jwt = make_test_jwt(user_id, "test-secret");

    let resp = app
        .oneshot(post_chat_message_request(
            &jwt,
            serde_json::json!({ "room_id": room_id.to_string(), "content": "hello world" }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["code"], 0, "REST-01: code=0 expected");

    // 接收 WS 广播
    let raw = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("REST-01: broadcast must arrive")
        .expect("REST-01: channel still open");
    let env: serde_json::Value = serde_json::from_str(&raw).unwrap();

    assert_eq!(env["type"], "RoomMessage");
    assert_eq!(env["payload"]["content"], "hello world");
    assert_eq!(env["payload"]["user_id"], user_id.to_string());

    let payload_msg_id = env["payload"]["msg_id"].as_str().expect("payload.msg_id");
    assert!(
        Uuid::parse_str(payload_msg_id).is_ok(),
        "REST-01: payload.msg_id must be UUID"
    );

    // payload.msg_id == DB id
    let (rows, _total) = chat_repo.list_messages(room_id, 10, 0).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].id.to_string(),
        payload_msg_id,
        "REST-01: payload.msg_id must equal DB row id"
    );
}

// ─── REST-02: envelope 顶层 msg_id (UUID v4) + timestamp + type ──────────────

#[tokio::test]
async fn rest02_envelope_has_msg_id_and_timestamp() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);
    let (_c, mut rx) = register_conn(&registry, user_id, Some(room_id));

    let app = build_app(build_state(chat_repo, room_manager, registry));
    let jwt = make_test_jwt(user_id, "test-secret");

    app.oneshot(post_chat_message_request(
        &jwt,
        serde_json::json!({ "room_id": room_id.to_string(), "content": "envelope-test" }),
    ))
    .await
    .unwrap();

    let raw = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .expect("REST-02 broadcast must arrive")
        .unwrap();
    let env: serde_json::Value = serde_json::from_str(&raw).unwrap();

    assert_eq!(env["type"], "RoomMessage");
    let env_msg_id = env["msg_id"].as_str().expect("envelope msg_id present");
    assert!(
        Uuid::parse_str(env_msg_id).is_ok(),
        "REST-02: envelope-level msg_id must be UUID v4 (injected by broadcaster)"
    );
    assert!(
        env["timestamp"].is_number(),
        "REST-02: envelope must contain numeric timestamp"
    );
}

// ─── REST-03: 其他房间不收到 ──────────────────────────────────────────────────

#[tokio::test]
async fn rest03_other_room_does_not_receive() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    let other_room = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);
    let (_self_conn, _self_rx) = register_conn(&registry, user_id, Some(room_id));
    let (_other_conn, mut other_rx) =
        register_conn(&registry, Uuid::new_v4(), Some(other_room));

    let app = build_app(build_state(chat_repo, room_manager, registry.clone()));
    let jwt = make_test_jwt(user_id, "test-secret");

    app.oneshot(post_chat_message_request(
        &jwt,
        serde_json::json!({ "room_id": room_id.to_string(), "content": "private-room" }),
    ))
    .await
    .unwrap();

    let result = tokio::time::timeout(Duration::from_millis(80), other_rx.recv()).await;
    assert!(
        result.is_err(),
        "REST-03: other room MUST NOT receive RoomMessage"
    );
}

// ─── REST-04: DB 落库 ─────────────────────────────────────────────────────────

#[tokio::test]
async fn rest04_message_persisted_to_db() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);

    let app = build_app(build_state(chat_repo.clone(), room_manager, registry));
    let jwt = make_test_jwt(user_id, "test-secret");

    let resp = app
        .oneshot(post_chat_message_request(
            &jwt,
            serde_json::json!({ "room_id": room_id.to_string(), "content": "persist-me" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let cnt = chat_repo.count_messages(room_id).await.unwrap();
    assert_eq!(cnt, 1);
    let (rows, _) = chat_repo.list_messages(room_id, 10, 0).await.unwrap();
    assert_eq!(rows[0].content, "persist-me");
    assert_eq!(rows[0].user_id, Some(user_id));
}

// ─── REST-05: 一个 sender 已断开 + 一个健康连接 → REST 仍 200，健康连接收到 ─

#[tokio::test]
async fn rest05_broadcast_tolerates_dead_sender() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let sender_user = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);

    // 注册一个"已断开"的连接：注册后立即 drop receiver
    let (_dead_conn_id, dead_rx) = register_conn(&registry, Uuid::new_v4(), Some(room_id));
    drop(dead_rx);

    // 注册一个健康连接
    let (_alive_conn, mut alive_rx) =
        register_conn(&registry, Uuid::new_v4(), Some(room_id));

    let app = build_app(build_state(chat_repo, room_manager, registry));
    let jwt = make_test_jwt(sender_user, "test-secret");

    let resp = app
        .oneshot(post_chat_message_request(
            &jwt,
            serde_json::json!({ "room_id": room_id.to_string(), "content": "tolerant" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["code"], 0, "REST-05: code=0 even if a sender is dead");

    let raw = tokio::time::timeout(Duration::from_millis(500), alive_rx.recv())
        .await
        .expect("REST-05: alive connection must receive broadcast")
        .unwrap();
    let env: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(env["payload"]["content"], "tolerant");
}

// ─── REST-06: content 长度校验 ────────────────────────────────────────────────

#[tokio::test]
async fn rest06_content_validation_empty_or_too_long() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);
    let (_conn, mut rx) = register_conn(&registry, user_id, Some(room_id));

    let jwt = make_test_jwt(user_id, "test-secret");

    // 空 content
    {
        let app = build_app(build_state(
            chat_repo.clone(),
            room_manager.clone(),
            registry.clone(),
        ));
        let resp = app
            .oneshot(post_chat_message_request(
                &jwt,
                serde_json::json!({ "room_id": room_id.to_string(), "content": "" }),
            ))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "REST-06: empty content must be 400"
        );
    }

    // 超长 content（501 字符）
    {
        let app = build_app(build_state(
            chat_repo.clone(),
            room_manager.clone(),
            registry.clone(),
        ));
        let too_long = "a".repeat(501);
        let resp = app
            .oneshot(post_chat_message_request(
                &jwt,
                serde_json::json!({ "room_id": room_id.to_string(), "content": too_long }),
            ))
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "REST-06: >500 chars must be 400"
        );
    }

    // 不广播 + 不入库
    let result = tokio::time::timeout(Duration::from_millis(80), rx.recv()).await;
    assert!(result.is_err(), "REST-06: validation failure must NOT broadcast");
    assert_eq!(
        chat_repo.count_messages(room_id).await.unwrap(),
        0,
        "REST-06: validation failure must NOT persist"
    );
}

// ─── REST-07: room_id 非法 UUID → 400 ─────────────────────────────────────────

#[tokio::test]
async fn rest07_invalid_room_id_returns_400() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let user_id = Uuid::new_v4();
    let app = build_app(build_state(chat_repo, room_manager, registry));
    let jwt = make_test_jwt(user_id, "test-secret");

    let resp = app
        .oneshot(post_chat_message_request(
            &jwt,
            serde_json::json!({ "room_id": "not-a-uuid", "content": "hi" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ─── REST-08: 未登录 → 401 ─────────────────────────────────────────────────────

#[tokio::test]
async fn rest08_missing_auth_returns_401() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());
    let app = build_app(build_state(chat_repo, room_manager, registry));

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/chat-messages")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({ "room_id": Uuid::new_v4().to_string(), "content": "x" })
                .to_string(),
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ─── REST-09: 房间未在内存（无 RoomState）→ REST 仍 200，DB 仍落库 ──────────

#[tokio::test]
async fn rest09_room_not_in_memory_still_succeeds() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new()); // 不预创建 room state
    let registry = Arc::new(ConnectionRegistry::new());

    let user_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();

    let app = build_app(build_state(chat_repo.clone(), room_manager, registry));
    let jwt = make_test_jwt(user_id, "test-secret");

    let resp = app
        .oneshot(post_chat_message_request(
            &jwt,
            serde_json::json!({ "room_id": room_id.to_string(), "content": "no-state" }),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "REST-09: room not in memory must NOT fail"
    );
    let body = body_json(resp).await;
    assert_eq!(body["code"], 0);
    assert_eq!(chat_repo.count_messages(room_id).await.unwrap(), 1);
}
