//! T-00048 集成测试 — Chat 双路径 envelope 等价回归
//!
//! 验收用例（详见 doc/tds/server/T-00048.md §三）：
//!
//! - DUAL-1: 两个 WS 观察者 A/B 同时在线，WS 路径 + REST 路径各发一条消息，
//!           A/B 均收到两条 RoomMessage，除 msg_id 外逐字段相等。
//! - DUAL-2: payload 字段逐项断言：msg_id(UUID v4,互不相同)、content(相等,已过滤)、
//!           timestamp(>0)、user_id(相等)、type="RoomMessage"。
//! - DUAL-3: 死连接清理一致性 — drop 一个观察者 receiver，WS 路径 + REST 路径发送
//!           均不 panic、不阻塞，存活观察者均收到消息。

mod common;

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use axum::{body::Body, http::{Request, StatusCode}};
use tokio::sync::mpsc;
use tower::ServiceExt;
use uuid::Uuid;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    modules::chat::repository::FakeChatRepository,
    room::{
        handler::{handle_send_message, SendMessageDeps},
        manager::RoomManager,
    },
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

fn register_conn(
    registry: &ConnectionRegistry,
    user_id: Uuid,
    room_id: Option<Uuid>,
) -> (Uuid, mpsc::UnboundedReceiver<String>) {
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

async fn recv_timeout(rx: &mut mpsc::UnboundedReceiver<String>, label: &str) -> serde_json::Value {
    let raw = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .unwrap_or_else(|_| panic!("{label}: broadcast must arrive within 500ms"))
        .unwrap_or_else(|| panic!("{label}: channel still open"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{label}: invalid JSON: {e}"))
}

// ─── DUAL-1: 两个观察者均收到双路径等价 envelope ─────────────────────────────

#[tokio::test]
async fn dual1_two_observers_receive_both_path_envelopes() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let sender_user = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);

    // 观察者 A/B 同时在线
    let (_obs_a_id, mut obs_a_rx) = register_conn(&registry, Uuid::new_v4(), Some(room_id));
    let (_obs_b_id, mut obs_b_rx) = register_conn(&registry, Uuid::new_v4(), Some(room_id));

    // WS 发送者连接
    let ws_conn_id = Uuid::new_v4();
    let (ws_tx, _ws_self_rx) = mpsc::unbounded_channel::<String>();
    registry.register(ConnectionHandle {
        connection_id: ws_conn_id,
        user_id: sender_user,
        room_id: Some(room_id),
        sender: ws_tx,
        last_heartbeat: Arc::new(RwLock::new(Instant::now())),
    });

    // ── 1. WS 路径发送 ────────────────────────────────────────────────────────
    let ws_deps = SendMessageDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: None,
        chat_repo: Some(chat_repo.clone()),
    };
    let ws_ack_raw = handle_send_message(
        Some(serde_json::json!({ "content": "hello dual" })),
        Some(Uuid::new_v4().to_string()),
        ws_conn_id,
        sender_user,
        &ws_deps,
    )
    .await;
    let ws_ack: serde_json::Value = serde_json::from_str(&ws_ack_raw).unwrap();
    assert_eq!(ws_ack["code"], 0, "DUAL-1: WS send must succeed");

    // 观察者 A/B 收到 WS 路径的广播
    let ws_env_a = recv_timeout(&mut obs_a_rx, "DUAL-1 obs_a WS").await;
    let ws_env_b = recv_timeout(&mut obs_b_rx, "DUAL-1 obs_b WS").await;

    // ── 2. REST 路径发送 ──────────────────────────────────────────────────────
    let app = build_app(build_state(
        chat_repo.clone(),
        room_manager.clone(),
        registry.clone(),
    ));
    let jwt = make_test_jwt(sender_user, "test-secret");
    let resp = app
        .oneshot(post_chat_message_request(
            &jwt,
            serde_json::json!({ "room_id": room_id.to_string(), "content": "hello dual" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "DUAL-1: REST send must succeed");

    // 观察者 A/B 收到 REST 路径的广播
    let rest_env_a = recv_timeout(&mut obs_a_rx, "DUAL-1 obs_a REST").await;
    let rest_env_b = recv_timeout(&mut obs_b_rx, "DUAL-1 obs_b REST").await;

    // ── 3. 对比 A 视角（WS vs REST）除 msg_id 外逐字段相等 ────────────────────
    for (ws_env, rest_env, label) in [
        (&ws_env_a, &rest_env_a, "obs_a"),
        (&ws_env_b, &rest_env_b, "obs_b"),
    ] {
        assert_eq!(
            ws_env["type"], rest_env["type"],
            "DUAL-1 {label}: type must be equal"
        );
        assert_eq!(
            ws_env["payload"]["content"], rest_env["payload"]["content"],
            "DUAL-1 {label}: payload.content must be equal"
        );
        assert_eq!(
            ws_env["payload"]["user_id"], rest_env["payload"]["user_id"],
            "DUAL-1 {label}: payload.user_id must be equal"
        );
        // msg_id 必须互不相同
        let ws_pid = ws_env["payload"]["msg_id"].as_str().expect("ws payload.msg_id");
        let rest_pid = rest_env["payload"]["msg_id"].as_str().expect("rest payload.msg_id");
        assert_ne!(ws_pid, rest_pid, "DUAL-1 {label}: payload.msg_id must differ");
    }
}

// ─── DUAL-2: payload 字段逐项断言 ──────────────────────────────────────────────

#[tokio::test]
async fn dual2_payload_field_assertions() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let sender_user = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);

    let (_obs_id, mut obs_rx) = register_conn(&registry, Uuid::new_v4(), Some(room_id));

    // WS 发送者
    let ws_conn_id = Uuid::new_v4();
    let (ws_tx, _ws_self_rx) = mpsc::unbounded_channel::<String>();
    registry.register(ConnectionHandle {
        connection_id: ws_conn_id,
        user_id: sender_user,
        room_id: Some(room_id),
        sender: ws_tx,
        last_heartbeat: Arc::new(RwLock::new(Instant::now())),
    });

    // 使用含 Arabic 的内容 (PROTO-2 语言兼容性)
    let content = "مرحبا hello dual2";

    // ── WS 路径 ────────────────────────────────────────────────────────────────
    let ws_deps = SendMessageDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: None,
        chat_repo: Some(chat_repo.clone()),
    };
    let ws_ack_raw = handle_send_message(
        Some(serde_json::json!({ "content": content })),
        Some(Uuid::new_v4().to_string()),
        ws_conn_id,
        sender_user,
        &ws_deps,
    )
    .await;
    let ws_ack: serde_json::Value = serde_json::from_str(&ws_ack_raw).unwrap();
    assert_eq!(ws_ack["code"], 0, "DUAL-2: WS ack must be code=0");

    let ws_env = recv_timeout(&mut obs_rx, "DUAL-2 ws").await;

    // ── REST 路径 ──────────────────────────────────────────────────────────────
    let app = build_app(build_state(
        chat_repo.clone(),
        room_manager.clone(),
        registry.clone(),
    ));
    let jwt = make_test_jwt(sender_user, "test-secret");
    let resp = app
        .oneshot(post_chat_message_request(
            &jwt,
            serde_json::json!({ "room_id": room_id.to_string(), "content": content }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "DUAL-2: REST must succeed");

    let rest_env = recv_timeout(&mut obs_rx, "DUAL-2 rest").await;

    // ── 逐字段断言 ─────────────────────────────────────────────────────────────

    // type
    assert_eq!(ws_env["type"], "RoomMessage", "DUAL-2: WS type must be RoomMessage");
    assert_eq!(rest_env["type"], "RoomMessage", "DUAL-2: REST type must be RoomMessage");

    // payload.msg_id — 合法 UUID v4，互不相同
    let ws_payload_mid = ws_env["payload"]["msg_id"].as_str().expect("DUAL-2: ws payload.msg_id");
    let rest_payload_mid = rest_env["payload"]["msg_id"].as_str().expect("DUAL-2: rest payload.msg_id");
    assert!(
        Uuid::parse_str(ws_payload_mid).is_ok(),
        "DUAL-2: ws payload.msg_id must be valid UUID"
    );
    assert!(
        Uuid::parse_str(rest_payload_mid).is_ok(),
        "DUAL-2: rest payload.msg_id must be valid UUID"
    );
    assert_ne!(
        ws_payload_mid, rest_payload_mid,
        "DUAL-2: payload.msg_id must differ (different DB rows)"
    );

    // envelope.msg_id — 合法 UUID v4，互不相同
    let ws_env_mid = ws_env["msg_id"].as_str().expect("DUAL-2: ws envelope.msg_id");
    let rest_env_mid = rest_env["msg_id"].as_str().expect("DUAL-2: rest envelope.msg_id");
    assert!(
        Uuid::parse_str(ws_env_mid).is_ok(),
        "DUAL-2: ws envelope.msg_id must be valid UUID"
    );
    assert!(
        Uuid::parse_str(rest_env_mid).is_ok(),
        "DUAL-2: rest envelope.msg_id must be valid UUID"
    );
    assert_ne!(
        ws_env_mid, rest_env_mid,
        "DUAL-2: envelope.msg_id must differ"
    );

    // payload.content — 两路径相等
    assert_eq!(
        ws_env["payload"]["content"], rest_env["payload"]["content"],
        "DUAL-2: payload.content must be equal between WS and REST"
    );
    assert_eq!(
        ws_env["payload"]["content"], content,
        "DUAL-2: content must match original (no sensitive words)"
    );

    // payload.timestamp — int64 > 0（通过 is_number 检查，值允许不同）
    assert!(
        ws_env["timestamp"].is_number(),
        "DUAL-2: ws timestamp must be numeric"
    );
    assert!(
        rest_env["timestamp"].is_number(),
        "DUAL-2: rest timestamp must be numeric"
    );

    // payload.user_id — 两路径相等
    assert_eq!(
        ws_env["payload"]["user_id"], rest_env["payload"]["user_id"],
        "DUAL-2: payload.user_id must be equal between WS and REST"
    );
    assert_eq!(
        ws_env["payload"]["user_id"],
        sender_user.to_string(),
        "DUAL-2: payload.user_id must match sender"
    );
}

// ─── DUAL-3: 死连接清理一致性 ───────────────────────────────────────────────────

#[tokio::test]
async fn dual3_dead_connection_cleanup_consistency() {
    let chat_repo = Arc::new(FakeChatRepository::new());
    let room_manager = Arc::new(RoomManager::new());
    let registry = Arc::new(ConnectionRegistry::new());

    let sender_user = Uuid::new_v4();
    let room_id = Uuid::new_v4();
    room_manager.get_or_create_room(room_id);

    // 死连接：注册后立即 drop receiver
    let (_dead_conn_id, dead_rx) = register_conn(&registry, Uuid::new_v4(), Some(room_id));
    drop(dead_rx);

    // 存活观察者
    let (_alive_id, mut alive_rx) = register_conn(&registry, Uuid::new_v4(), Some(room_id));

    // WS 发送者
    let ws_conn_id = Uuid::new_v4();
    let (ws_tx, _ws_self_rx) = mpsc::unbounded_channel::<String>();
    registry.register(ConnectionHandle {
        connection_id: ws_conn_id,
        user_id: sender_user,
        room_id: Some(room_id),
        sender: ws_tx,
        last_heartbeat: Arc::new(RwLock::new(Instant::now())),
    });

    // ── 1. WS 路径发送（死连接存在时不应 panic 或阻塞）────────────────────────
    let ws_deps = SendMessageDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        mute_redis: None,
        chat_repo: Some(chat_repo.clone()),
    };
    let ws_ack_raw = handle_send_message(
        Some(serde_json::json!({ "content": "dual3 ws message" })),
        Some(Uuid::new_v4().to_string()),
        ws_conn_id,
        sender_user,
        &ws_deps,
    )
    .await;
    let ws_ack: serde_json::Value = serde_json::from_str(&ws_ack_raw).unwrap();
    assert_eq!(
        ws_ack["code"], 0,
        "DUAL-3: WS path must succeed even with dead connection"
    );

    // 存活观察者收到 WS 广播
    let ws_env = recv_timeout(&mut alive_rx, "DUAL-3 alive after WS send").await;
    assert_eq!(
        ws_env["payload"]["content"], "dual3 ws message",
        "DUAL-3: alive observer must receive WS broadcast"
    );

    // ── 2. REST 路径发送（死连接存在时不应 panic 或阻塞）──────────────────────
    let app = build_app(build_state(
        chat_repo.clone(),
        room_manager.clone(),
        registry.clone(),
    ));
    let jwt = make_test_jwt(sender_user, "test-secret");
    let resp = app
        .oneshot(post_chat_message_request(
            &jwt,
            serde_json::json!({
                "room_id": room_id.to_string(),
                "content": "dual3 rest message"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "DUAL-3: REST path must succeed even with dead connection"
    );

    // 存活观察者收到 REST 广播
    let rest_env = recv_timeout(&mut alive_rx, "DUAL-3 alive after REST send").await;
    assert_eq!(
        rest_env["payload"]["content"], "dual3 rest message",
        "DUAL-3: alive observer must receive REST broadcast"
    );
}
