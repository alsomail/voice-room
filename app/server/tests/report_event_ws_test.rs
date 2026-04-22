//! 集成/单元测试 — T-00023 WS ReportEvent 信令 + EventWriter 复用
//!
//! 测试用例 RE01~RE08 验证以下内容：
//! - RE01: 认证 WS 发 ReportEvent 1 event → received=1, code=0
//! - RE02: 50 events 一次上报 → received=50, code=0
//! - RE03: 101 events → code=40204(BATCH_TOO_LARGE), received=100, rejected_indices=[100]
//! - RE04: server_ts 由 writer 统一覆盖（client_ts 仅作参考）
//! - RE05: 客户端 user_id ≠ JWT user_id → 存 JWT user_id（JWT 优先）
//! - RE06: payload.events 为空数组 → code=40003
//! - RE07: payload 缺失 → code=40003
//! - RE08: 与 HTTP 通道并行写入 1000 条无丢失（FakeEventWriter）
//!
//! 全部为内存/单元测试（不依赖 DATABASE_URL），使用 FakeEventWriter。

use std::sync::Arc;

use uuid::Uuid;

use voice_room_server::{
    core::analytics::writer::{EventInput, FakeEventWriter},
    modules::events::ws::{ReportEventDeps, handle_report_event},
};

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 创建带 FakeEventWriter 的 ReportEventDeps
fn make_deps() -> (ReportEventDeps, Arc<FakeEventWriter>) {
    let fake = Arc::new(FakeEventWriter::default());
    let deps = ReportEventDeps {
        event_writer: fake.clone(),
    };
    (deps, fake)
}

/// 构造单条事件 JSON
fn make_event_json(idx: usize) -> serde_json::Value {
    serde_json::json!({
        "event_name": format!("test_event_{idx}"),
        "device_id": format!("device-{idx:04}"),
        "user_id": null,
        "session_id": format!("sess-{idx}"),
        "client_ts": 1720000000000_i64,
        "properties": { "idx": idx },
        "app_version": "1.0.0",
        "os_version": "Android 14",
        "locale": "ar-SA",
        "network_type": "wifi"
    })
}

/// 构造含 N 条事件的 ReportEvent payload
fn make_payload(n: usize) -> serde_json::Value {
    let events: Vec<serde_json::Value> = (0..n).map(make_event_json).collect();
    serde_json::json!({ "events": events })
}

// ─── RE01: 1 event → received=1, code=0 ────────────────────────────────────

/// RE01: 认证 WS 发 ReportEvent 1 event，handler 返回 code=0 且 received=1
#[tokio::test]
async fn re01_single_event_returns_code0_received1() {
    let (deps, fake) = make_deps();
    let jwt_user_id = Uuid::new_v4();
    let payload = make_payload(1);

    let response = handle_report_event(
        Some(payload),
        Some("msg-re01".to_string()),
        jwt_user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).expect("response must be valid JSON");

    assert_eq!(json["type"], "EventReportAck", "type should be EventReportAck");
    assert_eq!(json["msg_id"], "msg-re01", "msg_id must echo");
    assert_eq!(json["code"], 0, "code should be 0 on success");
    assert_eq!(json["payload"]["received"], 1, "received should be 1");
    assert_eq!(
        json["payload"]["rejected_indices"].as_array().unwrap().len(),
        0,
        "no rejected indices"
    );

    // 验证 FakeEventWriter 存储了 1 条事件
    let stored = fake.stored.lock().unwrap();
    assert_eq!(stored.len(), 1, "FakeEventWriter should have stored 1 event");
}

// ─── RE02: 50 events → received=50, code=0 ─────────────────────────────────

/// RE02: 50 events 一次上报，返回 received=50, code=0
#[tokio::test]
async fn re02_50_events_returns_received50() {
    let (deps, fake) = make_deps();
    let jwt_user_id = Uuid::new_v4();
    let payload = make_payload(50);

    let response = handle_report_event(
        Some(payload),
        Some("msg-re02".to_string()),
        jwt_user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(json["code"], 0);
    assert_eq!(json["payload"]["received"], 50);
    assert_eq!(
        json["payload"]["rejected_indices"].as_array().unwrap().len(),
        0
    );

    let stored = fake.stored.lock().unwrap();
    assert_eq!(stored.len(), 50);
}

// ─── RE03: 101 events → BATCH_TOO_LARGE (40204), received=100, rejected=[100] ─

/// RE03: 101 events → code=40204, received=100, rejected_indices=[100]
#[tokio::test]
async fn re03_101_events_batch_too_large_writes_100() {
    let (deps, fake) = make_deps();
    let jwt_user_id = Uuid::new_v4();
    let payload = make_payload(101);

    let response = handle_report_event(
        Some(payload),
        Some("msg-re03".to_string()),
        jwt_user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(json["type"], "EventReportAck");
    assert_eq!(json["code"], 40204, "should return BATCH_TOO_LARGE code 40204");
    assert_eq!(json["payload"]["received"], 100, "first 100 should be written");

    let rejected: Vec<usize> =
        serde_json::from_value(json["payload"]["rejected_indices"].clone()).unwrap();
    assert_eq!(rejected, vec![100usize], "only index 100 should be rejected");

    // 验证 FakeEventWriter 存储了前 100 条
    let stored = fake.stored.lock().unwrap();
    assert_eq!(stored.len(), 100, "FakeEventWriter should have stored exactly 100 events");
}

// ─── RE04: server_ts 覆盖 client_ts ─────────────────────────────────────────

/// RE04: client_ts 字段被传入，EventWriter 会以 server_ts 覆盖（FakeEventWriter
///       不存储 server_ts，但验证 client_ts 字段被正确传入 EventInput）
#[tokio::test]
async fn re04_client_ts_passed_through_to_event_input() {
    let (deps, fake) = make_deps();
    let jwt_user_id = Uuid::new_v4();

    // 构造 payload 含特定 client_ts
    let payload = serde_json::json!({
        "events": [{
            "event_name": "re04_event",
            "device_id": "device-re04",
            "client_ts": 1720000012345_i64,
            "properties": {}
        }]
    });

    let response = handle_report_event(
        Some(payload),
        Some("msg-re04".to_string()),
        jwt_user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(json["code"], 0);

    // 验证 client_ts 被正确传给了 EventInput
    let stored = fake.stored.lock().unwrap();
    assert_eq!(stored.len(), 1);
    let (event, _) = &stored[0];
    assert_eq!(
        event.client_ts,
        Some(1720000012345_i64),
        "client_ts should be preserved in EventInput for EventWriter"
    );
}

// ─── RE05: client user_id ≠ JWT user_id → 存 JWT user_id ───────────────────

/// RE05: 客户端上报 user_id 与 JWT 不一致时，DB 存 JWT user_id（JWT 优先）
#[tokio::test]
async fn re05_client_user_id_overridden_by_jwt() {
    let (deps, fake) = make_deps();
    let jwt_user_id = Uuid::new_v4();
    let client_user_id = Uuid::new_v4(); // 不同的 user_id

    let payload = serde_json::json!({
        "events": [{
            "event_name": "re05_event",
            "device_id": "device-re05",
            "user_id": client_user_id.to_string(),
            "properties": {}
        }]
    });

    let response = handle_report_event(
        Some(payload),
        Some("msg-re05".to_string()),
        jwt_user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(json["code"], 0);

    // 验证存储的事件 user_id 为 JWT user_id（而非客户端提交的）
    let stored = fake.stored.lock().unwrap();
    assert_eq!(stored.len(), 1);
    let (event, jwt_uid) = &stored[0];
    assert_eq!(
        event.user_id,
        Some(jwt_user_id),
        "stored user_id must be JWT user_id, not client user_id"
    );
    assert_eq!(
        *jwt_uid,
        Some(jwt_user_id),
        "jwt_user_id passed to persist should match"
    );
}

// ─── RE06: payload.events 为空数组 → code=40003 ────────────────────────────

/// RE06: payload.events 为空数组 → 返回 code=40003 (VALIDATION_ERROR)
#[tokio::test]
async fn re06_empty_events_array_returns_40003() {
    let (deps, _fake) = make_deps();
    let jwt_user_id = Uuid::new_v4();

    let payload = serde_json::json!({ "events": [] });

    let response = handle_report_event(
        Some(payload),
        Some("msg-re06".to_string()),
        jwt_user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(json["type"], "EventReportAck");
    assert_eq!(json["code"], 40003, "empty events should return code 40003");
}

// ─── RE07: payload 缺失 → code=40003 ───────────────────────────────────────

/// RE07: payload 为 None → 返回 code=40003 (payload 非法)
#[tokio::test]
async fn re07_missing_payload_returns_40003() {
    let (deps, _fake) = make_deps();
    let jwt_user_id = Uuid::new_v4();

    let response = handle_report_event(
        None, // 无 payload
        Some("msg-re07".to_string()),
        jwt_user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert_eq!(json["code"], 40003, "missing payload should return 40003");
    assert_eq!(json["msg_id"], "msg-re07");
}

// ─── RE08: HTTP + WS 并发 1000 条无丢失 ─────────────────────────────────────

/// RE08: 与 HTTP 通道并行写入 1000 条无丢失（各 500 条）
///
/// 使用同一个 FakeEventWriter 实例，并发提交 HTTP 和 WS 各 500 条事件，
/// 总计 1000 条，全部无丢失。
#[tokio::test]
async fn re08_concurrent_http_and_ws_writes_no_loss() {
    use voice_room_server::core::analytics::writer::EventWriterPort;

    let fake = Arc::new(FakeEventWriter::default());
    let _ws_deps = ReportEventDeps {
        event_writer: fake.clone() as Arc<dyn EventWriterPort>,
    };

    let jwt_user_id = Uuid::new_v4();

    // WS 写 500 条（10批次 × 50条）
    let ws_fake = Arc::clone(&fake);
    let ws_handle = tokio::spawn(async move {
        for batch_i in 0..10_usize {
            let events: Vec<serde_json::Value> = (0..50)
                .map(|j| {
                    serde_json::json!({
                        "event_name": format!("ws_event_{batch_i}_{j}"),
                        "device_id": format!("ws-device-{batch_i}-{j}"),
                        "properties": {}
                    })
                })
                .collect();
            let payload = serde_json::json!({ "events": events });
            let deps_local = ReportEventDeps {
                event_writer: ws_fake.clone() as Arc<dyn EventWriterPort>,
            };
            let _ = handle_report_event(
                Some(payload),
                Some(format!("ws-msg-{batch_i}")),
                jwt_user_id,
                &deps_local,
            )
            .await;
        }
    });

    // HTTP 模拟写 500 条（10批次 × 50条，直接调用 FakeEventWriter.persist）
    let http_fake = Arc::clone(&fake);
    let http_handle = tokio::spawn(async move {
        for batch_i in 0..10_usize {
            let batch: Vec<EventInput> = (0..50)
                .map(|j| EventInput {
                    event_name: format!("http_event_{batch_i}_{j}"),
                    device_id: format!("http-device-{batch_i}-{j}"),
                    user_id: None,
                    session_id: None,
                    client_ts: None,
                    properties: serde_json::json!({}),
                    app_version: None,
                    os_version: None,
                    locale: None,
                    network_type: None,
                })
                .collect();
            http_fake.persist(batch, None).await.unwrap();
        }
    });

    let _ = tokio::join!(ws_handle, http_handle);

    let stored = fake.stored.lock().unwrap();
    assert_eq!(
        stored.len(),
        1000,
        "concurrent WS + HTTP writes should result in exactly 1000 stored events, got {}",
        stored.len()
    );
}

// ─── 额外验证：msg_id 为 None 时响应也包含 msg_id=null ──────────────────────

/// 补充：msg_id 为 None 时响应格式也正确（msg_id 字段存在但为 null）
#[tokio::test]
async fn re_extra_null_msg_id_response_is_valid() {
    let (deps, _) = make_deps();
    let jwt_user_id = Uuid::new_v4();
    let payload = make_payload(1);

    let response = handle_report_event(Some(payload), None, jwt_user_id, &deps).await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(json["type"], "EventReportAck");
    assert_eq!(json["code"], 0);
    // msg_id 存在且为 null（或缺失）- 两种都可接受
    assert!(
        json.get("msg_id").is_some(),
        "msg_id field should exist (may be null)"
    );
}

// ─── 额外验证：device_id 缺失 → code=40002 ─────────────────────────────────

/// RE_EXTRA: device_id 缺失时 EventWriter 返回 40002（ParameterMissing）
#[tokio::test]
async fn re_extra_missing_device_id_returns_40002() {
    let (deps, _) = make_deps();
    let jwt_user_id = Uuid::new_v4();

    let payload = serde_json::json!({
        "events": [{
            "event_name": "missing_device",
            "device_id": "",  // 空字符串
            "properties": {}
        }]
    });

    let response = handle_report_event(
        Some(payload),
        Some("msg-device".to_string()),
        jwt_user_id,
        &deps,
    )
    .await;

    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(
        json["code"], 40002,
        "empty device_id should return code 40002 (ParameterMissing)"
    );
}
