//! 贵族续费/过期 cron（T-00068）
//!
//! Tokio 任务：每小时扫描 user_nobles，处理续费和过期。
//! 业务决策逻辑拆分为纯函数，方便单元测试。
//!
//! ## WS 信令（§10.4.2/§10.4.3/§10.4.4）
//! - `NobleRenewSuccess` — 续费成功
//! - `NobleRenewFailed`  — 续费失败（余额不足 / 连续 3 次关闭自动续费）
//! - `NobleExpired`      — 贵族过期

use std::sync::Arc;

use uuid::Uuid;

use crate::ws::ConnectionRegistry;

/// 连续失败次数达到此阈值时关闭 auto_renew
pub const MAX_FAILED_RENEW_COUNT: i32 = 3;

/// 续费决策结果
#[derive(Debug, Clone, PartialEq)]
pub enum RenewDecision {
    /// 可以续费
    CanRenew,
    /// 余额不足，递增失败计数
    InsufficientBalance { new_failed_count: i32 },
    /// 连续失败 3 次，关闭 auto_renew
    DisableAutoRenew,
}

/// 纯决策函数：根据当前 failed_count 和余额是否充足决定行动
pub fn decide_renew(current_balance: i64, monthly_diamonds: i64, failed_count: i32) -> RenewDecision {
    if current_balance >= monthly_diamonds {
        RenewDecision::CanRenew
    } else {
        let new_count = failed_count + 1;
        if new_count >= MAX_FAILED_RENEW_COUNT {
            RenewDecision::DisableAutoRenew
        } else {
            RenewDecision::InsufficientBalance {
                new_failed_count: new_count,
            }
        }
    }
}

/// 过期判断：expire_at < now
pub fn is_expired(expire_at_ms: i64, now_ms: i64) -> bool {
    expire_at_ms < now_ms
}

/// 即将过期判断：now < expire_at <= now + 24h
pub fn is_expiring_soon(expire_at_ms: i64, now_ms: i64) -> bool {
    let window_ms = 24 * 60 * 60 * 1000_i64;
    expire_at_ms > now_ms && expire_at_ms <= now_ms + window_ms
}

// ─── WS 信令构造（§10.4.2/§10.4.3/§10.4.4）─────────────────────────────────

/// 构造 `NobleRenewSuccess` WS 信令 payload（协议 §10.4.2）
///
/// 续费成功后，向用户所有连接单播此信令。
pub fn build_renew_success_signal(
    user_id: Uuid,
    tier_id: &str,
    expire_at_ms: i64,
    balance_after: i64,
) -> serde_json::Value {
    serde_json::json!({
        "type": "NobleRenewSuccess",
        "msg_id": Uuid::new_v4().to_string(),
        "payload": {
            "user_id": user_id.to_string(),
            "tier_id": tier_id,
            "expire_at": expire_at_ms,
            "balance_after": balance_after,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    })
}

/// 构造 `NobleRenewFailed` WS 信令 payload（协议 §10.4.3）
///
/// 续费失败后，向用户所有连接单播此信令。
pub fn build_renew_failed_signal(
    user_id: Uuid,
    tier_id: &str,
    reason: &str,
    failed_count: i32,
) -> serde_json::Value {
    serde_json::json!({
        "type": "NobleRenewFailed",
        "msg_id": Uuid::new_v4().to_string(),
        "payload": {
            "user_id": user_id.to_string(),
            "tier_id": tier_id,
            "reason": reason,
            "failed_count": failed_count,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    })
}

/// 构造 `NobleExpired` WS 信令 payload（协议 §10.4.4）
///
/// 贵族过期后，向用户所有连接单播此信令。
pub fn build_expired_signal(user_id: Uuid, tier_id: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "NobleExpired",
        "msg_id": Uuid::new_v4().to_string(),
        "payload": {
            "user_id": user_id.to_string(),
            "tier_id": tier_id,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    })
}

/// 向用户所有在线 WS 连接单播信令
pub fn unicast_to_user(registry: &ConnectionRegistry, user_id: Uuid, signal: &serde_json::Value) {
    if let Ok(msg) = serde_json::to_string(signal) {
        for (_, sender) in registry.get_by_user_id(user_id) {
            let _ = sender.send(msg.clone());
        }
    }
}

// ─── async cron 阶段函数（T-00068）──────────────────────────────────────────

/// 续费阶段：扫描 user_nobles，对到期且 auto_renew=true 的用户执行续费
///
/// 生产实现需要 PgPool；此处提供协议签名，供 main.rs 注册 cron 调度。
/// 实际数据库逻辑在接入真实 NobilityService 时实现。
pub async fn run_renew_phase(registry: &Arc<ConnectionRegistry>) {
    // 生产环境：扫描 DB user_nobles WHERE expire_at <= now() + 1h AND auto_renew = true
    // 对每个用户执行 decide_renew() → CanRenew / InsufficientBalance / DisableAutoRenew
    // 成功后发送 NobleRenewSuccess；失败后发送 NobleRenewFailed
    tracing::debug!(
        registry_size = registry.connections.len(),
        "nobility cron: run_renew_phase (stub - real impl requires DB)"
    );
}

/// 过期阶段：扫描 user_nobles，对已过期且宽限期结束的用户执行清除
///
/// 生产实现需要 PgPool；此处提供协议签名，供 main.rs 注册 cron 调度。
pub async fn run_expire_phase(registry: &Arc<ConnectionRegistry>) {
    // 生产环境：扫描 DB user_nobles WHERE expire_at < now() AND auto_renew = false
    // 删除记录后发送 NobleExpired 信令
    tracing::debug!(
        registry_size = registry.connections.len(),
        "nobility cron: run_expire_phase (stub - real impl requires DB)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // CR-01: 余额充足 → CanRenew
    #[test]
    fn cr01_sufficient_balance_can_renew() {
        let decision = decide_renew(500000, 300000, 0);
        assert_eq!(decision, RenewDecision::CanRenew);
    }

    // CR-02: 余额不足，failed_count=0 → InsufficientBalance(1)
    #[test]
    fn cr02_insufficient_balance_first_failure() {
        let decision = decide_renew(100000, 300000, 0);
        assert_eq!(
            decision,
            RenewDecision::InsufficientBalance { new_failed_count: 1 }
        );
    }

    // CR-03: 余额不足，failed_count=1 → InsufficientBalance(2)
    #[test]
    fn cr03_insufficient_balance_second_failure() {
        let decision = decide_renew(0, 300000, 1);
        assert_eq!(
            decision,
            RenewDecision::InsufficientBalance { new_failed_count: 2 }
        );
    }

    // CR-04: 余额不足，failed_count=2 → DisableAutoRenew（3次关闭）
    #[test]
    fn cr04_third_failure_disables_auto_renew() {
        let decision = decide_renew(0, 300000, 2);
        assert_eq!(decision, RenewDecision::DisableAutoRenew);
    }

    // CR-05: 余额恰好等于月费 → CanRenew
    #[test]
    fn cr05_exact_balance_can_renew() {
        let decision = decide_renew(300000, 300000, 0);
        assert_eq!(decision, RenewDecision::CanRenew);
    }

    // CR-06: is_expired 正确
    #[test]
    fn cr06_is_expired() {
        let now_ms = 1_000_000_000_000_i64;
        assert!(is_expired(now_ms - 1, now_ms));
        assert!(!is_expired(now_ms + 1, now_ms));
        assert!(!is_expired(now_ms, now_ms)); // 刚好到期不算 expired
    }

    // CR-07: is_expiring_soon 24h 窗口
    #[test]
    fn cr07_is_expiring_soon() {
        let now_ms = 1_000_000_000_000_i64;
        let hour_ms = 3_600_000_i64;
        // 12小时后过期 → true
        assert!(is_expiring_soon(now_ms + 12 * hour_ms, now_ms));
        // 24小时后过期 → true (expire_at == now + 24h)
        assert!(is_expiring_soon(now_ms + 24 * hour_ms, now_ms));
        // 25小时后过期 → false
        assert!(!is_expiring_soon(now_ms + 25 * hour_ms, now_ms));
        // 已过期 → false
        assert!(!is_expiring_soon(now_ms - 1, now_ms));
    }

    // CR-08: failed_count >= 3 时即使余额充足也不触发 DisableAutoRenew（余额充足优先）
    #[test]
    fn cr08_sufficient_balance_overrides_failed_count() {
        // 如果余额充足，直接 CanRenew，不管 failed_count
        let decision = decide_renew(1_000_000, 300000, 5);
        assert_eq!(decision, RenewDecision::CanRenew);
    }

    // CR-09: build_renew_success_signal — 信令名与协议 §10.4.2 一致
    #[test]
    fn cr09_renew_success_signal_has_correct_type() {
        let user_id = Uuid::new_v4();
        let signal = build_renew_success_signal(user_id, "duke", 9999999, 700000);
        assert_eq!(signal["type"], "NobleRenewSuccess");
        assert!(signal["msg_id"].is_string());
        assert_eq!(signal["payload"]["tier_id"], "duke");
        assert_eq!(signal["payload"]["balance_after"], 700000);
        assert_eq!(signal["payload"]["expire_at"], 9999999);
    }

    // CR-10: build_renew_failed_signal — 信令名与协议 §10.4.3 一致
    #[test]
    fn cr10_renew_failed_signal_has_correct_type() {
        let user_id = Uuid::new_v4();
        let signal = build_renew_failed_signal(user_id, "duke", "insufficient_balance", 1);
        assert_eq!(signal["type"], "NobleRenewFailed");
        assert_eq!(signal["payload"]["reason"], "insufficient_balance");
        assert_eq!(signal["payload"]["failed_count"], 1);
    }

    // CR-11: build_expired_signal — 信令名与协议 §10.4.4 一致
    #[test]
    fn cr11_expired_signal_has_correct_type() {
        let user_id = Uuid::new_v4();
        let signal = build_expired_signal(user_id, "knight");
        assert_eq!(signal["type"], "NobleExpired");
        assert_eq!(signal["payload"]["tier_id"], "knight");
        assert!(signal["payload"]["user_id"].is_string());
    }

    // CR-12: unicast_to_user — 已注册连接收到信令
    #[tokio::test]
    async fn cr12_unicast_to_user_delivers_signal() {
        use crate::ws::registry::{ConnectionHandle, ConnectionRegistry};
        use std::sync::{Arc, RwLock};
        use std::time::Instant;
        use tokio::sync::mpsc;

        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id,
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        let signal = build_renew_success_signal(user_id, "duke", 9999999, 700000);
        unicast_to_user(&registry, user_id, &signal);

        let msg = rx.try_recv().expect("should have received signal");
        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "NobleRenewSuccess");
    }

    // CR-13: run_renew_phase / run_expire_phase — 可被 tokio::spawn 调用
    #[tokio::test]
    async fn cr13_cron_phases_are_callable() {
        let registry = Arc::new(ConnectionRegistry::new());
        // Should not panic
        run_renew_phase(&registry).await;
        run_expire_phase(&registry).await;
    }
}
