//! Redis `admin:events` 频道下 `type=balance_updated` 事件的 payload schema
//!
//! ## 契约（单一事实源）
//! ```json
//! {
//!   "type": "balance_updated",
//!   "payload": {
//!     "user_id":       "uuid",
//!     "balance_after": 4800,
//!     "delta":         -520,
//!     "reason":        "admin_adjust",
//!     "ref_id":        "uuid|null"
//!   },
//!   "admin_id": "uuid",
//!   "ts":       1720000000
//! }
//! ```
//!
//! - `balance_after`：调整后的钻石余额（必填，i64）
//! - `delta`：本次变化量（正=加，负=扣）
//! - `reason`：原因短语（snake_case 或自定义）
//! - `ref_id`：关联业务 ID，可空（admin_logs.id / gift_records.id 等）

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `balance_updated` 事件的 `payload` 字段。
///
/// Admin Server 发布、App Server 订阅；字段名由本结构体锁定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceUpdatedEvent {
    pub user_id: Uuid,
    pub balance_after: i64,
    pub delta: i64,
    pub reason: String,
    /// 关联业务 ID（如 admin_logs.id / gift_records.id），可为 `null`。
    #[serde(default)]
    pub ref_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // BUE-01: 完整字段 round-trip
    #[test]
    fn balance_updated_event_round_trip_with_ref_id() {
        let user_id = Uuid::new_v4();
        let ref_id = Uuid::new_v4();
        let event = BalanceUpdatedEvent {
            user_id,
            balance_after: 4800,
            delta: -520,
            reason: "admin_adjust".to_string(),
            ref_id: Some(ref_id),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: BalanceUpdatedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, event);
    }

    // BUE-02: ref_id 缺失时 serde 默认为 None（防止 Admin 老消息 panic）
    #[test]
    fn balance_updated_event_ref_id_default_when_missing() {
        let json = serde_json::json!({
            "user_id": Uuid::new_v4().to_string(),
            "balance_after": 1000_i64,
            "delta": 1000_i64,
            "reason": "recharge",
        })
        .to_string();
        let parsed: BalanceUpdatedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ref_id, None);
    }

    // BUE-03: 序列化键名必须是 balance_after（不是 new_balance）
    #[test]
    fn balance_updated_event_serializes_balance_after_key() {
        let event = BalanceUpdatedEvent {
            user_id: Uuid::nil(),
            balance_after: 100,
            delta: 100,
            reason: "test".to_string(),
            ref_id: None,
        };
        let val = serde_json::to_value(&event).unwrap();
        assert!(
            val.get("balance_after").is_some(),
            "BUE-03: 字段名必须是 balance_after"
        );
        assert!(
            val.get("new_balance").is_none(),
            "BUE-03: 不允许出现旧契约 new_balance"
        );
    }

    // BUE-04: 缺失必填字段（balance_after）必须解析失败
    #[test]
    fn balance_updated_event_rejects_missing_balance_after() {
        let json = serde_json::json!({
            "user_id": Uuid::new_v4().to_string(),
            "delta": 100_i64,
            "reason": "x",
        })
        .to_string();
        let res: Result<BalanceUpdatedEvent, _> = serde_json::from_str(&json);
        assert!(
            res.is_err(),
            "BUE-04: 缺失 balance_after 必须返回 Err（fail-fast）"
        );
    }
}
