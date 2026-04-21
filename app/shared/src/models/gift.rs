use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 礼物配置表数据库模型
///
/// 对应 `app/server/migrations/005_create_gifts.sql`。
/// 见 `doc/tds/server/T-00019.md`。
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GiftModel {
    /// 主键 — UUID v4，由 PostgreSQL `gen_random_uuid()` 生成
    pub id: Uuid,

    /// 稳定业务标识符，如 `rose_01`（唯一约束）
    pub code: String,

    /// 英文名称（最长 64 字符）
    pub name_en: String,

    /// 阿拉伯文名称（最长 64 字符）
    pub name_ar: String,

    /// 礼物图标 URL
    pub icon_url: String,

    /// 价格（钻石数，≥ 1）
    pub price: i64,

    /// 档位 1~5（1=基础，5=顶级）
    pub tier: i16,

    /// 特效等级：1=无 2=局部 3=底部 4=全屏 5=全屏+边框
    pub effect_level: i16,

    /// 动效资源 URL（可为 None）
    pub animation_url: Option<String>,

    /// 显示排序权重（同 tier 内升序）
    pub sort_order: i32,

    /// 是否上架（false 则不返回给客户端）
    pub is_active: bool,

    /// 软删除标记
    pub is_deleted: bool,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::Value;
    use uuid::Uuid;

    fn make_gift() -> GiftModel {
        GiftModel {
            id: Uuid::new_v4(),
            code: "rose_01".to_string(),
            name_en: "Rose".to_string(),
            name_ar: "وردة".to_string(),
            icon_url: "/assets/gifts/rose.png".to_string(),
            price: 1,
            tier: 1,
            effect_level: 1,
            animation_url: None,
            sort_order: 10,
            is_active: true,
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // GM01: GiftModel 可构造并克隆
    #[test]
    fn gm01_gift_model_clone() {
        let gift = make_gift();
        let cloned = gift.clone();
        assert_eq!(gift.id, cloned.id);
        assert_eq!(gift.code, cloned.code);
        assert_eq!(gift.name_en, cloned.name_en);
        assert_eq!(gift.name_ar, cloned.name_ar);
    }

    // GM02: GiftModel 可 Debug 打印
    #[test]
    fn gm02_gift_model_debug() {
        let gift = make_gift();
        let debug_str = format!("{:?}", gift);
        assert!(debug_str.contains("GiftModel"));
        assert!(debug_str.contains("rose_01"));
    }

    // GM03: GiftModel 可序列化为 JSON
    #[test]
    fn gm03_gift_model_serialize() {
        let gift = make_gift();
        let json = serde_json::to_string(&gift).expect("serialize should succeed");
        let v: Value = serde_json::from_str(&json).unwrap();
        assert!(v["id"].is_string());
        assert_eq!(v["code"], "rose_01");
        assert_eq!(v["name_en"], "Rose");
        assert_eq!(v["name_ar"], "وردة");
        assert_eq!(v["price"], 1);
        assert_eq!(v["tier"], 1);
        assert_eq!(v["effect_level"], 1);
        assert_eq!(v["sort_order"], 10);
        assert!(v["is_active"].as_bool().unwrap());
        assert!(!v["is_deleted"].as_bool().unwrap());
        assert!(v["animation_url"].is_null());
    }

    // GM04: animation_url 支持 Some 和 None
    #[test]
    fn gm04_gift_model_animation_url_nullable() {
        let mut gift = make_gift();
        assert!(gift.animation_url.is_none());
        gift.animation_url = Some("https://cdn.example.com/anim.mp4".to_string());
        assert!(gift.animation_url.is_some());
    }

    // GM05: price 类型为 i64 — 支持大额礼物
    #[test]
    fn gm05_gift_model_price_type_i64() {
        let gift = make_gift();
        assert_eq!(gift.price, 1i64);
        // 验证 i64 范围（1314 钻石礼物）
        let diamond = GiftModel {
            price: 1314,
            ..make_gift()
        };
        assert_eq!(diamond.price, 1314i64);
    }

    // GM06: tier 范围 1~5
    #[test]
    fn gm06_gift_model_tier_range() {
        for tier in 1i16..=5i16 {
            let gift = GiftModel { tier, ..make_gift() };
            assert!(gift.tier >= 1 && gift.tier <= 5);
        }
    }

    // GM07: GiftModel 从 JSON 反序列化
    #[test]
    fn gm07_gift_model_deserialize() {
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "code": "camel_01",
            "name_en": "Desert Camel",
            "name_ar": "جمل",
            "icon_url": "/assets/gifts/camel.png",
            "price": 66,
            "tier": 3,
            "effect_level": 3,
            "animation_url": null,
            "sort_order": 30,
            "is_active": true,
            "is_deleted": false,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;
        let gift: GiftModel = serde_json::from_str(json).expect("deserialize should succeed");
        assert_eq!(gift.code, "camel_01");
        assert_eq!(gift.name_ar, "جمل");
        assert_eq!(gift.price, 66);
        assert_eq!(gift.tier, 3);
    }

    // GM08: GiftModel 支持 Unicode 和阿拉伯字符
    #[test]
    fn gm08_gift_model_supports_arabic_unicode() {
        let gift = GiftModel {
            name_ar: "خاتم الماس 💎".to_string(),
            ..make_gift()
        };
        assert!(gift.name_ar.contains('💎'));
        assert!(gift.name_ar.contains('خ'));
    }

    // GM09: is_active=false 礼物可构造（下架场景）
    #[test]
    fn gm09_gift_model_inactive_constructible() {
        let gift = GiftModel {
            is_active: false,
            ..make_gift()
        };
        assert!(!gift.is_active);
    }

    // GM10: is_deleted=true 礼物可构造（软删除场景）
    #[test]
    fn gm10_gift_model_deleted_constructible() {
        let gift = GiftModel {
            is_deleted: true,
            ..make_gift()
        };
        assert!(gift.is_deleted);
    }
}
