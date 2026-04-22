//! WS 信令处理器 — ReportEvent（T-00023）
//!
//! ## 功能
//! - 处理 WS `ReportEvent` 信令，复用 T-00022 的 `EventWriter`
//! - JWT user_id 覆盖客户端上报的 user_id（WS 必然登录）
//! - 单次 > 100 events → code 40204 (BATCH_TOO_LARGE) 但仍写前 100 条
//! - payload.events 为空 → code 40003 (VALIDATION_ERROR)
//! - device_id 缺失 → code 40002 (PARAMETER_MISSING)
//!
//! ## 信令格式
//! ```json
//! // C→S
//! { "type": "ReportEvent", "msg_id": "uuid", "payload": { "events": [...] } }
//!
//! // S→C
//! { "type": "EventReportAck", "msg_id": "uuid", "code": 0,
//!   "payload": { "received": 98, "rejected_indices": [100] } }
//! ```
//!
//! ## 与 HTTP 通道的复用关系
//! `handle_report_event` 与 HTTP `batch_events` handler 共用同一 `EventWriter::persist`，
//! 无任何重复写入逻辑。

use std::sync::Arc;

use uuid::Uuid;

use crate::common::error::AppError;
use crate::core::analytics::writer::{EventInput, EventWriterPort};

// ─── 依赖注入结构 ──────────────────────────────────────────────────────────────

/// `handle_report_event` 所需依赖
pub struct ReportEventDeps {
    /// 事件写入服务（与 HTTP 通道共享同一实例）
    pub event_writer: Arc<dyn EventWriterPort>,
}

// ─── 错误码常量 ────────────────────────────────────────────────────────────────

/// BATCH_TOO_LARGE 错误码（超出 100 条限制，仍写前 100）
const CODE_BATCH_TOO_LARGE: i32 = 40204;

/// VALIDATION_ERROR 错误码（payload 非法或 events 为空）
const CODE_VALIDATION_ERROR: i32 = 40003;

/// PARAMETER_MISSING 错误码（device_id 缺失）
const CODE_PARAMETER_MISSING: i32 = 40002;

/// INTERNAL_ERROR 错误码
const CODE_INTERNAL_ERROR: i32 = 50000;

// ─── 纯函数 WS 信令处理器 ──────────────────────────────────────────────────────

/// 处理 WS `ReportEvent` 信令，返回 `EventReportAck` JSON 字符串
///
/// # 参数
/// - `payload`：信令 payload（来自 `IncomingMessage.payload`）
/// - `msg_id`：信令 msg_id，回显到 ACK
/// - `jwt_user_id`：WS 连接鉴权时从 JWT 解析出的 user_id（覆盖客户端上报值）
/// - `deps`：依赖注入（EventWriter）
///
/// # 返回
/// JSON 字符串，格式为 `EventReportAck`
pub async fn handle_report_event(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    jwt_user_id: Uuid,
    deps: &ReportEventDeps,
) -> String {
    // 1. 校验 payload 存在
    let payload_val = match payload {
        Some(p) => p,
        None => {
            return build_ack(msg_id, CODE_VALIDATION_ERROR, 0, vec![]);
        }
    };

    // 2. 提取 events 字段
    let events_arr = match payload_val.get("events").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => {
            return build_ack(msg_id, CODE_VALIDATION_ERROR, 0, vec![]);
        }
    };

    // 3. events 不可为空
    if events_arr.is_empty() {
        return build_ack(msg_id, CODE_VALIDATION_ERROR, 0, vec![]);
    }

    // 4. 反序列化为 Vec<EventInput>
    let batch: Vec<EventInput> =
        match serde_json::from_value(serde_json::Value::Array(events_arr)) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = %e, "failed to deserialize ReportEvent events");
                return build_ack(msg_id, CODE_VALIDATION_ERROR, 0, vec![]);
            }
        };

    // 5. 检查是否超出批量限制（超出时写前 100 条并返回 BATCH_TOO_LARGE）
    let batch_too_large = batch.len() > 100;

    // 6. 调用 EventWriter::persist（与 HTTP 通道共享，无重复代码）
    match deps.event_writer.persist(batch, Some(jwt_user_id)).await {
        Ok(result) => {
            // 超出 100 条时返回 40204，否则 0
            let code = if batch_too_large {
                CODE_BATCH_TOO_LARGE
            } else {
                0
            };
            build_ack(msg_id, code, result.received, result.rejected_indices)
        }
        Err(AppError::ParameterMissing(field)) => {
            tracing::warn!(field = %field, "ReportEvent: parameter missing");
            build_ack(msg_id, CODE_PARAMETER_MISSING, 0, vec![])
        }
        Err(e) => {
            tracing::error!(error = %e, "ReportEvent: persist failed");
            build_ack(msg_id, CODE_INTERNAL_ERROR, 0, vec![])
        }
    }
}

// ─── 响应构建辅助 ──────────────────────────────────────────────────────────────

/// 构建 `EventReportAck` JSON 字符串
fn build_ack(
    msg_id: Option<String>,
    code: i32,
    received: usize,
    rejected_indices: Vec<usize>,
) -> String {
    let resp = serde_json::json!({
        "type": "EventReportAck",
        "msg_id": msg_id,
        "code": code,
        "payload": {
            "received": received,
            "rejected_indices": rejected_indices
        }
    });
    serde_json::to_string(&resp).unwrap_or_else(|_| {
        r#"{"type":"EventReportAck","code":50000,"payload":{"received":0,"rejected_indices":[]}}"#
            .to_string()
    })
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::analytics::writer::FakeEventWriter;

    fn make_deps_local() -> (ReportEventDeps, Arc<FakeEventWriter>) {
        let fake = Arc::new(FakeEventWriter::default());
        let deps = ReportEventDeps {
            event_writer: fake.clone(),
        };
        (deps, fake)
    }

    fn make_payload(n: usize) -> serde_json::Value {
        let events: Vec<serde_json::Value> = (0..n)
            .map(|i| {
                serde_json::json!({
                    "event_name": format!("event_{i}"),
                    "device_id": format!("device-{i}"),
                    "properties": {}
                })
            })
            .collect();
        serde_json::json!({ "events": events })
    }

    // WS01: build_ack — code=0 响应格式正确
    #[test]
    fn ws01_build_ack_success_format() {
        let s = build_ack(Some("msg-1".to_string()), 0, 5, vec![]);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["type"], "EventReportAck");
        assert_eq!(v["msg_id"], "msg-1");
        assert_eq!(v["code"], 0);
        assert_eq!(v["payload"]["received"], 5);
        assert_eq!(v["payload"]["rejected_indices"].as_array().unwrap().len(), 0);
    }

    // WS02: build_ack — BATCH_TOO_LARGE 格式正确
    #[test]
    fn ws02_build_ack_batch_too_large_format() {
        let s = build_ack(Some("msg-2".to_string()), 40204, 100, vec![100]);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["code"], 40204);
        assert_eq!(v["payload"]["received"], 100);
        let rejected: Vec<usize> =
            serde_json::from_value(v["payload"]["rejected_indices"].clone()).unwrap();
        assert_eq!(rejected, vec![100usize]);
    }

    // WS03: build_ack — msg_id 为 None 时序列化为 null
    #[test]
    fn ws03_build_ack_null_msg_id() {
        let s = build_ack(None, 0, 1, vec![]);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(v["msg_id"].is_null(), "msg_id should be null when None");
    }

    // WS04: 空 events → code=40003
    #[tokio::test]
    async fn ws04_empty_events_returns_40003() {
        let (deps, _) = make_deps_local();
        let payload = serde_json::json!({ "events": [] });
        let resp =
            handle_report_event(Some(payload), Some("m1".to_string()), Uuid::new_v4(), &deps)
                .await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["code"], 40003);
    }

    // WS05: payload=None → code=40003
    #[tokio::test]
    async fn ws05_missing_payload_returns_40003() {
        let (deps, _) = make_deps_local();
        let resp = handle_report_event(None, Some("m2".to_string()), Uuid::new_v4(), &deps).await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["code"], 40003);
    }

    // WS06: events 字段缺失 → code=40003
    #[tokio::test]
    async fn ws06_missing_events_field_returns_40003() {
        let (deps, _) = make_deps_local();
        let payload = serde_json::json!({ "other_field": 123 });
        let resp =
            handle_report_event(Some(payload), Some("m3".to_string()), Uuid::new_v4(), &deps)
                .await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["code"], 40003);
    }

    // WS07: 1 event → code=0, received=1
    #[tokio::test]
    async fn ws07_single_event_success() {
        let (deps, fake) = make_deps_local();
        let payload = make_payload(1);
        let uid = Uuid::new_v4();
        let resp =
            handle_report_event(Some(payload), Some("m4".to_string()), uid, &deps).await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["code"], 0);
        assert_eq!(v["payload"]["received"], 1);
        let stored = fake.stored.lock().unwrap();
        assert_eq!(stored.len(), 1);
    }

    // WS08: 100 events → code=0, received=100
    #[tokio::test]
    async fn ws08_100_events_success() {
        let (deps, fake) = make_deps_local();
        let payload = make_payload(100);
        let uid = Uuid::new_v4();
        let resp =
            handle_report_event(Some(payload), Some("m5".to_string()), uid, &deps).await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["code"], 0);
        assert_eq!(v["payload"]["received"], 100);
        let stored = fake.stored.lock().unwrap();
        assert_eq!(stored.len(), 100);
    }

    // WS09: 101 events → code=40204, received=100, rejected=[100]
    #[tokio::test]
    async fn ws09_101_events_batch_too_large() {
        let (deps, fake) = make_deps_local();
        let payload = make_payload(101);
        let uid = Uuid::new_v4();
        let resp =
            handle_report_event(Some(payload), Some("m6".to_string()), uid, &deps).await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["code"], 40204);
        assert_eq!(v["payload"]["received"], 100);
        let rejected: Vec<usize> =
            serde_json::from_value(v["payload"]["rejected_indices"].clone()).unwrap();
        assert_eq!(rejected, vec![100usize]);
        let stored = fake.stored.lock().unwrap();
        assert_eq!(stored.len(), 100);
    }

    // WS10: device_id 空字符串 → code=40002
    #[tokio::test]
    async fn ws10_empty_device_id_returns_40002() {
        let (deps, _) = make_deps_local();
        let payload = serde_json::json!({
            "events": [{ "event_name": "e", "device_id": "", "properties": {} }]
        });
        let resp =
            handle_report_event(Some(payload), Some("m7".to_string()), Uuid::new_v4(), &deps)
                .await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["code"], 40002);
    }

    // WS11: JWT user_id 覆盖客户端 user_id
    #[tokio::test]
    async fn ws11_jwt_user_id_overrides_client_user_id() {
        let (deps, fake) = make_deps_local();
        let jwt_uid = Uuid::new_v4();
        let client_uid = Uuid::new_v4();
        let payload = serde_json::json!({
            "events": [{
                "event_name": "e",
                "device_id": "d1",
                "user_id": client_uid.to_string(),
                "properties": {}
            }]
        });
        let resp =
            handle_report_event(Some(payload), Some("m8".to_string()), jwt_uid, &deps).await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["code"], 0);
        let stored = fake.stored.lock().unwrap();
        let (event, _) = &stored[0];
        assert_eq!(event.user_id, Some(jwt_uid), "JWT uid must override client uid");
    }

    // WS12: msg_id 在 ACK 中回显
    #[tokio::test]
    async fn ws12_msg_id_echoed_in_ack() {
        let (deps, _) = make_deps_local();
        let payload = make_payload(1);
        let resp = handle_report_event(
            Some(payload),
            Some("unique-msg-echo-123".to_string()),
            Uuid::new_v4(),
            &deps,
        )
        .await;
        let v: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["msg_id"], "unique-msg-echo-123");
    }

    // WS13: CODE_BATCH_TOO_LARGE 常量值为 40204
    #[test]
    fn ws13_batch_too_large_code_is_40204() {
        assert_eq!(CODE_BATCH_TOO_LARGE, 40204);
    }

    // WS14: CODE_VALIDATION_ERROR 常量值为 40003
    #[test]
    fn ws14_validation_error_code_is_40003() {
        assert_eq!(CODE_VALIDATION_ERROR, 40003);
    }
}
