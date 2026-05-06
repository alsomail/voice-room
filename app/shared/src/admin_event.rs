//! AdminEvent — Redis `admin:events` 频道的事件类型定义（发布端 + 消费端共享）
//!
//! PROTO-BINDING: doc/protocol/schemas/pubsub/BanUser.schema.json
//! PROTO-BINDING: doc/protocol/schemas/pubsub/UnbanUser.schema.json
//! PROTO-BINDING: doc/protocol/schemas/pubsub/CloseRoom.schema.json
//! PROTO-BINDING: doc/protocol/schemas/pubsub/BroadcastNotice.schema.json
//!
//! # 设计说明
//!
//! 使用 internally-tagged serde（`tag = "type"`），序列化为：
//! ```json
//! {"type":"ban_user","payload":{"user_id":"..."},"admin_id":"...","ts":1234567890}
//! ```
//!
//! - **发布端**（adminServer）：序列化为 JSON 写入 Redis
//! - **消费端**（server）：从 Redis 读取 JSON 反序列化
//! - 共享同一份枚举，消除双端 `event_type: String` 拼写错误的可能性
//!
//! # Schema 对齐
//!
//! 字段完全匹配 `doc/protocol/schemas/pubsub/*.schema.json`：
//! - `type`     → serde internally-tagged（`#[serde(tag = "type")]`）
//! - `payload`  → 嵌套 struct 字段
//! - `admin_id` → Uuid
//! - `ts`       → i64（epoch milliseconds）

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── 主事件枚举 ───────────────────────────────────────────────────────────────

/// 管理员通过 Redis Pub/Sub 推送的操作事件。
///
/// # 序列化格式示例
///
/// ```json
/// {"type":"ban_user","payload":{"user_id":"00000000-0000-0000-0000-000000000001"},
///  "admin_id":"00000000-0000-0000-0000-000000000099","ts":1700000000}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdminEvent {
    /// 封禁用户
    ///
    /// PROTO-BINDING: doc/protocol/schemas/pubsub/BanUser.schema.json
    BanUser {
        payload: BanUserPayload,
        admin_id: Uuid,
        ts: i64,
    },
    /// 解封用户
    ///
    /// PROTO-BINDING: doc/protocol/schemas/pubsub/UnbanUser.schema.json
    UnbanUser {
        payload: UnbanUserPayload,
        admin_id: Uuid,
        ts: i64,
    },
    /// 强制关闭房间
    ///
    /// PROTO-BINDING: doc/protocol/schemas/pubsub/CloseRoom.schema.json
    CloseRoom {
        payload: CloseRoomPayload,
        admin_id: Uuid,
        ts: i64,
    },
    /// 系统广播公告
    ///
    /// PROTO-BINDING: doc/protocol/schemas/pubsub/BroadcastNotice.schema.json
    BroadcastNotice {
        payload: BroadcastNoticePayload,
        admin_id: Uuid,
        ts: i64,
    },
}

// ─── Payload 结构体 ──────────────────────────────────────────────────────────

/// BanUser 事件载荷
///
/// PROTO-BINDING: doc/protocol/schemas/pubsub/BanUser.schema.json
/// schema: `payload.user_id` (uuid, required, additionalProperties: false)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BanUserPayload {
    /// 被封禁的用户 ID
    pub user_id: Uuid,
}

/// UnbanUser 事件载荷
///
/// PROTO-BINDING: doc/protocol/schemas/pubsub/UnbanUser.schema.json
/// schema: `payload.user_id` (uuid, required, additionalProperties: false)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnbanUserPayload {
    /// 被解封的用户 ID
    pub user_id: Uuid,
}

/// CloseRoom 事件载荷
///
/// PROTO-BINDING: doc/protocol/schemas/pubsub/CloseRoom.schema.json
/// schema: `payload.room_id` (uuid, required, additionalProperties: false)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloseRoomPayload {
    /// 被关闭的房间 ID
    pub room_id: Uuid,
}

/// BroadcastNotice 事件载荷
///
/// PROTO-BINDING: doc/protocol/schemas/pubsub/BroadcastNotice.schema.json
/// schema: `payload.message` (string, minLength=1, required, additionalProperties: false)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BroadcastNoticePayload {
    /// 公告内容（schema 要求 minLength=1，业务层负责校验）
    pub message: String,
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // S01: ban_user JSON 正确反序列化
    #[test]
    fn s01_deserialize_ban_user_event() {
        let json = r#"{
            "type": "ban_user",
            "payload": {"user_id": "00000000-0000-0000-0000-000000000001"},
            "admin_id": "00000000-0000-0000-0000-000000000099",
            "ts": 1700000000
        }"#;

        let event: AdminEvent =
            serde_json::from_str(json).expect("ban_user JSON should deserialize successfully");

        match event {
            AdminEvent::BanUser {
                payload,
                admin_id,
                ts,
            } => {
                assert_eq!(
                    payload.user_id.to_string(),
                    "00000000-0000-0000-0000-000000000001"
                );
                assert_eq!(
                    admin_id.to_string(),
                    "00000000-0000-0000-0000-000000000099"
                );
                assert_eq!(ts, 1700000000);
            }
            other => panic!("expected BanUser, got {:?}", other),
        }
    }

    // S02: close_room JSON 正确反序列化
    #[test]
    fn s02_deserialize_close_room_event() {
        let json = r#"{
            "type": "close_room",
            "payload": {"room_id": "00000000-0000-0000-0000-000000000002"},
            "admin_id": "00000000-0000-0000-0000-000000000099",
            "ts": 1700000001
        }"#;

        let event: AdminEvent =
            serde_json::from_str(json).expect("close_room JSON should deserialize successfully");

        match event {
            AdminEvent::CloseRoom {
                payload,
                admin_id,
                ts,
            } => {
                assert_eq!(
                    payload.room_id.to_string(),
                    "00000000-0000-0000-0000-000000000002"
                );
                assert_eq!(admin_id.to_string(), "00000000-0000-0000-0000-000000000099");
                assert_eq!(ts, 1700000001);
            }
            other => panic!("expected CloseRoom, got {:?}", other),
        }
    }

    // S03: broadcast_notice JSON 正确反序列化
    #[test]
    fn s03_deserialize_broadcast_notice_event() {
        let json = r#"{
            "type": "broadcast_notice",
            "payload": {"message": "System maintenance at 10pm"},
            "admin_id": "00000000-0000-0000-0000-000000000099",
            "ts": 1700000002
        }"#;

        let event: AdminEvent = serde_json::from_str(json)
            .expect("broadcast_notice JSON should deserialize successfully");

        match event {
            AdminEvent::BroadcastNotice {
                payload,
                admin_id,
                ts,
            } => {
                assert_eq!(payload.message, "System maintenance at 10pm");
                assert_eq!(admin_id.to_string(), "00000000-0000-0000-0000-000000000099");
                assert_eq!(ts, 1700000002);
            }
            other => panic!("expected BroadcastNotice, got {:?}", other),
        }
    }

    // S04: 未知 type 反序列化不 panic（返回 Err）
    #[test]
    fn s04_deserialize_unknown_event_type() {
        let json = r#"{
            "type": "nuke_everything",
            "payload": {},
            "admin_id": "00000000-0000-0000-0000-000000000099",
            "ts": 0
        }"#;

        let result = serde_json::from_str::<AdminEvent>(json);
        assert!(
            result.is_err(),
            "unknown event type should return Err, not panic"
        );
    }

    // S05: unban_user JSON 正确反序列化
    #[test]
    fn s05_deserialize_unban_user_event() {
        let json = r#"{
            "type": "unban_user",
            "payload": {"user_id": "00000000-0000-0000-0000-000000000003"},
            "admin_id": "00000000-0000-0000-0000-000000000099",
            "ts": 1700000003
        }"#;

        let event: AdminEvent =
            serde_json::from_str(json).expect("unban_user JSON should deserialize successfully");

        match event {
            AdminEvent::UnbanUser {
                payload,
                admin_id,
                ts,
            } => {
                assert_eq!(
                    payload.user_id.to_string(),
                    "00000000-0000-0000-0000-000000000003"
                );
                assert_eq!(admin_id.to_string(), "00000000-0000-0000-0000-000000000099");
                assert_eq!(ts, 1700000003);
            }
            other => panic!("expected UnbanUser, got {:?}", other),
        }
    }

    // S06: BanUser 序列化 JSON 字段精确匹配 schema（additionalProperties: false）
    #[test]
    fn s06_ban_user_serializes_to_schema_compliant_json() {
        let user_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let admin_id = Uuid::parse_str("00000000-0000-0000-0000-000000000099").unwrap();
        let event = AdminEvent::BanUser {
            payload: BanUserPayload { user_id },
            admin_id,
            ts: 1_700_000_000,
        };

        let json_str = serde_json::to_string(&event).expect("serialize must succeed");
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // 顶层字段精确匹配
        let top_keys: std::collections::BTreeSet<&str> =
            value.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        let expected: std::collections::BTreeSet<&str> =
            ["type", "payload", "admin_id", "ts"].iter().cloned().collect();
        assert_eq!(top_keys, expected, "S06: 顶层字段必须严格匹配 schema");

        // payload 字段精确匹配
        let payload_keys: std::collections::BTreeSet<&str> = value["payload"]
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        let expected_payload: std::collections::BTreeSet<&str> =
            ["user_id"].iter().cloned().collect();
        assert_eq!(
            payload_keys, expected_payload,
            "S06: payload 字段必须严格匹配 schema（仅 user_id）"
        );

        assert_eq!(value["type"].as_str(), Some("ban_user"));
        assert_eq!(
            value["payload"]["user_id"].as_str(),
            Some(user_id.to_string().as_str())
        );
        assert_eq!(
            value["admin_id"].as_str(),
            Some(admin_id.to_string().as_str())
        );
        assert_eq!(value["ts"].as_i64(), Some(1_700_000_000));
    }

    // S07: 全部 4 类事件往返相等（PartialEq 验证）
    #[test]
    fn s07_all_events_roundtrip_with_partial_eq() {
        let admin_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        let events = vec![
            AdminEvent::BanUser {
                payload: BanUserPayload { user_id },
                admin_id,
                ts: 1,
            },
            AdminEvent::UnbanUser {
                payload: UnbanUserPayload { user_id },
                admin_id,
                ts: 2,
            },
            AdminEvent::CloseRoom {
                payload: CloseRoomPayload { room_id },
                admin_id,
                ts: 3,
            },
            AdminEvent::BroadcastNotice {
                payload: BroadcastNoticePayload {
                    message: "test".to_string(),
                },
                admin_id,
                ts: 4,
            },
        ];

        for event in &events {
            let json = serde_json::to_string(event).expect("serialize");
            let back: AdminEvent = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(event, &back, "roundtrip must be identity for {:?}", event);
        }
    }

    // S08: Clone 确保独立副本（修改副本不影响原始）
    #[test]
    fn s08_clone_produces_independent_copy() {
        let original = AdminEvent::BanUser {
            payload: BanUserPayload {
                user_id: Uuid::new_v4(),
            },
            admin_id: Uuid::new_v4(),
            ts: 999,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned, "clone must equal original");
    }
}
