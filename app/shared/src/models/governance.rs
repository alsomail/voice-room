//! 房间治理审计模型 — T-00024
//!
//! 对应迁移脚本 `app/server/migrations/008_room_governance.sql`。
//! 包含踢人记录和禁言记录两张审计表的 Rust 模型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// RoomKickRecord — 踢人审计记录
// ─────────────────────────────────────────────────────────────────────────────

/// 踢人审计记录，对应 `room_kick_records` 表。
///
/// 当房主或管理员将某用户踢出房间时写入一条记录。
/// 外键均为 RESTRICT，软删除房间记录仍保留。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RoomKickRecord {
    /// 主键 — UUID v4，由 PostgreSQL `gen_random_uuid()` 生成。
    pub id: Uuid,

    /// 被操作的房间 ID（外键 → `rooms(id)`）。
    pub room_id: Uuid,

    /// 被踢出的用户 ID（外键 → `users(id)`）。
    pub target_user_id: Uuid,

    /// 执行踢人操作的用户 ID（外键 → `users(id)`）。
    pub operator_user_id: Uuid,

    /// 踢人原因（可选）。
    pub reason: Option<String>,

    /// 记录创建时间。
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// MuteType — 禁言类型枚举
// ─────────────────────────────────────────────────────────────────────────────

/// 禁言类型，对应 `room_mute_records.type` 列的 CHECK 约束。
///
/// - `Mic`  — 禁止上麦发言（`'mic'`）
/// - `Chat` — 禁止文字聊天（`'chat'`）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MuteType {
    Mic,
    Chat,
}

// ─────────────────────────────────────────────────────────────────────────────
// RoomMuteRecord — 禁言审计记录
// ─────────────────────────────────────────────────────────────────────────────

/// 禁言审计记录，对应 `room_mute_records` 表。
///
/// 当房主或管理员对某用户执行禁言或解除禁言时写入一条记录。
/// `duration_sec = 0` 表示解除禁言（≥ 0 由 DB CHECK 约束保证）。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RoomMuteRecord {
    /// 主键 — UUID v4，由 PostgreSQL `gen_random_uuid()` 生成。
    pub id: Uuid,

    /// 被操作的房间 ID（外键 → `rooms(id)`）。
    pub room_id: Uuid,

    /// 被禁言的用户 ID（外键 → `users(id)`）。
    pub target_user_id: Uuid,

    /// 执行禁言操作的用户 ID（外键 → `users(id)`）。
    pub operator_user_id: Uuid,

    /// 禁言类型：`mic`（禁麦）或 `chat`（禁文字）。
    #[sqlx(rename = "type")]
    pub mute_type: MuteType,

    /// 禁言时长（秒）；`0` 表示解除禁言（CHECK duration_sec >= 0）。
    pub duration_sec: i32,

    /// 禁言原因（可选）。
    pub reason: Option<String>,

    /// 记录创建时间。
    pub created_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_kick_record() -> RoomKickRecord {
        RoomKickRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            reason: Some("test reason".to_string()),
            created_at: Utc::now(),
        }
    }

    fn make_mute_record(mute_type: MuteType, duration_sec: i32) -> RoomMuteRecord {
        RoomMuteRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            mute_type,
            duration_sec,
            reason: None,
            created_at: Utc::now(),
        }
    }

    // ── RoomKickRecord ──────────────────────────────────────────────────────

    #[test]
    fn test_kick_record_clone() {
        let r = make_kick_record();
        let c = r.clone();
        assert_eq!(r.id, c.id);
        assert_eq!(r.room_id, c.room_id);
    }

    #[test]
    fn test_kick_record_debug() {
        let r = make_kick_record();
        let s = format!("{:?}", r);
        assert!(s.contains("RoomKickRecord"));
    }

    #[test]
    fn test_kick_record_serialize_deserialize() {
        let r = make_kick_record();
        let json = serde_json::to_string(&r).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["reason"], "test reason");
        assert!(v["id"].is_string());
        // round-trip
        let r2: RoomKickRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r.id, r2.id);
    }

    #[test]
    fn test_kick_record_reason_none() {
        let mut r = make_kick_record();
        r.reason = None;
        assert!(r.reason.is_none());
        let json = serde_json::to_string(&r).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v["reason"].is_null());
    }

    // ── MuteType ────────────────────────────────────────────────────────────

    #[test]
    fn test_mute_type_equality() {
        assert_eq!(MuteType::Mic, MuteType::Mic);
        assert_eq!(MuteType::Chat, MuteType::Chat);
        assert_ne!(MuteType::Mic, MuteType::Chat);
    }

    #[test]
    fn test_mute_type_clone() {
        let t = MuteType::Mic;
        let c = t.clone();
        assert_eq!(t, c);
    }

    #[test]
    fn test_mute_type_debug() {
        assert!(format!("{:?}", MuteType::Mic).contains("Mic"));
        assert!(format!("{:?}", MuteType::Chat).contains("Chat"));
    }

    #[test]
    fn test_mute_type_serialize_lowercase() {
        let mic_json = serde_json::to_string(&MuteType::Mic).unwrap();
        assert_eq!(mic_json, r#""mic""#, "MuteType::Mic should serialize as 'mic'");
        let chat_json = serde_json::to_string(&MuteType::Chat).unwrap();
        assert_eq!(chat_json, r#""chat""#, "MuteType::Chat should serialize as 'chat'");
    }

    #[test]
    fn test_mute_type_deserialize_lowercase() {
        let mic: MuteType = serde_json::from_str(r#""mic""#).unwrap();
        assert_eq!(mic, MuteType::Mic);
        let chat: MuteType = serde_json::from_str(r#""chat""#).unwrap();
        assert_eq!(chat, MuteType::Chat);
    }

    #[test]
    fn test_mute_type_rejects_sms() {
        // 'sms' 不在枚举中，反序列化应失败
        let result: Result<MuteType, _> = serde_json::from_str(r#""sms""#);
        assert!(result.is_err(), "MuteType should reject 'sms'");
    }

    // ── RoomMuteRecord ──────────────────────────────────────────────────────

    #[test]
    fn test_mute_record_mic_duration() {
        let r = make_mute_record(MuteType::Mic, 300);
        assert_eq!(r.duration_sec, 300);
        assert_eq!(r.mute_type, MuteType::Mic);
    }

    #[test]
    fn test_mute_record_duration_zero_unmute() {
        let r = make_mute_record(MuteType::Chat, 0);
        assert_eq!(r.duration_sec, 0, "0 = 解除禁言");
    }

    #[test]
    fn test_mute_record_clone() {
        let r = make_mute_record(MuteType::Chat, 120);
        let c = r.clone();
        assert_eq!(r.id, c.id);
        assert_eq!(r.duration_sec, c.duration_sec);
    }

    #[test]
    fn test_mute_record_serialize() {
        let r = make_mute_record(MuteType::Mic, 600);
        let json = serde_json::to_string(&r).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["duration_sec"], 600);
        assert_eq!(v["mute_type"], "mic");
    }

    #[test]
    fn test_mute_record_reason_optional() {
        let mut r = make_mute_record(MuteType::Chat, 60);
        assert!(r.reason.is_none());
        r.reason = Some("spam".to_string());
        assert_eq!(r.reason.as_deref(), Some("spam"));
    }
}
