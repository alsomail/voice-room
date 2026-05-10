//! T-10027: SKU CRUD DTO

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ─── 列表/详情 ────────────────────────────────────────────────────────────────

/// payment_skus 行响应（与 payment_api.md §9.2.1 字段名一致）。
#[derive(Debug, Clone, Serialize)]
pub struct SkuResponse {
    pub sku_id: String,
    pub provider: String,
    pub diamonds: i64,
    pub display_price_usd: String,
    pub display_price_local: Option<String>,
    pub display_currency: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub tag: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─── 创建 ─────────────────────────────────────────────────────────────────────

/// `POST /api/v1/admin/payments/skus` 请求体。
#[derive(Debug, Deserialize)]
pub struct CreateSkuRequest {
    pub sku_id: String,
    pub provider: String,
    pub diamonds: i64,
    pub display_price_usd: String,
    pub display_price_local: Option<String>,
    pub display_currency: Option<String>,
    pub sort_order: Option<i32>,
    pub tag: Option<String>,
    pub is_active: Option<bool>,
}

impl CreateSkuRequest {
    /// 校验 diamonds > 0 且 display_price_usd 可解析且 > 0。
    pub fn validate(&self) -> Result<(), String> {
        if self.diamonds <= 0 {
            return Err("diamonds must be > 0".to_string());
        }
        let price: f64 = self
            .display_price_usd
            .parse()
            .map_err(|_| "display_price_usd must be a valid decimal".to_string())?;
        if price <= 0.0 {
            return Err("display_price_usd must be > 0".to_string());
        }
        Ok(())
    }

    /// 检查 sku_id 是否符合 `^[a-z][a-z0-9_]{2,63}$` 模式（非阻断，仅 warning）。
    pub fn sku_id_warning(&self) -> Option<String> {
        let re_ok = self.sku_id.len() >= 3
            && self.sku_id.len() <= 64
            && self.sku_id.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false)
            && self
                .sku_id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if re_ok {
            None
        } else {
            Some(format!(
                "sku_id '{}' may not match Google Play Console productId pattern ^[a-z][a-z0-9_]{{2,63}}$",
                self.sku_id
            ))
        }
    }
}

// ─── 更新 ─────────────────────────────────────────────────────────────────────

/// `PUT /api/v1/admin/payments/skus/:sku_id` 查询参数。
#[derive(Debug, Deserialize, Default)]
pub struct UpdateSkuQuery {
    /// 价格/钻石变更时必须携带 confirm=true
    pub confirm: Option<bool>,
}

/// `PUT /api/v1/admin/payments/skus/:sku_id` 请求体。
#[derive(Debug, Deserialize)]
pub struct UpdateSkuRequest {
    pub diamonds: Option<i64>,
    pub display_price_usd: Option<String>,
    pub display_price_local: Option<String>,
    pub display_currency: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
    pub tag: Option<String>,
}

impl UpdateSkuRequest {
    /// 校验 diamonds/display_price_usd（如提供）必须 > 0。
    pub fn validate(&self) -> Result<(), String> {
        if let Some(d) = self.diamonds {
            if d <= 0 {
                return Err("diamonds must be > 0".to_string());
            }
        }
        if let Some(ref p) = self.display_price_usd {
            let price: f64 = p
                .parse()
                .map_err(|_| "display_price_usd must be a valid decimal".to_string())?;
            if price <= 0.0 {
                return Err("display_price_usd must be > 0".to_string());
            }
        }
        Ok(())
    }

    /// 检查是否修改了价格/钻石数。
    pub fn has_price_change(&self, current_diamonds: i64, current_price: &str) -> bool {
        if let Some(d) = self.diamonds {
            if d != current_diamonds {
                return true;
            }
        }
        if let Some(ref p) = self.display_price_usd {
            if p != current_price {
                return true;
            }
        }
        false
    }
}

// ─── 创建响应 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CreateSkuResponse {
    pub sku: SkuResponse,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_create_req(sku_id: &str, diamonds: i64, price: &str) -> CreateSkuRequest {
        CreateSkuRequest {
            sku_id: sku_id.to_string(),
            provider: "google_play".to_string(),
            diamonds,
            display_price_usd: price.to_string(),
            display_price_local: None,
            display_currency: None,
            sort_order: None,
            tag: None,
            is_active: None,
        }
    }

    // ── SK-09: price=0 → 400 ValidationError ────────────────────────────

    #[test]
    fn sk09_price_zero_validation_error() {
        let req = make_create_req("diamond_600", 600, "0.00");
        assert!(
            req.validate().is_err(),
            "SK-09: display_price_usd=0 should fail"
        );
    }

    /// price 负值 → Err
    #[test]
    fn sk09b_negative_price_validation_error() {
        let req = make_create_req("diamond_600", 600, "-1.00");
        assert!(req.validate().is_err());
    }

    // ── SK-10: diamonds=0 → ValidationError ─────────────────────────────

    #[test]
    fn sk10_diamonds_zero_validation_error() {
        let req = make_create_req("diamond_600", 0, "9.99");
        assert!(
            req.validate().is_err(),
            "SK-10: diamonds=0 should fail"
        );
    }

    /// diamonds 负值 → Err
    #[test]
    fn sk10b_negative_diamonds_validation_error() {
        let req = make_create_req("diamond_600", -1, "9.99");
        assert!(req.validate().is_err());
    }

    // ── SK-02: 正常 → Ok ─────────────────────────────────────────────────

    #[test]
    fn sk02_valid_request_passes() {
        let req = make_create_req("diamond_600", 600, "9.99");
        assert!(req.validate().is_ok());
    }

    // ── SK-03: sku_id 不符合格式 → warning ──────────────────────────────

    #[test]
    fn sk03_sku_id_invalid_format_returns_warning() {
        let req = make_create_req("Diamond_600", 600, "9.99"); // uppercase D
        assert!(
            req.sku_id_warning().is_some(),
            "SK-03: uppercase should trigger warning"
        );
    }

    /// sku_id 符合格式 → no warning
    #[test]
    fn sk03b_valid_sku_id_no_warning() {
        let req = make_create_req("diamond_600", 600, "9.99");
        assert!(req.sku_id_warning().is_none());
    }

    // ── UpdateSkuRequest 测试 ────────────────────────────────────────────

    /// has_price_change: diamonds 变更 → true
    #[test]
    fn update_has_price_change_diamonds() {
        let req = UpdateSkuRequest {
            diamonds: Some(1200),
            display_price_usd: None,
            display_price_local: None,
            display_currency: None,
            is_active: None,
            sort_order: None,
            tag: None,
        };
        assert!(req.has_price_change(600, "9.99"));
    }

    /// has_price_change: only sort_order → false
    #[test]
    fn update_no_price_change_sort_order_only() {
        let req = UpdateSkuRequest {
            diamonds: None,
            display_price_usd: None,
            display_price_local: None,
            display_currency: None,
            is_active: None,
            sort_order: Some(10),
            tag: None,
        };
        assert!(!req.has_price_change(600, "9.99"));
    }

    /// UpdateSkuRequest validate: diamonds=0 → Err
    #[test]
    fn update_validate_diamonds_zero() {
        let req = UpdateSkuRequest {
            diamonds: Some(0),
            display_price_usd: None,
            display_price_local: None,
            display_currency: None,
            is_active: None,
            sort_order: None,
            tag: None,
        };
        assert!(req.validate().is_err());
    }
}
