use serde::{Deserialize, Serialize};
use voice_room_shared::models::gift::GiftModel;

// ─── 请求 DTO ─────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/gifts 查询参数
#[derive(Debug, Deserialize)]
pub struct ListGiftsQuery {
    /// 是否包含未上架礼物（默认 false）
    pub include_inactive: Option<bool>,
    /// 页码（默认 1）
    pub page: Option<i64>,
    /// 每页条数（默认 20，最大 100）
    pub size: Option<i64>,
}

/// POST /api/v1/admin/gifts 请求体
#[derive(Debug, Deserialize)]
pub struct CreateGiftRequest {
    pub code: String,
    pub name_en: String,
    pub name_ar: String,
    pub icon_url: String,
    pub price: i64,
    pub tier: i16,
    pub effect_level: Option<i16>,
    pub animation_url: Option<String>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

/// PUT /api/v1/admin/gifts/:id 请求体（全部可选，只更新传入的字段）
#[derive(Debug, Deserialize, Default)]
pub struct UpdateGiftRequest {
    pub name_en: Option<String>,
    pub name_ar: Option<String>,
    pub icon_url: Option<String>,
    pub price: Option<i64>,
    pub tier: Option<i16>,
    pub effect_level: Option<i16>,
    pub animation_url: Option<String>,
    pub sort_order: Option<i32>,
    /// 上/下架切换（true=上架，false=下架）
    pub is_active: Option<bool>,
}

// ─── 响应 DTO ─────────────────────────────────────────────────────────────────

/// 单个礼物响应体
#[derive(Debug, Serialize)]
pub struct GiftResponse {
    pub id: String,
    pub code: String,
    pub name_en: String,
    pub name_ar: String,
    pub icon_url: String,
    pub price: i64,
    pub tier: i16,
    pub effect_level: i16,
    pub animation_url: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub is_deleted: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<GiftModel> for GiftResponse {
    fn from(m: GiftModel) -> Self {
        Self {
            id: m.id.to_string(),
            code: m.code,
            name_en: m.name_en,
            name_ar: m.name_ar,
            icon_url: m.icon_url,
            price: m.price,
            tier: m.tier,
            effect_level: m.effect_level,
            animation_url: m.animation_url,
            sort_order: m.sort_order,
            is_active: m.is_active,
            is_deleted: m.is_deleted,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

/// 礼物列表响应体
#[derive(Debug, Serialize)]
pub struct ListGiftsResponse {
    pub total: i64,
    pub page: i64,
    pub size: i64,
    pub items: Vec<GiftResponse>,
}

/// 文件上传响应体
#[derive(Debug, Serialize)]
pub struct UploadGiftFileResponse {
    pub url: String,
}

// ─── 单元测试（DTO）──────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_model(price: i64, tier: i16, is_active: bool, is_deleted: bool) -> GiftModel {
        GiftModel {
            id: Uuid::new_v4(),
            code: "rose_01".to_string(),
            name_en: "Rose".to_string(),
            name_ar: "وردة".to_string(),
            icon_url: "/uploads/gifts/rose.png".to_string(),
            price,
            tier,
            effect_level: 1,
            animation_url: None,
            sort_order: 10,
            is_active,
            is_deleted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// DTO-01: GiftModel → GiftResponse 字段正确映射
    #[test]
    fn dto01_gift_model_converts_to_gift_response() {
        let id = Uuid::new_v4();
        let model = GiftModel {
            id,
            code: "test_gift".to_string(),
            name_en: "Test Gift".to_string(),
            name_ar: "هدية".to_string(),
            icon_url: "/uploads/gifts/test.png".to_string(),
            price: 66,
            tier: 3,
            effect_level: 2,
            animation_url: Some("/uploads/gifts/test.json".to_string()),
            sort_order: 30,
            is_active: true,
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let resp = GiftResponse::from(model);
        assert_eq!(resp.id, id.to_string());
        assert_eq!(resp.code, "test_gift");
        assert_eq!(resp.price, 66);
        assert_eq!(resp.tier, 3);
        assert_eq!(resp.effect_level, 2);
        assert!(resp.animation_url.is_some());
        assert!(resp.is_active);
        assert!(!resp.is_deleted);
    }

    /// DTO-02: is_active=false 的礼物也能正确转换
    #[test]
    fn dto02_inactive_gift_converts_correctly() {
        let model = make_model(1, 1, false, false);
        let resp = GiftResponse::from(model);
        assert!(!resp.is_active);
        assert!(!resp.is_deleted);
    }

    /// DTO-03: is_deleted=true 的礼物转换后 is_deleted 正确
    #[test]
    fn dto03_deleted_gift_converts_correctly() {
        let model = make_model(100, 2, true, true);
        let resp = GiftResponse::from(model);
        assert!(resp.is_deleted);
    }
}
