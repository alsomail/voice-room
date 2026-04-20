//! AdminEvent — Redis `admin:events` 频道的事件类型定义
//!
//! 使用 internally-tagged serde：`{"type":"ban_user", ...}`
//! 未知 type 值反序列化返回 Err，不 panic。

use serde::Deserialize;
use uuid::Uuid;

// ─── 事件枚举 ─────────────────────────────────────────────────────────────────

/// 管理员通过 Redis Pub/Sub 推送的操作事件。
///
/// JSON 格式（internally-tagged，tag 字段为 `"type"`）：
/// ```json
/// {"type":"ban_user","payload":{"user_id":"..."},"admin_id":"...","ts":1234567890}
/// ```
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AdminEvent {
    BanUser {
        payload: BanUserPayload,
        admin_id: Uuid,
        ts: i64,
    },
    CloseRoom {
        payload: CloseRoomPayload,
        admin_id: Uuid,
        ts: i64,
    },
    BroadcastNotice {
        payload: BroadcastNoticePayload,
        admin_id: Uuid,
        ts: i64,
    },
}

// ─── Payload 结构体 ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BanUserPayload {
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CloseRoomPayload {
    pub room_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct BroadcastNoticePayload {
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

        let event: AdminEvent = serde_json::from_str(json)
            .expect("ban_user JSON should deserialize successfully");

        match event {
            AdminEvent::BanUser { payload, admin_id, ts } => {
                assert_eq!(
                    payload.user_id.to_string(),
                    "00000000-0000-0000-0000-000000000001",
                    "user_id should match"
                );
                assert_eq!(
                    admin_id.to_string(),
                    "00000000-0000-0000-0000-000000000099",
                    "admin_id should match"
                );
                assert_eq!(ts, 1700000000, "ts should match");
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

        let event: AdminEvent = serde_json::from_str(json)
            .expect("close_room JSON should deserialize successfully");

        match event {
            AdminEvent::CloseRoom { payload, admin_id, ts } => {
                assert_eq!(
                    payload.room_id.to_string(),
                    "00000000-0000-0000-0000-000000000002",
                    "room_id should match"
                );
                assert_eq!(
                    admin_id.to_string(),
                    "00000000-0000-0000-0000-000000000099"
                );
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
            AdminEvent::BroadcastNotice { payload, admin_id, ts } => {
                assert_eq!(payload.message, "System maintenance at 10pm");
                assert_eq!(
                    admin_id.to_string(),
                    "00000000-0000-0000-0000-000000000099"
                );
                assert_eq!(ts, 1700000002);
            }
            other => panic!("expected BroadcastNotice, got {:?}", other),
        }
    }

    // S04: 未知 type 反序列化不 panic（返回 Err，不 panic）
    #[test]
    fn s04_deserialize_unknown_event_type() {
        let json = r#"{
            "type": "nuke_everything",
            "payload": {},
            "admin_id": "00000000-0000-0000-0000-000000000099",
            "ts": 0
        }"#;

        // 未知 type 必须返回 Err，而不是 panic
        let result = serde_json::from_str::<AdminEvent>(json);
        assert!(
            result.is_err(),
            "unknown event type should return Err, not panic"
        );
    }
}
