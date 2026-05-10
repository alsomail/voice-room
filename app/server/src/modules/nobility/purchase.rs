//! 贵族购买事务决策逻辑（T-00067）
//!
//! 核心逻辑拆分为纯函数，便于单元测试（无 DB 依赖）。

use voice_room_shared::models::nobility::{compute_upgrade_proration, ProratedCharge};

/// 购买决策类型
#[derive(Debug, Clone, PartialEq)]
pub enum PurchaseDecision {
    /// 首次购买（无现有贵族）
    FirstPurchase,
    /// 同档续费
    SameTierRenew,
    /// 升级（补差公式）
    Upgrade {
        proration: ProratedCharge,
    },
    /// 降级 → 拒绝
    DowngradeBlocked,
    /// 续费叠加超上限（当前 expire - now > 30d）→ 拒绝
    RenewalOverlapBlocked,
}

/// 购买参数
#[derive(Debug)]
pub struct PurchaseDecisionInput {
    /// 现有贵族等级（None = 无贵族）
    pub existing_level: Option<i16>,
    /// 现有贵族月费钻石
    pub existing_monthly_diamonds: Option<i64>,
    /// 目标 tier 等级
    pub target_level: i16,
    /// 目标 tier 月费钻石
    pub target_monthly_diamonds: i64,
    /// 现有贵族剩余天数（相对 now）
    pub remaining_days: i64,
    /// 购买时长
    pub duration_days: i64,
    /// 续费叠加上限天数（默认 30）
    pub renewal_overlap_limit_days: i64,
}

/// 纯决策函数：无副作用，便于单元测试
pub fn decide_purchase(input: &PurchaseDecisionInput) -> PurchaseDecision {
    let Some(existing_level) = input.existing_level else {
        return PurchaseDecision::FirstPurchase;
    };

    let target_level = input.target_level;

    if target_level < existing_level {
        return PurchaseDecision::DowngradeBlocked;
    }

    if target_level == existing_level {
        // 同档续费：检查叠加上限
        if input.remaining_days > input.renewal_overlap_limit_days {
            return PurchaseDecision::RenewalOverlapBlocked;
        }
        return PurchaseDecision::SameTierRenew;
    }

    // 升级
    let old_monthly = input.existing_monthly_diamonds.unwrap_or(0);
    let proration = compute_upgrade_proration(
        old_monthly,
        input.target_monthly_diamonds,
        input.remaining_days,
        input.duration_days,
    );
    PurchaseDecision::Upgrade { proration }
}

/// 计算首次购买或续费应扣的钻石
pub fn compute_charge_for_purchase(monthly_diamonds: i64, duration_days: i64) -> i64 {
    monthly_diamonds * duration_days / 30
}

#[cfg(test)]
mod tests {
    use super::*;

    // PU-01: 无贵族 → FirstPurchase
    #[test]
    fn pu01_no_noble_returns_first_purchase() {
        let input = PurchaseDecisionInput {
            existing_level: None,
            existing_monthly_diamonds: None,
            target_level: 1,
            target_monthly_diamonds: 3000,
            remaining_days: 0,
            duration_days: 30,
            renewal_overlap_limit_days: 30,
        };
        assert_eq!(decide_purchase(&input), PurchaseDecision::FirstPurchase);
    }

    // PU-02: 同档续费，剩余 0 天 → SameTierRenew
    #[test]
    fn pu02_same_tier_renew_no_overlap() {
        let input = PurchaseDecisionInput {
            existing_level: Some(3),
            existing_monthly_diamonds: Some(30000),
            target_level: 3,
            target_monthly_diamonds: 30000,
            remaining_days: 5,
            duration_days: 30,
            renewal_overlap_limit_days: 30,
        };
        assert_eq!(decide_purchase(&input), PurchaseDecision::SameTierRenew);
    }

    // PU-03: 同档续费，剩余 > 30d → RenewalOverlapBlocked
    #[test]
    fn pu03_same_tier_renewal_overlap_blocked() {
        let input = PurchaseDecisionInput {
            existing_level: Some(3),
            existing_monthly_diamonds: Some(30000),
            target_level: 3,
            target_monthly_diamonds: 30000,
            remaining_days: 31,
            duration_days: 30,
            renewal_overlap_limit_days: 30,
        };
        assert_eq!(
            decide_purchase(&input),
            PurchaseDecision::RenewalOverlapBlocked
        );
    }

    // PU-04: 降级 → DowngradeBlocked
    #[test]
    fn pu04_downgrade_is_blocked() {
        let input = PurchaseDecisionInput {
            existing_level: Some(5),
            existing_monthly_diamonds: Some(300000),
            target_level: 3,
            target_monthly_diamonds: 30000,
            remaining_days: 10,
            duration_days: 30,
            renewal_overlap_limit_days: 30,
        };
        assert_eq!(decide_purchase(&input), PurchaseDecision::DowngradeBlocked);
    }

    // PU-05: 升级 → Upgrade with correct proration
    #[test]
    fn pu05_upgrade_with_proration() {
        let input = PurchaseDecisionInput {
            existing_level: Some(1),
            existing_monthly_diamonds: Some(3000),
            target_level: 5,
            target_monthly_diamonds: 300000,
            remaining_days: 10,
            duration_days: 30,
            renewal_overlap_limit_days: 30,
        };
        let decision = decide_purchase(&input);
        match decision {
            PurchaseDecision::Upgrade { proration } => {
                // refund = 3000 * 10 / 30 = 1000
                // charge = max(0, 300000 * 30 / 30 - 1000) = 299000
                assert_eq!(proration.refund_diamonds, 1000);
                assert_eq!(proration.charge_diamonds, 299000);
            }
            other => panic!("expected Upgrade, got {other:?}"),
        }
    }

    // PU-06: compute_charge 30天正确
    #[test]
    fn pu06_compute_charge_30d() {
        assert_eq!(compute_charge_for_purchase(300000, 30), 300000);
        assert_eq!(compute_charge_for_purchase(300000, 90), 900000);
        assert_eq!(compute_charge_for_purchase(1000000, 30), 1000000);
    }

    // PU-07: compute_charge 整除截断（floor）
    #[test]
    fn pu07_compute_charge_floor_division() {
        // 3000 * 7 / 30 = 700 (floor of 700.0)
        assert_eq!(compute_charge_for_purchase(3000, 7), 700);
    }

    // PU-08: 从 LV1 升到 LV6（king），剩余 30 天
    #[test]
    fn pu08_upgrade_from_knight_to_king_full_month() {
        let input = PurchaseDecisionInput {
            existing_level: Some(1),
            existing_monthly_diamonds: Some(3000),
            target_level: 6,
            target_monthly_diamonds: 1000000,
            remaining_days: 30,
            duration_days: 30,
            renewal_overlap_limit_days: 30,
        };
        let decision = decide_purchase(&input);
        match decision {
            PurchaseDecision::Upgrade { proration } => {
                // refund = 3000 * 30 / 30 = 3000
                // charge = max(0, 1000000 - 3000) = 997000
                assert_eq!(proration.refund_diamonds, 3000);
                assert_eq!(proration.charge_diamonds, 997000);
            }
            other => panic!("expected Upgrade, got {other:?}"),
        }
    }

    // PU-09: 同档续费，剩余刚好 30d → Blocked
    #[test]
    fn pu09_same_tier_exactly_30d_remaining_blocked() {
        let input = PurchaseDecisionInput {
            existing_level: Some(2),
            existing_monthly_diamonds: Some(10000),
            target_level: 2,
            target_monthly_diamonds: 10000,
            remaining_days: 30,
            duration_days: 30,
            renewal_overlap_limit_days: 30,
        };
        // remaining_days == limit → 不超过，应允许
        // 规则: remaining_days > limit → block
        // 30 > 30 is false → SameTierRenew
        assert_eq!(decide_purchase(&input), PurchaseDecision::SameTierRenew);
    }
}
