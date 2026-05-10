//! 贵族特权钩子（T-00070）
//!
//! 纯函数实现，全部由 privileges JSONB 配置驱动，修改种子数据无需改代码。
//! - 隐身过滤
//! - 优先抢麦权重
//! - 礼物折扣
//! - 密码房免密

use voice_room_shared::models::nobility::{compute_gift_discounted_price, NoblePrivileges};

// ─── 隐身过滤（T-00070 §1） ──────────────────────────────────────────────────

/// 判断该贵族用户在房间成员列表中是否对指定观看者可见
///
/// - `viewer_is_admin_or_owner` — 管理员/房主始终可见
/// - `viewer_is_on_mic` — 麦上用户对 mic_and_audience 可见
/// - 隐身 scope: none → 全可见, mic_only → 仅过滤麦位, mic_and_audience → 观众不可见, all → 仅 admin 可见
pub fn is_visible_to_viewer(
    privileges: &NoblePrivileges,
    viewer_is_admin_or_owner: bool,
    viewer_is_on_mic: bool,
) -> bool {
    if viewer_is_admin_or_owner {
        return true;
    }
    match privileges.invisibility_scope() {
        "none" => true,
        "mic_only" => true, // 对观众可见，只有麦位上的自己隐身
        "mic_and_audience" => {
            // 仅房主/Admin/麦上用户可见
            viewer_is_on_mic
        }
        "all" => false, // 仅 admin 可见（admin 已在上面返回 true）
        _ => true,
    }
}

// ─── 礼物折扣（T-00070 §3） ──────────────────────────────────────────────────

/// 计算礼物折扣后实际扣款
///
/// - `original_price`: 礼物原价
/// - 发送者实际扣 `discounted_price`，接收者得 `original_price`
/// - 差值 = `original_price - discounted_price` 由平台补贴
pub fn apply_gift_discount(privileges: &NoblePrivileges, original_price: i64) -> GiftDiscountResult {
    let percent = privileges.discount_percent();
    if percent <= 0 {
        return GiftDiscountResult {
            discounted_price: original_price,
            subsidy_amount: 0,
        };
    }
    let discounted = compute_gift_discounted_price(original_price, percent);
    GiftDiscountResult {
        discounted_price: discounted,
        subsidy_amount: original_price - discounted,
    }
}

/// 礼物折扣计算结果
#[derive(Debug, Clone, PartialEq)]
pub struct GiftDiscountResult {
    /// 发送者实际扣款
    pub discounted_price: i64,
    /// 平台补贴金额（原价 - 折扣价）
    pub subsidy_amount: i64,
}

// ─── 密码房免密（T-00070 §4） ────────────────────────────────────────────────

/// 判断贵族用户是否可以跳过密码验证
///
/// `respect_room_owner_switch` = true 且房主已关闭该特权时，仍需要密码。
/// 本函数仅判断贵族自身特权，房主开关需外层额外处理。
pub fn can_bypass_password(privileges: &NoblePrivileges) -> bool {
    privileges.can_bypass_password()
}

// ─── 优先抢麦权重（T-00070 §2） ──────────────────────────────────────────────

/// 获取贵族的抢麦权重（用于 Lua/Rust 端加权抽签）
///
/// 默认权重 = 1.0；duke = 3.0；king = 10.0
pub fn mic_priority_weight(privileges: &NoblePrivileges) -> f64 {
    privileges.mic_weight()
}

// ─── 全服广播条件（T-00069 §4） ──────────────────────────────────────────────

/// 判断贵族是否可以触发全服 NobleEntranceGlobal（LV5+ duke/king）
pub fn can_trigger_global_broadcast(level: i16) -> bool {
    level >= 5
}

#[cfg(test)]
mod tests {
    use super::*;
    use voice_room_shared::models::nobility::{
        BypassPasswordPrivilege, GiftDiscountPrivilege, InvisibilityPrivilege,
        MicPriorityPrivilege, MonthlyStipendPrivilege,
    };

    fn make_privileges(
        invisibility_scope: &str,
        discount_percent: i64,
        mic_weight: f64,
        bypass: bool,
    ) -> NoblePrivileges {
        NoblePrivileges {
            badge: None,
            entry_effect: None,
            chat_bubble: None,
            audience_pin: None,
            invisibility: Some(InvisibilityPrivilege {
                scope: invisibility_scope.to_string(),
                always_visible_to: vec!["admin".to_string()],
            }),
            bypass_password: Some(BypassPasswordPrivilege {
                enabled: bypass,
                respect_room_owner_switch: true,
            }),
            mic_priority: Some(MicPriorityPrivilege { weight: mic_weight }),
            gift_discount: Some(GiftDiscountPrivilege {
                percent: discount_percent,
            }),
            global_broadcast: None,
            vip_support: None,
            monthly_stipend: Some(MonthlyStipendPrivilege {
                diamonds: 0,
                pay_immediately: false,
            }),
            expiry: None,
        }
    }

    // T70-01: 隐身 scope=none → 所有人可见
    #[test]
    fn t70_01_invisible_none_visible_to_all() {
        let p = make_privileges("none", 0, 1.0, false);
        assert!(is_visible_to_viewer(&p, false, false)); // 普通观众
        assert!(is_visible_to_viewer(&p, false, true)); // 麦上
        assert!(is_visible_to_viewer(&p, true, false)); // 管理员
    }

    // T70-02: 隐身 scope=mic_and_audience → 观众不可见，管理员/麦上可见
    #[test]
    fn t70_02_invisible_mic_and_audience() {
        let p = make_privileges("mic_and_audience", 0, 1.0, true);
        assert!(!is_visible_to_viewer(&p, false, false)); // 普通观众 → 不可见
        assert!(is_visible_to_viewer(&p, false, true)); // 麦上 → 可见
        assert!(is_visible_to_viewer(&p, true, false)); // 管理员 → 可见
    }

    // T70-03: 隐身 scope=all → 仅管理员可见
    #[test]
    fn t70_03_invisible_all_only_admin() {
        let p = make_privileges("all", 0, 1.0, true);
        assert!(!is_visible_to_viewer(&p, false, false)); // 普通观众 → 不可见
        assert!(!is_visible_to_viewer(&p, false, true)); // 麦上 → 不可见
        assert!(is_visible_to_viewer(&p, true, false)); // 管理员 → 可见
    }

    // T70-04: 礼物折扣 0% → 无折扣
    #[test]
    fn t70_04_gift_discount_zero_percent() {
        let p = make_privileges("none", 0, 1.0, false);
        let result = apply_gift_discount(&p, 10000);
        assert_eq!(result.discounted_price, 10000);
        assert_eq!(result.subsidy_amount, 0);
    }

    // T70-05: 礼物折扣 10% → 9000，补贴 1000（ceil 到 1000）
    #[test]
    fn t70_05_gift_discount_10_percent() {
        let p = make_privileges("none", 10, 1.0, false);
        // 10000 * (100-10)/100 = 9000, ceil 1000 = 9000
        let result = apply_gift_discount(&p, 10000);
        assert_eq!(result.discounted_price, 9000);
        assert_eq!(result.subsidy_amount, 1000);
    }

    // T70-06: 礼物折扣 15% → 精确整数（原价 10000）
    #[test]
    fn t70_06_gift_discount_15_percent_precision() {
        let p = make_privileges("none", 15, 1.0, false);
        // 10000 * 85 / 100 = 8500, ceil(8500/1000)*1000 = 9000
        let result = apply_gift_discount(&p, 10000);
        assert_eq!(result.discounted_price, 9000);
        assert_eq!(result.subsidy_amount, 1000);
    }

    // T70-07: 密码房免密：duke → 可跳过
    #[test]
    fn t70_07_duke_bypass_password() {
        let p = make_privileges("mic_and_audience", 10, 3.0, true);
        assert!(can_bypass_password(&p));
    }

    // T70-08: 密码房免密：knight → 不可跳过
    #[test]
    fn t70_08_knight_cannot_bypass() {
        let p = make_privileges("none", 0, 1.0, false);
        assert!(!can_bypass_password(&p));
    }

    // T70-09: 抢麦权重：duke = 3.0
    #[test]
    fn t70_09_duke_mic_weight() {
        let p = make_privileges("mic_and_audience", 10, 3.0, true);
        assert_eq!(mic_priority_weight(&p), 3.0);
    }

    // T70-10: 抢麦权重：knight = 1.0
    #[test]
    fn t70_10_knight_mic_weight() {
        let p = make_privileges("none", 0, 1.0, false);
        assert_eq!(mic_priority_weight(&p), 1.0);
    }

    // T70-11: 全服广播条件
    #[test]
    fn t70_11_global_broadcast_condition() {
        assert!(!can_trigger_global_broadcast(1)); // knight
        assert!(!can_trigger_global_broadcast(4)); // earl
        assert!(can_trigger_global_broadcast(5)); // duke
        assert!(can_trigger_global_broadcast(6)); // king
    }

    // T70-12: 礼物折扣计算无浮点误差（大额）
    #[test]
    fn t70_12_discount_large_price_no_float_error() {
        let p = make_privileges("none", 15, 1.0, false);
        // 30000 * 85 / 100 = 25500, ceil(25500/1000)*1000 = 26000
        let result = apply_gift_discount(&p, 30000);
        assert_eq!(result.discounted_price, 26000);
        assert_eq!(result.subsidy_amount, 4000);
    }
}
