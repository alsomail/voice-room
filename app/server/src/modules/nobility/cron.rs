//! 贵族续费/过期 cron（T-00068）
//!
//! Tokio 任务：每小时扫描 user_nobles，处理续费和过期。
//! 业务决策逻辑拆分为纯函数，方便单元测试。

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
}
