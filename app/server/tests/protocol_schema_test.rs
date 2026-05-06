//! T-00103 protocol schema integration tests
//!
//! ## Test cases
//! - PS-01 ~ PS-08: 抽样 8 条出栈 envelope，断言 schema validate 通过
//! - DENY-1:  Request DTO deny_unknown_fields → 反序列化返回错误
//! - PING-COMPAT: server 在 dev 模式下同时处理 Ping (新) 与 ping (旧)
//!
//! 所有测试纯内存，不依赖数据库或 Redis。

use std::sync::{Arc, RwLock};
use std::time::Instant;

use jsonschema::JSONSchema;
use serde_json::{json, Value};
use uuid::Uuid;

// ─── 辅助：从嵌入的 schema 字符串构建验证器 ──────────────────────────────────

/// 从 schema JSON 字符串构建 jsonschema 验证器（panic on malformed schema）
fn compile_schema(schema_str: &str) -> JSONSchema {
    let schema: Value = serde_json::from_str(schema_str).expect("schema must be valid JSON");
    JSONSchema::compile(&schema).expect("schema must compile")
}

/// 断言 instance 对 schema_str 有效，否则 panic 并打印差异
fn assert_valid(schema_str: &str, instance: &Value, label: &str) {
    let validator = compile_schema(schema_str);
    let result = validator.validate(instance);
    if let Err(errs) = result {
        let msgs: Vec<String> = errs.map(|e| e.to_string()).collect();
        panic!("{label} failed schema validation:\n  instance: {instance}\n  errors: {msgs:?}");
    }
}

// ─── 嵌入 schema 文件 ─────────────────────────────────────────────────────────

macro_rules! ws_schema {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../doc/protocol/schemas/ws/",
            $name,
            ".schema.json"
        ))
    };
}

// ─── PS-01: MicTaken 广播 envelope 符合 schema ────────────────────────────────

/// PS-01: S→Room MicTaken broadcast envelope
#[test]
fn ps01_mic_taken_envelope_valid() {
    let schema_str = ws_schema!("MicTaken");
    let user_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "MicTaken",
        "msg_id": msg_id,
        "payload": {
            "mic_index": 2,
            "user_id": user_id,
            "nickname": "Alice",
            "avatar": null
        },
        "timestamp": 1_700_000_000i64
    });

    assert_valid(schema_str, &envelope, "PS-01: MicTaken");
}

// ─── PS-02: MicLeft 广播 envelope 符合 schema ─────────────────────────────────

/// PS-02: S→Room MicLeft broadcast envelope
#[test]
fn ps02_mic_left_envelope_valid() {
    let schema_str = ws_schema!("MicLeft");
    let user_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "MicLeft",
        "msg_id": msg_id,
        "payload": {
            "mic_index": 1,
            "user_id": user_id,
            "forced": false
        },
        "timestamp": 1_700_000_001i64
    });

    assert_valid(schema_str, &envelope, "PS-02: MicLeft");
}

// ─── PS-03: UserJoined 广播 envelope 符合 schema ──────────────────────────────

/// PS-03: S→Room UserJoined broadcast envelope
#[test]
fn ps03_user_joined_envelope_valid() {
    let schema_str = ws_schema!("UserJoined");
    let user_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "UserJoined",
        "msg_id": msg_id,
        "payload": {
            "user_id": user_id,
            "nickname": "Bob",
            "avatar": null,
            "member_count": 5
        },
        "timestamp": 1_700_000_002i64
    });

    assert_valid(schema_str, &envelope, "PS-03: UserJoined");
}

// ─── PS-04: UserLeft 广播 envelope 符合 schema ────────────────────────────────

/// PS-04: S→Room UserLeft broadcast envelope
#[test]
fn ps04_user_left_envelope_valid() {
    let schema_str = ws_schema!("UserLeft");
    let user_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "UserLeft",
        "msg_id": msg_id,
        "payload": {
            "user_id": user_id,
            "nickname": "Bob",
            "member_count": 4
        },
        "timestamp": 1_700_000_003i64
    });

    assert_valid(schema_str, &envelope, "PS-04: UserLeft");
}

// ─── PS-05: RoomMessage 广播 envelope 符合 schema ────────────────────────────

/// PS-05: S→Room RoomMessage broadcast envelope
#[test]
fn ps05_room_message_envelope_valid() {
    let schema_str = ws_schema!("RoomMessage");
    let user_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();
    let payload_msg_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "RoomMessage",
        "msg_id": msg_id,
        "payload": {
            "msg_id": payload_msg_id,
            "user_id": user_id,
            "nickname": "Charlie",
            "avatar": null,
            "content": "Hello everyone!"
        },
        "timestamp": 1_700_000_004i64
    });

    assert_valid(schema_str, &envelope, "PS-05: RoomMessage");
}

// ─── PS-06: JoinRoomResult envelope 符合 schema ───────────────────────────────

/// PS-06: S→C JoinRoomResult success envelope
#[test]
fn ps06_join_room_result_envelope_valid() {
    let schema_str = ws_schema!("JoinRoomResult");
    let msg_id = Uuid::new_v4().to_string();
    let room_id = Uuid::new_v4().to_string();
    let owner_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "JoinRoomResult",
        "msg_id": msg_id,
        "code": 0,
        "payload": {
            "room": {
                "room_id": room_id,
                "title": "Test Room",
                "owner_id": owner_id,
                "member_count": 1,
                "mic_slots": [null, null, null, null, null, null, null, null, null]
            },
            "mic_slot": null
        },
        "timestamp": 1_700_000_005i64
    });

    assert_valid(schema_str, &envelope, "PS-06: JoinRoomResult");
}

// ─── PS-07: TakeMicResult envelope 符合 schema ───────────────────────────────

/// PS-07: S→C TakeMicResult success envelope
#[test]
fn ps07_take_mic_result_envelope_valid() {
    let schema_str = ws_schema!("TakeMicResult");
    let msg_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "TakeMicResult",
        "msg_id": msg_id,
        "code": 0,
        "payload": {
            "mic_index": 3
        },
        "timestamp": 1_700_000_006i64
    });

    assert_valid(schema_str, &envelope, "PS-07: TakeMicResult");
}

// ─── PS-08: SendGiftResult envelope 符合 schema ──────────────────────────────

/// PS-08: S→C SendGiftResult success envelope
#[test]
fn ps08_send_gift_result_envelope_valid() {
    let schema_str = ws_schema!("SendGiftResult");
    let msg_id = Uuid::new_v4().to_string();
    let gift_record_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "SendGiftResult",
        "msg_id": msg_id,
        "code": 0,
        "payload": {
            "gift_record_id": gift_record_id,
            "total_price": 100
        },
        "timestamp": 1_700_000_007i64
    });

    assert_valid(schema_str, &envelope, "PS-08: SendGiftResult");
}

// ─── DENY-1: HTTP Request DTO deny_unknown_fields ────────────────────────────

/// DENY-1: SendGiftRequest 含未知字段 → 反序列化失败
///
/// 此测试在 deny_unknown_fields 添加之前会失败（RED），添加后通过（GREEN）。
#[test]
fn deny1_send_gift_request_rejects_unknown_fields() {
    let bad_json = r#"{
        "room_id": "00000000-0000-0000-0000-000000000001",
        "gift_id":  "00000000-0000-0000-0000-000000000002",
        "receiver_id": "00000000-0000-0000-0000-000000000003",
        "count": 1,
        "hacked_field": "i should not be here"
    }"#;

    let result =
        serde_json::from_str::<voice_room_server::modules::gift::dto::SendGiftRequest>(bad_json);

    assert!(
        result.is_err(),
        "DENY-1: SendGiftRequest must reject unknown fields, \
         but deserialization succeeded. Add #[serde(deny_unknown_fields)]."
    );
}

/// DENY-1b: CreateRoomRequest 含未知字段 → 反序列化失败
#[test]
fn deny1b_create_room_request_rejects_unknown_fields() {
    let bad_json = r#"{
        "title": "Test Room",
        "room_type": "public",
        "injected_admin": true
    }"#;

    let result =
        serde_json::from_str::<voice_room_server::modules::room::dto::CreateRoomRequest>(bad_json);

    assert!(
        result.is_err(),
        "DENY-1b: CreateRoomRequest must reject unknown fields. \
         Add #[serde(deny_unknown_fields)]."
    );
}

/// DENY-1c: WS IncomingMessage 含未知字段 → 反序列化失败
#[test]
fn deny1c_incoming_message_rejects_unknown_fields() {
    use voice_room_server::ws::connection::IncomingMessage;

    let bad_json = r#"{
        "type": "ping",
        "msg_id": "test-id",
        "secret_backdoor": "injected"
    }"#;

    let result = serde_json::from_str::<IncomingMessage>(bad_json);

    assert!(
        result.is_err(),
        "DENY-1c: IncomingMessage must reject unknown fields. \
         Add #[serde(deny_unknown_fields)]."
    );
}

// ─── PING-COMPAT: server 同时处理 Ping (新) 与 ping (旧) ────────────────────

/// PING-COMPAT-1: handle_text_message 对大写 Ping 信令返回 Pong 响应
///
/// 此测试在 Ping 处理器添加之前会失败（RED），添加后通过（GREEN）。
#[test]
fn ping_compat_1_uppercase_ping_returns_pong() {
    use voice_room_server::ws::connection::handle_text_message;

    let hb = Arc::new(RwLock::new(Instant::now()));
    let msg_id = Uuid::new_v4().to_string();
    let ping_json = format!(r#"{{"type":"Ping","msg_id":"{msg_id}"}}"#);

    let response = handle_text_message(&ping_json, &hb);

    assert!(
        response.is_some(),
        "PING-COMPAT-1: Ping (uppercase) must return a Pong response, got None"
    );

    let resp_value: Value =
        serde_json::from_str(&response.unwrap()).expect("response must be valid JSON");

    assert_eq!(
        resp_value["type"], "Pong",
        "PING-COMPAT-1: uppercase Ping must return type=Pong (uppercase)"
    );
}

/// PING-COMPAT-2: dev 模式下 ping_pong_responses 双发 Pong + pong
///
/// 此测试验证兼容期行为：dev 模式服务端同时含新格式 Pong 和旧格式 pong。
#[cfg(debug_assertions)]
#[test]
fn ping_compat_2_dev_mode_double_sends() {
    use voice_room_server::ws::connection::ping_pong_responses;

    let msg_id = Some(Uuid::new_v4().to_string());
    let responses = ping_pong_responses(msg_id);

    assert_eq!(
        responses.len(),
        2,
        "PING-COMPAT-2: dev mode must double-send [Pong (new), pong (legacy)]"
    );

    let types: Vec<String> = responses
        .iter()
        .map(|s| {
            let v: Value = serde_json::from_str(s).expect("each response must be valid JSON");
            v["type"].as_str().expect("type must be string").to_owned()
        })
        .collect();

    assert!(
        types.contains(&"Pong".to_owned()),
        "PING-COMPAT-2: responses must contain new format Pong, got: {types:?}"
    );
    assert!(
        types.contains(&"pong".to_owned()),
        "PING-COMPAT-2: responses must contain legacy pong for compat period, got: {types:?}"
    );
}

/// PING-COMPAT-3: Pong envelope 符合 Pong schema（新格式）
#[test]
fn ping_compat_3_pong_envelope_matches_schema() {
    let schema_str = ws_schema!("Pong");
    let msg_id = Uuid::new_v4().to_string();

    let envelope = json!({
        "type": "Pong",
        "msg_id": msg_id,
        "timestamp": 1_700_000_010i64
    });

    assert_valid(schema_str, &envelope, "PING-COMPAT-3: Pong schema");
}

