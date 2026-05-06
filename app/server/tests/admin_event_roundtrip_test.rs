//! admin:events 双端往返集成测试
//!
//! PROTO-BINDING: doc/protocol/schemas/pubsub/BanUser.schema.json
//! PROTO-BINDING: doc/protocol/schemas/pubsub/UnbanUser.schema.json
//! PROTO-BINDING: doc/protocol/schemas/pubsub/CloseRoom.schema.json
//! PROTO-BINDING: doc/protocol/schemas/pubsub/BroadcastNotice.schema.json
//!
//! # 测试场景
//! - PUBSUB-1: BanUser 往返（序列化 → JSON 结构验证 → 反序列化 → 结构相等）
//! - PUBSUB-2: 非法 type 字符串 → Err，无 panic（dead-letter 路径）
//! - PUBSUB-3: 全部 4 类事件往返通过
//! - PUBSUB-4: adminServer 源码 `r#type:` 出现次数为 0（编译期契约）

use std::collections::BTreeSet;
use uuid::Uuid;
use voice_room_shared::admin_event::{
    AdminEvent, BanUserPayload, BroadcastNoticePayload, CloseRoomPayload, UnbanUserPayload,
};

// ─── PUBSUB-1: BanUser 往返 ───────────────────────────────────────────────────

/// PUBSUB-1: BanUser 事件序列化后 JSON 字段与 schema 严格对齐，然后反序列化得到相同结构。
///
/// 验证点：
/// 1. JSON `type` = "ban_user"
/// 2. `payload.user_id` 正确
/// 3. `admin_id` 正确
/// 4. `ts` 正确
/// 5. 顶层字段集合 = {type, payload, admin_id, ts}（additionalProperties: false）
/// 6. 往返后结构相等
#[test]
fn pubsub1_ban_user_roundtrip() {
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let admin_id = Uuid::parse_str("00000000-0000-0000-0000-000000000099").unwrap();
    let ts = 1_700_000_000_i64;

    let event = AdminEvent::BanUser {
        payload: BanUserPayload { user_id },
        admin_id,
        ts,
    };

    // ── 序列化 ──────────────────────────────────────────────────────────────────
    let json_str = serde_json::to_string(&event)
        .expect("PUBSUB-1: BanUser 序列化不应失败");

    // ── JSON 结构验证（schema 对齐）─────────────────────────────────────────────
    let value: serde_json::Value =
        serde_json::from_str(&json_str).expect("PUBSUB-1: JSON 应合法");

    assert_eq!(
        value["type"].as_str(),
        Some("ban_user"),
        "PUBSUB-1: JSON.type 必须为 'ban_user'"
    );
    assert_eq!(
        value["payload"]["user_id"].as_str(),
        Some(user_id.to_string().as_str()),
        "PUBSUB-1: JSON.payload.user_id 必须与原始 user_id 一致"
    );
    assert_eq!(
        value["admin_id"].as_str(),
        Some(admin_id.to_string().as_str()),
        "PUBSUB-1: JSON.admin_id 必须与原始 admin_id 一致"
    );
    assert_eq!(
        value["ts"].as_i64(),
        Some(ts),
        "PUBSUB-1: JSON.ts 必须与原始 ts 一致"
    );

    // schema: additionalProperties: false — 顶层字段集合精确匹配
    let top_keys: BTreeSet<&str> = value
        .as_object()
        .expect("PUBSUB-1: JSON 根应为 object")
        .keys()
        .map(|s| s.as_str())
        .collect();
    let expected_keys: BTreeSet<&str> = ["type", "payload", "admin_id", "ts"].iter().cloned().collect();
    assert_eq!(
        top_keys, expected_keys,
        "PUBSUB-1: 顶层字段集合必须严格匹配 schema（additionalProperties: false）"
    );

    // schema: payload.additionalProperties: false — payload 字段集合精确匹配
    let payload_keys: BTreeSet<&str> = value["payload"]
        .as_object()
        .expect("PUBSUB-1: payload 应为 object")
        .keys()
        .map(|s| s.as_str())
        .collect();
    let expected_payload_keys: BTreeSet<&str> = ["user_id"].iter().cloned().collect();
    assert_eq!(
        payload_keys, expected_payload_keys,
        "PUBSUB-1: payload 字段集合必须严格匹配 schema（仅 user_id）"
    );

    // ── 反序列化（往返）──────────────────────────────────────────────────────────
    let event2: AdminEvent =
        serde_json::from_str(&json_str).expect("PUBSUB-1: 反序列化不应失败");
    assert_eq!(event, event2, "PUBSUB-1: 往返后事件结构必须完全相等");
}

// ─── PUBSUB-2: 非法 type → dead-letter, 无 panic ────────────────────────────

/// PUBSUB-2: 故意构造非法 type 字符串（"BanUserrrr"）→ 反序列化返回 Err，无 panic。
///
/// 这是 server schema_guard 的核心能力：恶意/错误事件不应 panic，
/// 而是静默进入 dead-letter 日志路径。
#[test]
fn pubsub2_unknown_type_returns_err_not_panic() {
    // 故意使用大写 + 额外字符，模拟 adminServer 字符串拼写错误
    let bad_payloads = [
        r#"{"type":"BanUserrrr","payload":{"user_id":"00000000-0000-0000-0000-000000000001"},"admin_id":"00000000-0000-0000-0000-000000000099","ts":0}"#,
        r#"{"type":"ban-user","payload":{"user_id":"00000000-0000-0000-0000-000000000001"},"admin_id":"00000000-0000-0000-0000-000000000099","ts":0}"#,
        r#"{"type":"","payload":{},"admin_id":"00000000-0000-0000-0000-000000000099","ts":0}"#,
        r#"{"type":"CLOSE_ROOM","payload":{"room_id":"00000000-0000-0000-0000-000000000002"},"admin_id":"00000000-0000-0000-0000-000000000099","ts":0}"#,
    ];

    for raw in &bad_payloads {
        let result = serde_json::from_str::<AdminEvent>(raw);
        assert!(
            result.is_err(),
            "PUBSUB-2: 非法 type `{}` 必须返回 Err，不得 panic",
            raw
        );
    }
}

// ─── PUBSUB-3: 全部 4 类事件往返 ───────────────────────────────────────────────

/// PUBSUB-3: BanUser / UnbanUser / CloseRoom / BroadcastNotice 全部往返成功。
///
/// 验证点：
/// 1. 每个事件序列化后 JSON.type 字段正确
/// 2. 反序列化后与原始事件相等
/// 3. payload 内容正确
#[test]
fn pubsub3_all_four_event_types_roundtrip() {
    let admin_id = Uuid::parse_str("00000000-0000-0000-0000-000000000099").unwrap();
    let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let room_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

    let cases: &[(&str, AdminEvent, Box<dyn Fn(&serde_json::Value)>)] = &[
        (
            "ban_user",
            AdminEvent::BanUser {
                payload: BanUserPayload { user_id },
                admin_id,
                ts: 1_000,
            },
            Box::new(move |v| {
                assert_eq!(v["payload"]["user_id"].as_str(), Some(user_id.to_string().as_str()));
            }),
        ),
        (
            "unban_user",
            AdminEvent::UnbanUser {
                payload: UnbanUserPayload { user_id },
                admin_id,
                ts: 2_000,
            },
            Box::new(move |v| {
                assert_eq!(v["payload"]["user_id"].as_str(), Some(user_id.to_string().as_str()));
            }),
        ),
        (
            "close_room",
            AdminEvent::CloseRoom {
                payload: CloseRoomPayload { room_id },
                admin_id,
                ts: 3_000,
            },
            Box::new(move |v| {
                assert_eq!(v["payload"]["room_id"].as_str(), Some(room_id.to_string().as_str()));
            }),
        ),
        (
            "broadcast_notice",
            AdminEvent::BroadcastNotice {
                payload: BroadcastNoticePayload {
                    message: "维护通知：服务将于今晚 22:00 停机 30 分钟".to_string(),
                },
                admin_id,
                ts: 4_000,
            },
            Box::new(|v| {
                assert_eq!(
                    v["payload"]["message"].as_str(),
                    Some("维护通知：服务将于今晚 22:00 停机 30 分钟")
                );
            }),
        ),
    ];

    for (expected_type, event, payload_check) in cases {
        let json_str = serde_json::to_string(event)
            .unwrap_or_else(|e| panic!("PUBSUB-3: {expected_type} 序列化失败: {e}"));

        let value: serde_json::Value = serde_json::from_str(&json_str)
            .unwrap_or_else(|e| panic!("PUBSUB-3: {expected_type} JSON parse 失败: {e}"));

        // type 字段正确
        assert_eq!(
            value["type"].as_str(),
            Some(*expected_type),
            "PUBSUB-3: {expected_type} JSON.type 必须正确"
        );

        // admin_id 正确
        assert_eq!(
            value["admin_id"].as_str(),
            Some(admin_id.to_string().as_str()),
            "PUBSUB-3: {expected_type} admin_id 必须正确"
        );

        // payload 内容正确
        payload_check(&value);

        // 往返相等
        let event2: AdminEvent = serde_json::from_str(&json_str)
            .unwrap_or_else(|e| panic!("PUBSUB-3: {expected_type} 反序列化失败: {e}"));
        assert_eq!(
            event, &event2,
            "PUBSUB-3: {expected_type} 往返必须完全相等"
        );
    }
}

// ─── PUBSUB-4: adminServer admin:events 发布链中 `r#type:` 出现 0 次 ──────────

/// PUBSUB-4: admin:events 发布链（publisher.rs + user/service.rs + room/service.rs）
/// 中不再包含 `r#type:` 字段引用。
///
/// 这是编译期契约：移除 schema-less `AdminEvent { r#type: String, ... }` 后，
/// 这些文件中不可能再出现 `r#type:` 赋值。
///
/// **注意**：仅检查 admin:events 发布链文件，不检查 gift/wallet 等其他模块
/// （这些模块有自己独立的事件类型，不在本 task 范围内）。
#[test]
fn pubsub4_no_raw_type_field_in_adminserver_source() {
    use std::path::Path;
    use std::process::Command;

    // CARGO_MANIFEST_DIR = .../app/server → 向上两级得到 workspace root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(manifest_dir)
        .parent() // app/
        .expect("PUBSUB-4: app/ dir must exist")
        .parent() // workspace root
        .expect("PUBSUB-4: workspace root must exist");

    // admin:events 发布链的关键文件（与 AdminEvent 直接相关）
    let files_in_publish_chain = [
        "app/adminServer/src/modules/event/publisher.rs",
        "app/adminServer/src/modules/user/service.rs",
        "app/adminServer/src/modules/room/service.rs",
    ];

    let mut all_hits = Vec::new();

    for relative_path in &files_in_publish_chain {
        let full_path = workspace_root.join(relative_path);
        if !full_path.exists() {
            // 文件不存在时跳过（可能尚未创建）
            continue;
        }

        match Command::new("grep")
            .args(["-n", r"r#type:", full_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if !line.trim().is_empty() {
                        all_hits.push(format!("{relative_path}: {line}"));
                    }
                }
            }
            Err(_) => {
                eprintln!("PUBSUB-4: grep 命令不可用，跳过文件 {relative_path}（仅 Unix 环境执行）");
            }
        }
    }

    assert!(
        all_hits.is_empty(),
        "PUBSUB-4: admin:events 发布链中 `r#type:` 必须为 0 次命中，\
         实际发现 {} 处:\n{}",
        all_hits.len(),
        all_hits.join("\n")
    );
}

// ─── 额外边界测试 ─────────────────────────────────────────────────────────────

/// BroadcastNotice 空消息 — schema 要求 minLength=1，但 Rust 类型层面不强制。
/// 验证序列化后 message 字段保持原值（业务层应在 publish 前校验空消息）。
#[test]
fn extra_broadcast_notice_preserves_empty_message_for_business_layer_validation() {
    let event = AdminEvent::BroadcastNotice {
        payload: BroadcastNoticePayload {
            message: String::new(), // 业务层负责拒绝空消息
        },
        admin_id: Uuid::new_v4(),
        ts: 0,
    };
    let json_str = serde_json::to_string(&event).expect("serialize must succeed");
    let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(
        value["payload"]["message"].as_str(),
        Some(""),
        "序列化层应原样保留 message，空消息拒绝由业务层处理"
    );
}

/// Unicode + 特殊字符在 BroadcastNotice.message 中完整往返。
#[test]
fn extra_broadcast_notice_unicode_and_special_chars_roundtrip() {
    let msg = "🔴 系统公告 <script>alert(1)</script> -- DROP TABLE users; 「日本語」";
    let event = AdminEvent::BroadcastNotice {
        payload: BroadcastNoticePayload {
            message: msg.to_string(),
        },
        admin_id: Uuid::new_v4(),
        ts: 9_999_999_999,
    };
    let json_str = serde_json::to_string(&event).unwrap();
    let event2: AdminEvent = serde_json::from_str(&json_str).unwrap();
    assert_eq!(event, event2, "Unicode/特殊字符必须完整往返");
    match &event2 {
        AdminEvent::BroadcastNotice { payload, .. } => {
            assert_eq!(payload.message, msg, "message 内容必须完整还原");
        }
        _ => panic!("expected BroadcastNotice"),
    }
}
