//! 贵族体系共享类型（E-09 / T-00065）
//!
//! `NoblePrivileges` 严格对应 `nobility_api.md §10.2.3` JSONB schema。
//! 使用 `serde_json::Value` 字段存储嵌套对象，避免过度类型化带来的维护成本。

use serde::{Deserialize, Serialize};

/// 贵族特权 JSONB 类型（对应 noble_tiers.privileges）
///
/// 所有字段均为 Option，允许种子数据只填必填字段。
/// 服务端代码通过 `.mic_priority.weight` 等路径读取。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoblePrivileges {
    #[serde(default)]
    pub badge: Option<BadgePrivilege>,
    #[serde(default)]
    pub entry_effect: Option<EntryEffectPrivilege>,
    #[serde(default)]
    pub chat_bubble: Option<ChatBubblePrivilege>,
    #[serde(default)]
    pub audience_pin: Option<AudiencePinPrivilege>,
    #[serde(default)]
    pub invisibility: Option<InvisibilityPrivilege>,
    #[serde(default)]
    pub bypass_password: Option<BypassPasswordPrivilege>,
    #[serde(default)]
    pub mic_priority: Option<MicPriorityPrivilege>,
    #[serde(default)]
    pub gift_discount: Option<GiftDiscountPrivilege>,
    #[serde(default)]
    pub global_broadcast: Option<GlobalBroadcastPrivilege>,
    #[serde(default)]
    pub vip_support: Option<VipSupportPrivilege>,
    #[serde(default)]
    pub monthly_stipend: Option<MonthlyStipendPrivilege>,
    #[serde(default)]
    pub expiry: Option<ExpiryPrivilege>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BadgePrivilege {
    pub color: String,
    pub shape: String,
    pub animated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryEffectPrivilege {
    pub duration_ms: i64,
    pub scope: String, // marquee | half | fullscreen
    pub marquee_color: String,
    pub user_can_disable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatBubblePrivilege {
    pub style_id: String,
    pub gradient: Vec<String>,
    pub border_color: String,
    pub username_color: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudiencePinPrivilege {
    pub scope: String, // none | own_room | own_lobby | global
    pub rank_offset: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvisibilityPrivilege {
    pub scope: String, // none | mic_only | mic_and_audience | all
    pub always_visible_to: Vec<String>,
}

impl InvisibilityPrivilege {
    /// 返回是否对观众隐身（mic_and_audience 或 all）
    pub fn is_audience_invisible(&self) -> bool {
        matches!(self.scope.as_str(), "mic_and_audience" | "all")
    }

    /// 返回是否对所有人隐身（all = 仅 admin 可见）
    pub fn is_fully_invisible(&self) -> bool {
        self.scope == "all"
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BypassPasswordPrivilege {
    pub enabled: bool,
    pub respect_room_owner_switch: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicPriorityPrivilege {
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GiftDiscountPrivilege {
    pub percent: i64, // 0..100
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalBroadcastPrivilege {
    pub enabled: bool,
    pub daily_limit: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VipSupportPrivilege {
    pub sla_minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthlyStipendPrivilege {
    /// 月费钻石的返还比例（百分比，0=无月津贴，20=返还20%）— 协议 §10.2.3
    pub percent: i64,
    pub pay_immediately: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpiryPrivilege {
    pub warn_days_before: i64,
    pub grace_days: i64,
    pub history_days: i64,
}

impl NoblePrivileges {
    /// 读取 mic_priority.weight（默认 1.0）
    pub fn mic_weight(&self) -> f64 {
        self.mic_priority.as_ref().map(|m| m.weight).unwrap_or(1.0)
    }

    /// 读取 gift_discount.percent（默认 0）
    pub fn discount_percent(&self) -> i64 {
        self.gift_discount.as_ref().map(|g| g.percent).unwrap_or(0)
    }

    /// 读取 monthly_stipend.percent（默认 0）— 协议 §10.2.3
    pub fn stipend_percent(&self) -> i64 {
        self.monthly_stipend
            .as_ref()
            .map(|s| s.percent)
            .unwrap_or(0)
    }

    /// 计算月津贴实际钻石数（percent × monthly_diamonds / 100）
    ///
    /// 全程整数运算，向下取整。
    pub fn compute_stipend_diamonds(&self, monthly_diamonds: i64) -> i64 {
        self.stipend_percent() * monthly_diamonds / 100
    }

    /// 读取 bypass_password.enabled（默认 false）
    pub fn can_bypass_password(&self) -> bool {
        self.bypass_password
            .as_ref()
            .map(|b| b.enabled)
            .unwrap_or(false)
    }

    /// 读取 invisibility.scope（默认 "none"）
    pub fn invisibility_scope(&self) -> &str {
        self.invisibility
            .as_ref()
            .map(|i| i.scope.as_str())
            .unwrap_or("none")
    }

    /// 是否对观众隐身
    pub fn is_audience_invisible(&self) -> bool {
        matches!(self.invisibility_scope(), "mic_and_audience" | "all")
    }
}

/// 礼物折扣计算（T-00070）
///
/// `price_after_discount = ceil((original_price * (100 - discount_percent)) / 100 / 1000) * 1000`
///
/// 全程整数运算，禁止浮点。
pub fn compute_gift_discounted_price(original_price: i64, discount_percent: i64) -> i64 {
    if discount_percent <= 0 {
        return original_price;
    }
    let numerator = original_price * (100 - discount_percent);
    // ceil 除以 100
    let discounted = (numerator + 99) / 100;
    // ceil 取整到最近 1000
    (discounted + 999) / 1000 * 1000
}

/// 贵族升级补差计算（T-00067）
///
/// - `refund_diamonds = floor(old_monthly_diamonds × remaining_days / 30)`
/// - `charge_diamonds = max(0, new_monthly_diamonds × duration_days / 30 - refund_diamonds)`
pub fn compute_upgrade_proration(
    old_monthly_diamonds: i64,
    new_monthly_diamonds: i64,
    remaining_days: i64,
    duration_days: i64,
) -> ProratedCharge {
    let refund = old_monthly_diamonds * remaining_days / 30;
    let gross = new_monthly_diamonds * duration_days / 30;
    let charge = (gross - refund).max(0);
    ProratedCharge {
        refund_diamonds: refund,
        charge_diamonds: charge,
    }
}

/// 升级补差计算结果
#[derive(Debug, Clone, PartialEq)]
pub struct ProratedCharge {
    pub refund_diamonds: i64,
    pub charge_diamonds: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── NO65-01: NoblePrivileges 反序列化正确 ─────────────────────────────────
    #[test]
    fn no65_01_deserialize_privileges_from_json() {
        let json = r#"{
            "mic_priority": {"weight": 3.0},
            "gift_discount": {"percent": 10},
            "monthly_stipend": {"percent": 20, "pay_immediately": true},
            "bypass_password": {"enabled": true, "respect_room_owner_switch": true},
            "invisibility": {"scope": "mic_and_audience", "always_visible_to": ["admin"]}
        }"#;
        let p: NoblePrivileges = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(p.mic_weight(), 3.0);
        assert_eq!(p.discount_percent(), 10);
        assert_eq!(p.stipend_percent(), 20);
        assert!(p.can_bypass_password());
        assert!(p.is_audience_invisible());
    }

    // ── NO65-01b: monthly_stipend.percent 字段名验证（§10.2.3）────────────────
    #[test]
    fn no65_01b_monthly_stipend_uses_percent_field() {
        // king tier: percent=20, pay_immediately=true
        let json = r#"{"monthly_stipend": {"percent": 20, "pay_immediately": true}}"#;
        let p: NoblePrivileges = serde_json::from_str(json).expect("deserialize percent field");
        assert_eq!(p.stipend_percent(), 20);
        // verify serialization preserves 'percent' key (not 'diamonds')
        let serialized = serde_json::to_string(&p).unwrap();
        assert!(serialized.contains("\"percent\""), "serialized must contain 'percent' key");
        assert!(!serialized.contains("\"diamonds\""), "serialized must NOT contain 'diamonds' key");
    }

    // ── NO65-01c: duke 月津贴计算 = 15% × 300000 = 45000 ──────────────────────
    #[test]
    fn no65_01c_duke_stipend_15_percent_of_300000_equals_45000() {
        let p: NoblePrivileges = serde_json::from_str(
            r#"{"monthly_stipend": {"percent": 15, "pay_immediately": true}}"#
        ).unwrap();
        assert_eq!(p.stipend_percent(), 15);
        // 15% × 300000 = 45000
        assert_eq!(p.compute_stipend_diamonds(300000), 45000);
    }

    // ── NO65-01d: 各档 percent 值符合产品 §3.5.11 ────────────────────────────
    #[test]
    fn no65_01d_seed_percent_values_per_tier() {
        let cases = [
            (1_i16, 5_i64),   // knight
            (2, 8),   // baron
            (3, 10),  // viscount
            (4, 12),  // earl
            (5, 15),  // duke
            (6, 20),  // king
        ];
        let monthly_diamonds = [3000_i64, 10000, 30000, 100000, 300000, 1000000];
        let expected_stipends = [150_i64, 800, 3000, 12000, 45000, 200000];
        for (i, (level, percent)) in cases.iter().enumerate() {
            let p = NoblePrivileges {
                badge: None, entry_effect: None, chat_bubble: None, audience_pin: None,
                invisibility: None, bypass_password: None, mic_priority: None,
                gift_discount: None, global_broadcast: None, vip_support: None,
                monthly_stipend: Some(MonthlyStipendPrivilege { percent: *percent, pay_immediately: true }),
                expiry: None,
            };
            assert_eq!(p.stipend_percent(), *percent, "level {level} percent mismatch");
            assert_eq!(
                p.compute_stipend_diamonds(monthly_diamonds[i]),
                expected_stipends[i],
                "level {level} stipend_diamonds mismatch"
            );
        }
    }

    // ── NO65-02: NoblePrivileges 序列化含所有字段 ─────────────────────────────
    #[test]
    fn no65_02_serialize_privileges_roundtrip() {
        let p = NoblePrivileges {
            badge: None,
            entry_effect: None,
            chat_bubble: None,
            audience_pin: None,
            invisibility: Some(InvisibilityPrivilege {
                scope: "all".to_string(),
                always_visible_to: vec!["admin".to_string()],
            }),
            bypass_password: Some(BypassPasswordPrivilege {
                enabled: true,
                respect_room_owner_switch: true,
            }),
            mic_priority: Some(MicPriorityPrivilege { weight: 10.0 }),
            gift_discount: Some(GiftDiscountPrivilege { percent: 15 }),
            global_broadcast: Some(GlobalBroadcastPrivilege {
                enabled: true,
                daily_limit: 1,
            }),
            vip_support: None,
            monthly_stipend: Some(MonthlyStipendPrivilege {
                percent: 20,       // ← king tier: 20%
                pay_immediately: true,
            }),
            expiry: None,
        };
        let json = serde_json::to_string(&p).expect("serialize ok");
        let p2: NoblePrivileges = serde_json::from_str(&json).expect("deserialize ok");
        assert_eq!(p, p2);
        assert_eq!(p2.mic_weight(), 10.0);
        assert_eq!(p2.stipend_percent(), 20);
    }

    // ── NO65-03: 默认值（空 privileges）───────────────────────────────────────
    #[test]
    fn no65_03_empty_privileges_defaults() {
        let p: NoblePrivileges = serde_json::from_str("{}").expect("empty json");
        assert_eq!(p.mic_weight(), 1.0);
        assert_eq!(p.discount_percent(), 0);
        assert_eq!(p.stipend_percent(), 0);
        assert!(!p.can_bypass_password());
        assert!(!p.is_audience_invisible());
        assert_eq!(p.invisibility_scope(), "none");
    }

    // ── NO65-04: InvisibilityPrivilege scopes ────────────────────────────────
    #[test]
    fn no65_04_invisibility_scopes() {
        let none = InvisibilityPrivilege {
            scope: "none".to_string(),
            always_visible_to: vec![],
        };
        let mic_only = InvisibilityPrivilege {
            scope: "mic_only".to_string(),
            always_visible_to: vec![],
        };
        let mic_and_audience = InvisibilityPrivilege {
            scope: "mic_and_audience".to_string(),
            always_visible_to: vec![],
        };
        let all = InvisibilityPrivilege {
            scope: "all".to_string(),
            always_visible_to: vec!["admin".to_string()],
        };
        assert!(!none.is_audience_invisible());
        assert!(!mic_only.is_audience_invisible());
        assert!(mic_and_audience.is_audience_invisible());
        assert!(all.is_audience_invisible());
        assert!(!none.is_fully_invisible());
        assert!(!mic_only.is_fully_invisible());
        assert!(!mic_and_audience.is_fully_invisible());
        assert!(all.is_fully_invisible());
    }

    // ── NO67-01: 礼物折扣计算（整数，ceil 到 1000）────────────────────────────
    #[test]
    fn no67_01_gift_discount_basic() {
        // 10000 * 85% = 8500, ceil to 9000
        assert_eq!(compute_gift_discounted_price(10000, 15), 9000);
        // 10000 * 90% = 9000, ceil to 9000
        assert_eq!(compute_gift_discounted_price(10000, 10), 9000);
        // 5000 * 100% = 5000 (no discount)
        assert_eq!(compute_gift_discounted_price(5000, 0), 5000);
        // 5000 * 98% = 4900, ceil to 5000
        assert_eq!(compute_gift_discounted_price(5000, 2), 5000);
        // 30000 * 80% = 24000, ceil to 24000
        assert_eq!(compute_gift_discounted_price(30000, 20), 24000);
    }

    // ── NO67-02: 升级补差公式（5 个边界用例）────────────────────────────────
    #[test]
    fn no67_02_upgrade_proration_boundary_cases() {
        // Case 1: 从 knight 升 duke，剩余 10 天，购买 30 天
        // refund = 3000 * 10 / 30 = 1000
        // charge = max(0, 300000 * 30 / 30 - 1000) = 299000
        let r1 = compute_upgrade_proration(3000, 300000, 10, 30);
        assert_eq!(r1.refund_diamonds, 1000);
        assert_eq!(r1.charge_diamonds, 299000);

        // Case 2: 从 duke 升 king，剩余 30 天，购买 30 天
        // refund = 300000 * 30 / 30 = 300000
        // charge = max(0, 1000000 * 30 / 30 - 300000) = 700000
        let r2 = compute_upgrade_proration(300000, 1000000, 30, 30);
        assert_eq!(r2.refund_diamonds, 300000);
        assert_eq!(r2.charge_diamonds, 700000);

        // Case 3: 剩余 0 天（刚到期）
        // refund = 300000 * 0 / 30 = 0
        // charge = 1000000 * 30 / 30 = 1000000
        let r3 = compute_upgrade_proration(300000, 1000000, 0, 30);
        assert_eq!(r3.refund_diamonds, 0);
        assert_eq!(r3.charge_diamonds, 1000000);

        // Case 4: 剩余 15 天，购买 90 天
        // refund = 3000 * 15 / 30 = 1500
        // charge = max(0, 300000 * 90 / 30 - 1500) = 898500
        let r4 = compute_upgrade_proration(3000, 300000, 15, 90);
        assert_eq!(r4.refund_diamonds, 1500);
        assert_eq!(r4.charge_diamonds, 898500);

        // Case 5: 剩余极大值（charge 可能为 0）
        // refund = 1000000 * 365 / 30 = 12166666
        // new_charge = 1000000 * 30 / 30 = 1000000
        // charge = max(0, 1000000 - 12166666) = 0
        let r5 = compute_upgrade_proration(1000000, 1000000, 365, 30);
        assert_eq!(r5.charge_diamonds, 0);
    }

    // ── NO67-03: 折扣计算无浮点溢出 ──────────────────────────────────────────
    #[test]
    fn no67_03_discount_no_float_overflow() {
        // 大额礼物
        let price = 1_000_000_i64;
        let result = compute_gift_discounted_price(price, 15);
        // 1000000 * 85 / 100 = 850000, ceil to 1000 = 850000
        assert_eq!(result, 850000);
        // 整数运算，结果确定
        assert!(result > 0 && result < price);
    }
}
