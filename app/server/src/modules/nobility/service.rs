//! 贵族服务抽象 + Fake 实现（T-00066）
//!
//! `NobilityServicePort` trait 供 controller 注入。
//! `FakeNobilityService` 用于单元/集成测试，无需真实 DB。

use async_trait::async_trait;
use uuid::Uuid;

use crate::common::error::AppError;
use voice_room_shared::models::nobility::{
    GiftDiscountPrivilege, MicPriorityPrivilege, MonthlyStipendPrivilege, NoblePrivileges,
};

use super::dto::{
    ListTiersResponse, MyNobleResponse, PurchaseRequest, PurchaseResponse, TierDto, UserNobleDto,
};

// ─── Port trait ───────────────────────────────────────────────────────────────

/// 贵族服务抽象：供 HTTP handler 注入，支持 Fake 测试替身
#[async_trait]
pub trait NobilityServicePort: Send + Sync {
    /// 获取所有上架 tier（Accept-Language 语言代码决定 name 字段）
    async fn list_tiers(&self, lang: &str) -> Result<ListTiersResponse, AppError>;

    /// 获取当前用户贵族状态
    async fn get_my_noble(&self, user_id: Uuid, lang: &str) -> Result<MyNobleResponse, AppError>;

    /// 钻石购买/续费/升级
    async fn purchase(
        &self,
        user_id: Uuid,
        req: PurchaseRequest,
    ) -> Result<PurchaseResponse, AppError>;

    /// 查询用户贵族（供 JoinRoom/WS 广播使用）
    async fn get_user_noble_dto(&self, user_id: Uuid) -> Option<UserNobleDto>;

    /// 切换自动续费
    async fn set_auto_renew(&self, user_id: Uuid, enabled: bool) -> Result<bool, AppError>;
}

// ─── Fake 实现 ────────────────────────────────────────────────────────────────

/// 内存测试替身：list_tiers 返回全 6 档种子；get_my_noble 返回无贵族；purchase 不真实扣钻
pub struct FakeNobilityService;

impl Default for FakeNobilityService {
    fn default() -> Self {
        Self
    }
}

fn make_fake_tier(
    tier_id: &str,
    name_en: &str,
    _name_ar: &str,
    level: i16,
    monthly_diamonds: i64,
    monthly_usd: &str,
    badge_color: &str,
) -> TierDto {
    TierDto {
        tier_id: tier_id.to_string(),
        name: name_en.to_string(),
        level,
        monthly_diamonds,
        monthly_usd: monthly_usd.to_string(),
        usd_sku_id: None,
        icon_url: format!("https://cdn.test/{tier_id}_icon.svg"),
        frame_url: format!("https://cdn.test/{tier_id}_frame.png"),
        entrance_animation_url: if level >= 3 {
            Some(format!("https://cdn.test/{tier_id}_entry.json"))
        } else {
            None
        },
        bgm_url: if level >= 2 {
            Some(format!("https://cdn.test/{tier_id}_bgm.mp3"))
        } else {
            None
        },
        badge_color: badge_color.to_string(),
        bubble_style_id: tier_id.to_string(),
        privileges: NoblePrivileges {
            badge: None,
            entry_effect: None,
            chat_bubble: None,
            audience_pin: None,
            invisibility: None,
            bypass_password: None,
            mic_priority: Some(MicPriorityPrivilege { weight: if level >= 5 { 3.0 } else { 1.0 } }),
            gift_discount: Some(GiftDiscountPrivilege { percent: (level as i64 - 1) * 2 }),
            global_broadcast: None,
            vip_support: None,
            monthly_stipend: Some(MonthlyStipendPrivilege {
                diamonds: if level == 5 { 60000 } else if level == 6 { 200000 } else { 0 },
                pay_immediately: level >= 5,
            }),
            expiry: None,
        },
    }
}

fn fake_all_tiers() -> Vec<TierDto> {
    vec![
        make_fake_tier("knight", "Knight", "فارس", 1, 3000, "9.99", "#6B7280"),
        make_fake_tier("baron", "Baron", "بارون", 2, 10000, "29.99", "#059669"),
        make_fake_tier("viscount", "Viscount", "نبيل", 3, 30000, "99.99", "#2563EB"),
        make_fake_tier("earl", "Earl", "أيرل", 4, 100000, "299.99", "#7C3AED"),
        make_fake_tier("duke", "Duke", "دوق", 5, 300000, "999.99", "#06B6D4"),
        make_fake_tier("king", "King", "ملك", 6, 1000000, "3999.99", "#DC2626"),
    ]
}

#[async_trait]
impl NobilityServicePort for FakeNobilityService {
    async fn list_tiers(&self, _lang: &str) -> Result<ListTiersResponse, AppError> {
        Ok(ListTiersResponse {
            tiers: fake_all_tiers(),
        })
    }

    async fn get_my_noble(&self, _user_id: Uuid, _lang: &str) -> Result<MyNobleResponse, AppError> {
        Ok(MyNobleResponse::none())
    }

    async fn purchase(
        &self,
        _user_id: Uuid,
        req: PurchaseRequest,
    ) -> Result<PurchaseResponse, AppError> {
        // Fake: 总是返回成功的首次购买
        let tier_dto = fake_all_tiers()
            .into_iter()
            .find(|t| t.tier_id == req.tier_id)
            .ok_or_else(|| AppError::NotFound(format!("tier {} not found", req.tier_id)))?;

        let charged = tier_dto.monthly_diamonds * req.duration_days / 30;
        Ok(PurchaseResponse {
            user_noble: MyNobleResponse::none(),
            diamonds_charged: charged,
            balance_after: 1_000_000 - charged,
            operation: "purchase".to_string(),
            upgrade_proration: None,
        })
    }

    async fn get_user_noble_dto(&self, _user_id: Uuid) -> Option<UserNobleDto> {
        None
    }

    async fn set_auto_renew(&self, _user_id: Uuid, enabled: bool) -> Result<bool, AppError> {
        Ok(enabled)
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // NS-01: FakeNobilityService list_tiers 返回 6 个 tier
    #[tokio::test]
    async fn ns01_fake_list_tiers_returns_six() {
        let svc = FakeNobilityService;
        let resp = svc.list_tiers("en-US").await.unwrap();
        assert_eq!(resp.tiers.len(), 6);
    }

    // NS-02: FakeNobilityService get_my_noble 返回无贵族（tier_id=null）
    #[tokio::test]
    async fn ns02_fake_get_my_noble_returns_none() {
        let svc = FakeNobilityService;
        let resp = svc.get_my_noble(Uuid::new_v4(), "en-US").await.unwrap();
        assert!(resp.tier_id.is_none());
    }

    // NS-03: FakeNobilityService tier 列表 level 1..6 连续
    #[tokio::test]
    async fn ns03_fake_tier_levels_are_1_to_6() {
        let svc = FakeNobilityService;
        let resp = svc.list_tiers("en-US").await.unwrap();
        let levels: Vec<i16> = resp.tiers.iter().map(|t| t.level).collect();
        assert_eq!(levels, vec![1, 2, 3, 4, 5, 6]);
    }

    // NS-04: FakeNobilityService set_auto_renew 返回请求值
    #[tokio::test]
    async fn ns04_fake_set_auto_renew_returns_value() {
        let svc = FakeNobilityService;
        let result = svc.set_auto_renew(Uuid::new_v4(), false).await.unwrap();
        assert!(!result);
    }

    // NS-05: FakeNobilityService purchase duke 充 30 天 = 300000
    #[tokio::test]
    async fn ns05_fake_purchase_duke_30d() {
        let svc = FakeNobilityService;
        let req = PurchaseRequest {
            tier_id: "duke".to_string(),
            msg_id: "msg-001".to_string(),
            auto_renew: true,
            duration_days: 30,
        };
        let resp = svc.purchase(Uuid::new_v4(), req).await.unwrap();
        assert_eq!(resp.diamonds_charged, 300000);
        assert_eq!(resp.operation, "purchase");
    }

    // NS-06: FakeNobilityService get_user_noble_dto 返回 None
    #[tokio::test]
    async fn ns06_fake_get_user_noble_dto_returns_none() {
        let svc = FakeNobilityService;
        let result = svc.get_user_noble_dto(Uuid::new_v4()).await;
        assert!(result.is_none());
    }

    // NS-07: FakeNobilityService 满足 Send + Sync + dyn 约束
    #[test]
    fn ns07_fake_is_send_sync() {
        let _svc: Arc<dyn NobilityServicePort> = Arc::new(FakeNobilityService);
    }
}
