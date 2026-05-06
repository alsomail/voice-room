//! T-00103 WS 出栈 schema 校验中间层
//!
//! 从 `doc/protocol/schemas/ws/*.schema.json` 加载（编译期嵌入）冻结协议 schema，
//! 对每条 WS 出栈 envelope 进行结构校验。
//!
//! ## 行为
//!
//! | Profile | mismatch 时 |
//! |---------|------------|
//! | test 构建 (`#[cfg(test)]`) | **panic** — 立即发现回归 |
//! | prod / debug build         | **no-op**（jsonschema 不链接）|
//!
//! 在单测中可通过 `guard_outbound_with_dev_flag(envelope, is_dev)` 注入行为，
//! 验证"不 panic"语义（GUARD-2）。

use serde_json::Value;

// ─── 编译期嵌入 schema（仅 test 构建）────────────────────────────────────────

#[cfg(test)]
macro_rules! ws_schema_str {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../doc/protocol/schemas/ws/",
            $name,
            ".schema.json"
        ))
    };
}

/// 已注册的 WS 出栈 envelope 类型 → schema 字符串对（仅 test 构建）。
#[cfg(test)]
const REGISTERED_SCHEMAS: &[(&str, &str)] = &[
    // ── 广播类（S→Room）
    ("MicTaken", ws_schema_str!("MicTaken")),
    ("MicLeft", ws_schema_str!("MicLeft")),
    ("UserJoined", ws_schema_str!("UserJoined")),
    ("UserLeft", ws_schema_str!("UserLeft")),
    ("RoomMessage", ws_schema_str!("RoomMessage")),
    // ── Result 类（S→C）
    ("JoinRoomResult", ws_schema_str!("JoinRoomResult")),
    ("LeaveRoomResult", ws_schema_str!("LeaveRoomResult")),
    ("TakeMicResult", ws_schema_str!("TakeMicResult")),
    ("LeaveMicResult", ws_schema_str!("LeaveMicResult")),
    ("SendGiftResult", ws_schema_str!("SendGiftResult")),
    ("SendMessageResult", ws_schema_str!("SendMessageResult")),
    // ── 心跳（S→C）
    ("Pong", ws_schema_str!("Pong")),
];

// ─── 公共 API ─────────────────────────────────────────────────────────────────

/// 对出栈 WS envelope 执行 schema 校验。
///
/// - **test 构建** (`#[cfg(test)]`)：mismatch → **panic**
/// - **其他构建**：**no-op**（jsonschema 不链接）
///
/// 若 envelope 的 `type` 未在注册表中（如 `RoomInfoUpdated`），静默跳过。
#[inline]
pub fn guard_outbound_envelope(envelope: &Value) {
    #[cfg(test)]
    _guard_impl(envelope, true);
    #[cfg(not(test))]
    let _ = envelope;
}

/// 内部版本，接受显式 `is_dev` 标志，供单测注入 prod 行为（GUARD-2）。
///
/// - `is_dev = true`  → panic on validation failure（dev 路径）
/// - `is_dev = false` → ERROR log on validation failure（prod 路径）
///
/// 在 test 构建中有效；其他构建为 no-op。
#[inline]
pub fn guard_outbound_with_dev_flag(envelope: &Value, is_dev: bool) {
    #[cfg(test)]
    _guard_impl(envelope, is_dev);
    #[cfg(not(test))]
    let _ = (envelope, is_dev);
}

// ─── 内部实现（仅 test 构建编译）────────────────────────────────────────────

#[cfg(test)]
fn _guard_impl(envelope: &Value, is_dev: bool) {
    use jsonschema::JSONSchema;

    let type_name = match envelope.get("type").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => {
            let msg = format!(
                "schema_guard: outbound envelope missing 'type' field: {envelope}"
            );
            if is_dev {
                panic!("{msg}");
            } else {
                tracing::error!("{msg}");
                return;
            }
        }
    };

    // 查找已注册 schema；未注册则跳过（向前兼容）
    let schema_str = match REGISTERED_SCHEMAS
        .iter()
        .find(|(name, _)| *name == type_name)
    {
        Some((_, s)) => *s,
        None => return,
    };

    let schema_value: Value =
        serde_json::from_str(schema_str).expect("embedded schema must be valid JSON");

    let validator =
        JSONSchema::compile(&schema_value).expect("embedded schema must compile");

    // Collect errors to owned Strings immediately so `validator` borrow is released
    let error_msgs: Vec<String> = match validator.validate(envelope) {
        Ok(()) => return, // valid — happy path
        Err(errs) => errs.map(|e| e.to_string()).collect(),
    };

    let msg = format!(
        "schema_guard: envelope type='{type_name}' \
         failed schema validation: {error_msgs:?}"
    );
    if is_dev {
        panic!("{msg}");
    } else {
        tracing::error!("{msg}");
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn valid_mic_taken() -> Value {
        let user_id = Uuid::new_v4().to_string();
        let msg_id = Uuid::new_v4().to_string();
        json!({
            "type": "MicTaken",
            "msg_id": msg_id,
            "payload": {
                "mic_index": 2,
                "user_id": user_id
            },
            "timestamp": 1_700_000_000i64
        })
    }

    fn invalid_mic_taken_missing_mic_index() -> Value {
        let user_id = Uuid::new_v4().to_string();
        let msg_id = Uuid::new_v4().to_string();
        json!({
            "type": "MicTaken",
            "msg_id": msg_id,
            "payload": {
                // mic_index 缺失！
                "user_id": user_id
            },
            "timestamp": 1_700_000_000i64
        })
    }

    // ── GUARD-1: dev profile → panic on invalid envelope ─────────────────────

    /// GUARD-1: dev 模式下，缺 mic_index 的 MicTaken envelope 触发 panic
    #[test]
    fn guard1_dev_mode_panics_on_invalid_mic_taken() {
        let invalid = invalid_mic_taken_missing_mic_index();

        let result = std::panic::catch_unwind(|| {
            guard_outbound_with_dev_flag(&invalid, true);
        });

        assert!(
            result.is_err(),
            "GUARD-1: dev mode must panic on invalid MicTaken (missing mic_index)"
        );
    }

    // ── GUARD-2: prod profile → ERROR log, no panic ──────────────────────────

    /// GUARD-2: prod 模式下，invalid envelope 不 panic（仅 ERROR log）
    #[test]
    fn guard2_prod_mode_no_panic_on_invalid_envelope() {
        let invalid = invalid_mic_taken_missing_mic_index();

        let result = std::panic::catch_unwind(|| {
            guard_outbound_with_dev_flag(&invalid, false);
        });

        assert!(
            result.is_ok(),
            "GUARD-2: prod mode must NOT panic on invalid envelope, only ERROR log"
        );
    }

    // ── valid envelope passes ─────────────────────────────────────────────────

    #[test]
    fn guard_valid_envelope_passes_dev() {
        let valid = valid_mic_taken();
        guard_outbound_with_dev_flag(&valid, true);
    }

    #[test]
    fn guard_valid_envelope_passes_prod() {
        let valid = valid_mic_taken();
        guard_outbound_with_dev_flag(&valid, false);
    }

    // ── unregistered type skips ───────────────────────────────────────────────

    /// 未注册类型（如 RoomInfoUpdated）跳过校验，不 panic
    #[test]
    fn guard_unregistered_type_skips() {
        let envelope = json!({
            "type": "RoomInfoUpdated",
            "payload": {},
            "timestamp": 123i64
        });
        guard_outbound_with_dev_flag(&envelope, true);
    }

    // ── missing type field ───────────────────────────────────────────────────

    #[test]
    fn guard_missing_type_dev_panics() {
        let envelope = json!({ "payload": {} });
        let result = std::panic::catch_unwind(|| {
            guard_outbound_with_dev_flag(&envelope, true);
        });
        assert!(result.is_err(), "missing 'type' must panic in dev mode");
    }

    #[test]
    fn guard_missing_type_prod_no_panic() {
        let envelope = json!({ "payload": {} });
        let result = std::panic::catch_unwind(|| {
            guard_outbound_with_dev_flag(&envelope, false);
        });
        assert!(result.is_ok(), "missing 'type' must not panic in prod mode");
    }
}
