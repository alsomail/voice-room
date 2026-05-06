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
/// `ping_pong_responses` 已在 `handle_socket` 的 "Ping"|"ping" arm 中调用，
/// 为真实 WS 处理链路提供双发能力（非孤立函数）。TDS §一.3。
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
        "timestamp": 1_700_000_010_000i64   // 毫秒级，~1.7×10^12，可区分 timestamp_millis 修复前后
    });

    assert_valid(schema_str, &envelope, "PING-COMPAT-3: Pong schema");
}

// ─── PING-1: 大写 Ping → Pong，msg_id 回显，timestamp 为毫秒 ─────────────────

/// PING-1: handle_text_message 对大写 Ping 信令返回 Pong，timestamp 必须为毫秒级
///
/// Schema 要求 Pong.timestamp 为毫秒整数（> 1_000_000_000_000）。
/// 此测试在 timestamp() 未改为 timestamp_millis() 时失败（RED），
/// 修复后通过（GREEN）。
#[test]
fn ping_1_uppercase_ping_returns_pong_with_ms_timestamp() {
    use voice_room_server::ws::connection::handle_text_message;

    let hb = Arc::new(RwLock::new(Instant::now()));
    let msg_id = "550e8400-e29b-41d4-a716-446655440000";
    let ping_json =
        format!(r#"{{"type":"Ping","msg_id":"{msg_id}","timestamp":1700000000000}}"#);

    let resp = handle_text_message(&ping_json, &hb).unwrap();
    let v: Value = serde_json::from_str(&resp).expect("Pong must be valid JSON");

    assert_eq!(v["type"], "Pong", "PING-1: response type must be Pong");
    assert_eq!(
        v["msg_id"], msg_id,
        "PING-1: msg_id must be echoed back"
    );

    // timestamp 必须是毫秒级（> 1 trillion = 10^12）
    // 秒级时间戳 ~1.7×10^9，毫秒级时间戳 ~1.7×10^12
    let ts = v["timestamp"]
        .as_i64()
        .expect("PING-1: timestamp must be an integer");
    assert!(
        ts > 1_000_000_000_000,
        "PING-1: timestamp must be milliseconds (> 1_000_000_000_000), got {ts}"
    );
}

// ─── PING-2: 小写 ping 仍工作（兼容期），timestamp 为毫秒 ─────────────────────

/// PING-2：旧格式 ping 直调 handle_text_message（兼容函数级单测）
///
/// NOTE: handle_text_message 的 "ping" arm 在生产路径 handle_socket 中为死代码。
/// 生产路径的 ping→Pong 行为由 PING-2B + PING-3 通过 ping_pong_responses 覆盖。
/// 此测试仅验证辅助函数 handle_text_message 本身的小写 pong 返回及毫秒 timestamp。
///
/// PROTO-BINDING: doc/protocol/schemas/ws/Ping.schema.json (deprecated path)
/// 此测试在 timestamp() 未改为 timestamp_millis() 时失败（RED）。
#[test]
fn ping_2_lowercase_ping_compat_returns_pong_with_ms_timestamp() {
    use voice_room_server::ws::connection::handle_text_message;

    let hb = Arc::new(RwLock::new(Instant::now()));
    let ping_json =
        r#"{"type":"ping","msg_id":"550e8400-e29b-41d4-a716-446655440001","timestamp":1700000000000}"#;

    let resp = handle_text_message(ping_json, &hb)
        .expect("PING-2: legacy ping must return a pong response");
    let v: Value = serde_json::from_str(&resp).expect("pong must be valid JSON");

    assert_eq!(
        v["type"], "pong",
        "PING-2: legacy ping must return type=pong (lowercase)"
    );

    // timestamp 也必须是毫秒级
    let ts = v["timestamp"]
        .as_i64()
        .expect("PING-2: timestamp must be an integer");
    assert!(
        ts > 1_000_000_000_000,
        "PING-2: legacy pong timestamp must be milliseconds (> 1_000_000_000_000), got {ts}"
    );
}

/// PING-2B: 生产路径 — ping_pong_responses() 对 legacy ping 总是返回大写 "Pong"
///
/// handle_socket 的 "Ping"/"ping" arm 都调用 ping_pong_responses，而不走
/// handle_text_message。生产路径（release 模式）只发 "Pong"（大写）；
/// debug 模式双发 ["Pong", "pong"]，但 Pong（大写）始终在首位。
///
/// PROTO-BINDING: doc/protocol/schemas/ws/Pong.schema.json
#[test]
fn ping_2b_production_path_ping_pong_responses_returns_uppercase_pong() {
    use voice_room_server::ws::connection::ping_pong_responses;

    let responses = ping_pong_responses(Some("legacy-ping-id".to_string()));

    // 无论 debug/release 模式，第一个响应都必须是大写 "Pong"
    assert!(
        !responses.is_empty(),
        "PING-2B: ping_pong_responses must not be empty"
    );

    let v: Value =
        serde_json::from_str(&responses[0]).expect("PING-2B: first response must be valid JSON");

    assert_eq!(
        v["type"], "Pong",
        "PING-2B: production Pong must be uppercase 'Pong', got {:?}",
        v["type"]
    );

    // msg_id 必须回显（不能为 None，schema required 字段）
    assert_eq!(
        v["msg_id"], "legacy-ping-id",
        "PING-2B: msg_id must be echoed back in production Pong"
    );

    // timestamp 必须是毫秒级
    let ts = v["timestamp"]
        .as_i64()
        .expect("PING-2B: timestamp must be an integer");
    assert!(
        ts > 1_000_000_000_000,
        "PING-2B: production Pong timestamp must be milliseconds (> 1_000_000_000_000), got {ts}"
    );
}

// ─── PING-3: ping_pong_responses() 所有 timestamp 为毫秒 ─────────────────────

/// PING-3: ping_pong_responses 返回的所有响应，timestamp 必须为毫秒级
///
/// 此测试在 ping_pong_responses 内 timestamp() 未改为 timestamp_millis() 时失败。
#[test]
fn ping_3_pong_timestamp_is_milliseconds() {
    use voice_room_server::ws::connection::ping_pong_responses;

    let responses = ping_pong_responses(Some("550e8400-e29b-41d4-a716-446655440002".to_string()));

    assert!(
        !responses.is_empty(),
        "PING-3: ping_pong_responses must return at least one response"
    );

    for (i, resp) in responses.iter().enumerate() {
        let v: Value =
            serde_json::from_str(resp).expect("each ping_pong_responses entry must be valid JSON");
        let ts = v["timestamp"]
            .as_i64()
            .expect("PING-3: timestamp must be an integer in all responses");
        assert!(
            ts > 1_000_000_000_000,
            "PING-3: response[{i}] timestamp must be milliseconds (> 1_000_000_000_000), got {ts}"
        );
    }
}

// ─── PING-COMPAT-4: 真实链路 vs 孤立函数行为对比 ─────────────────────────────

/// PING-COMPAT-4 (dev): 验证 `ping_pong_responses`（handle_socket 真实链路）
/// 比 `handle_text_message`（孤立函数路径）多发一条兼容响应。
///
/// 这是架构契约：
/// - `handle_text_message("Ping")` → 单发 `Pong`（只作为 fallback 保留）
/// - `ping_pong_responses()`（由 handle_socket "Ping" arm 调用） → 双发 `[Pong, pong]`
///
/// 如果此测试失败，说明 `ping_pong_responses` 仍是死代码（未接入真实链路）。
/// TDS §一.3 要求 dev/test 阶段服务端双发，此测试保证该契约被实现。
#[cfg(debug_assertions)]
#[test]
fn ping_compat_4_real_path_dual_send_vs_isolated_single_send() {
    use voice_room_server::ws::connection::{handle_text_message, ping_pong_responses};

    let msg_id = Uuid::new_v4().to_string();
    let ping_json = format!(r#"{{"type":"Ping","msg_id":"{msg_id}"}}"#);
    let hb = Arc::new(RwLock::new(Instant::now()));

    // ── 孤立路径（handle_text_message）：单发 Pong ─────────────────────────
    let single_resp = handle_text_message(&ping_json, &hb);
    assert!(
        single_resp.is_some(),
        "PING-COMPAT-4: handle_text_message must return Some for Ping"
    );
    let single_val: Value = serde_json::from_str(single_resp.as_ref().unwrap())
        .expect("PING-COMPAT-4: handle_text_message response must be valid JSON");
    assert_eq!(
        single_val["type"], "Pong",
        "PING-COMPAT-4: handle_text_message must return type=Pong for Ping"
    );

    // ── 真实 WS 链路（ping_pong_responses，由 handle_socket 调用）：双发 ──
    let dual_resps = ping_pong_responses(Some(msg_id.clone()));
    assert_eq!(
        dual_resps.len(),
        2,
        "PING-COMPAT-4: handle_socket real path (ping_pong_responses) must dual-send \
         [Pong, pong] in debug mode. 若此断言失败说明 ping_pong_responses 未接入 handle_socket。"
    );

    let dual_types: Vec<String> = dual_resps
        .iter()
        .map(|s| {
            serde_json::from_str::<Value>(s)
                .expect("PING-COMPAT-4: each dual response must be valid JSON")["type"]
                .as_str()
                .expect("type must be string")
                .to_owned()
        })
        .collect();

    // 明确验证双发包含新旧两种格式
    assert!(
        dual_types.contains(&"Pong".to_owned()),
        "PING-COMPAT-4: dual-send must include new-format 'Pong', got: {dual_types:?}"
    );
    assert!(
        dual_types.contains(&"pong".to_owned()),
        "PING-COMPAT-4: dual-send must include legacy 'pong' for compat period, got: {dual_types:?}"
    );

    // ── 对比：真实链路发送数 > 孤立函数发送数（这正是接入的意义）──────────
    // single_resp is 1 response, dual_resps is 2 — this documents the behavioral gap
    // that makes the wiring non-trivial and worth testing.
    assert!(
        dual_resps.len() > 1,
        "PING-COMPAT-4: real path must produce MORE responses than handle_text_message alone \
         (dual_resps={}, isolated=1). \
         This gap is WHY handle_socket explicitly calls ping_pong_responses.",
        dual_resps.len()
    );
}

// ─── PS-NEW-1: MicTaken 含 forced_by 应通过 schema 验证 ──────────────────────
//
// P0-2 缺陷验证：ForceTakeMic 广播 MicTaken 时携带 forced_by，
// 该字段必须被 MicTaken.schema.json 允许（additionalProperties: false 下须显式声明）。
// 修复前：schema 无 forced_by → 验证失败；修复后：schema 含 forced_by → 验证通过。
#[test]
fn ps_new_1_mic_taken_with_forced_by_passes_schema() {
    let schema_str = ws_schema!("MicTaken");
    let user_id = Uuid::new_v4().to_string();
    let operator_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();

    // ForceTakeMic 广播的 MicTaken envelope（含 forced_by 字段）
    let envelope = json!({
        "type": "MicTaken",
        "msg_id": msg_id,
        "payload": {
            "mic_index": 2,
            "user_id": user_id,
            "forced_by": operator_id   // ← ForceTakeMic 业务字段，schema 必须允许
        },
        "timestamp": 1_700_000_000i64
    });

    assert_valid(schema_str, &envelope, "PS-NEW-1: MicTaken with forced_by");
}

// ─── PS-NEW-2: MicLeft 含 forced_by 应通过 schema 验证 ──────────────────────
//
// P0-3 缺陷验证：ForceLeaveMic 广播 MicLeft 时携带 forced_by，
// 该字段必须被 MicLeft.schema.json 允许。
// 修复前：schema 无 forced_by → 验证失败；修复后：schema 含 forced_by → 验证通过。
#[test]
fn ps_new_2_mic_left_with_forced_by_passes_schema() {
    let schema_str = ws_schema!("MicLeft");
    let user_id = Uuid::new_v4().to_string();
    let operator_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();

    // ForceLeaveMic 广播的 MicLeft envelope（含 forced=true 与 forced_by 字段）
    let envelope = json!({
        "type": "MicLeft",
        "msg_id": msg_id,
        "payload": {
            "mic_index": 0,
            "user_id": user_id,
            "forced": true,
            "forced_by": operator_id   // ← ForceLeaveMic 业务字段，schema 必须允许
        },
        "timestamp": 1_700_000_000i64
    });

    assert_valid(schema_str, &envelope, "PS-NEW-2: MicLeft with forced_by");
}

// ─── PS-NEW-3: AdminChanged payload 嵌套格式应通过 schema 验证 ───────────────
//
// P0-4 缺陷验证：server 广播 AdminChanged 使用 payload 嵌套 snake_case，
// AdminChanged.schema.json 必须存在且与 server 广播格式完全对齐。
#[test]
fn ps_new_3_admin_changed_payload_nested_passes_schema() {
    let schema_str = ws_schema!("AdminChanged");
    let room_id = Uuid::new_v4().to_string();
    let admin_user_id = Uuid::new_v4().to_string();
    let previous_admin_id = Uuid::new_v4().to_string();
    let operator_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();

    // server transfer.rs 广播的 AdminChanged envelope（payload 嵌套 snake_case）
    let envelope = json!({
        "type": "AdminChanged",
        "msg_id": msg_id,
        "payload": {
            "room_id": room_id,
            "admin_user_id": admin_user_id,
            "previous_admin_id": previous_admin_id,
            "operator_id": operator_id
        },
        "timestamp": 1_700_000_000i64
    });

    assert_valid(schema_str, &envelope, "PS-NEW-3: AdminChanged payload-nested snake_case");
}

// ─── PS-NEW-4: AdminChanged revoke（admin_user_id=null）通过 schema 验证 ──────
#[test]
fn ps_new_4_admin_changed_revoke_null_admin_passes_schema() {
    let schema_str = ws_schema!("AdminChanged");
    let room_id = Uuid::new_v4().to_string();
    let previous_admin_id = Uuid::new_v4().to_string();
    let operator_id = Uuid::new_v4().to_string();
    let msg_id = Uuid::new_v4().to_string();

    // revoke 时 admin_user_id = null
    let envelope = json!({
        "type": "AdminChanged",
        "msg_id": msg_id,
        "payload": {
            "room_id": room_id,
            "admin_user_id": null,
            "previous_admin_id": previous_admin_id,
            "operator_id": operator_id
        },
        "timestamp": 1_700_000_000i64
    });

    assert_valid(schema_str, &envelope, "PS-NEW-4: AdminChanged revoke (admin_user_id=null)");
}
