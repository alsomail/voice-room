//! T-10025: 订单查询 DTO
//!
//! 路由参数、响应体字段名与 payment_api.md §9.7 保持一致。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use voice_room_shared::payment::{OrderState, Provider};

// ─── 列表查询参数 ─────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/payments/orders` 查询参数。
#[derive(Debug, Default, Deserialize)]
pub struct ListOrdersQuery {
    pub user_id: Option<Uuid>,
    pub state: Option<String>,
    pub provider: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub amount_min: Option<i64>,
    pub amount_max: Option<i64>,
    /// 1-based，默认 1
    pub page: Option<u32>,
    /// 1–100，超出截断为 100
    pub page_size: Option<u32>,
}

impl ListOrdersQuery {
    /// 校验查询参数，返回 (page, page_size) 或错误描述。
    pub fn validate(&self) -> Result<(u32, u32), String> {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);

        if let (Some(from), Some(to)) = (self.created_from, self.created_to) {
            if from > to {
                return Err("created_from must be <= created_to".to_string());
            }
        }
        if let (Some(min), Some(max)) = (self.amount_min, self.amount_max) {
            if min > max {
                return Err("amount_min must be <= amount_max".to_string());
            }
        }
        Ok((page, page_size))
    }
}

// ─── 列表项 ──────────────────────────────────────────────────────────────────

/// 订单列表项（不含 raw response）。
#[derive(Debug, Clone, Serialize)]
pub struct AdminOrderListItem {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub sku_id: String,
    pub provider: String,
    pub amount_micros: Option<i64>,
    pub currency: Option<String>,
    pub country_code: Option<String>,
    pub state: String,
    pub purchase_token_masked: Option<String>,
    pub provider_order_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub credited_at: Option<DateTime<Utc>>,
    pub acked_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
}

// ─── 详情响应 ─────────────────────────────────────────────────────────────────

/// 订单详情响应（含 state_history 与 Google raw response）。
#[derive(Debug, Clone, Serialize)]
pub struct AdminOrderDetail {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub sku_id: String,
    pub provider: String,
    pub amount_micros: Option<i64>,
    pub currency: Option<String>,
    pub country_code: Option<String>,
    pub state: String,
    pub state_history: serde_json::Value,
    pub provider_response_raw: Option<serde_json::Value>,
    pub purchase_token_masked: Option<String>,
    pub provider_order_id: Option<String>,
    pub risk_flags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub credited_at: Option<DateTime<Utc>>,
    pub acked_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub failed_reason: Option<String>,
}

// ─── 列表响应 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ListOrdersResponse {
    pub data: Vec<AdminOrderListItem>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

/// 对 purchase_token 做脱敏：前 5 字符 + "..." + 后 4 字符。
/// 长度不足时返回 None。
pub fn mask_purchase_token(token: &str) -> Option<String> {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() < 9 {
        return None;
    }
    let prefix: String = chars[..5].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    Some(format!("{prefix}...{suffix}"))
}

/// 从共享 OrderState / Provider 枚举转换为 AdminOrderListItem（用于 From 实现）。
pub fn state_to_string(state: &OrderState) -> String {
    state.to_string()
}

pub fn provider_to_string(provider: &Provider) -> String {
    provider.to_string()
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // ── DTO-01: validate 默认值 ──────────────────────────────────────────────

    /// validate 默认值 → page=1, page_size=20
    #[test]
    fn dto01_validate_defaults() {
        let q = ListOrdersQuery::default();
        let (page, size) = q.validate().unwrap();
        assert_eq!(page, 1);
        assert_eq!(size, 20);
    }

    /// page_size=200 → 截断为 100
    #[test]
    fn dto02_page_size_clamped_to_100() {
        let q = ListOrdersQuery {
            page_size: Some(200),
            ..Default::default()
        };
        let (_, size) = q.validate().unwrap();
        assert_eq!(size, 100, "page_size > 100 should clamp to 100");
    }

    /// created_from > created_to → Err
    #[test]
    fn dto03_created_from_greater_than_to_returns_err() {
        let q = ListOrdersQuery {
            created_from: Some(Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap()),
            created_to: Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()),
            ..Default::default()
        };
        let result = q.validate();
        assert!(result.is_err(), "created_from > created_to must be err");
    }

    /// amount_min > amount_max → Err
    #[test]
    fn dto04_amount_min_greater_than_max_returns_err() {
        let q = ListOrdersQuery {
            amount_min: Some(1000),
            amount_max: Some(500),
            ..Default::default()
        };
        assert!(q.validate().is_err());
    }

    // ── DTO-05: mask_purchase_token ──────────────────────────────────────────

    /// 正常 token 脱敏: 前5+...+后4
    #[test]
    fn dto05_mask_purchase_token_normal() {
        let result = mask_purchase_token("oojkl1234567ABCD");
        assert_eq!(result, Some("oojkl...ABCD".to_string()));
    }

    /// 过短 token → None
    #[test]
    fn dto06_mask_purchase_token_too_short_returns_none() {
        assert_eq!(mask_purchase_token("abc"), None);
        assert_eq!(mask_purchase_token("12345678"), None); // 8 chars < 9
        assert_eq!(mask_purchase_token("123456789"), Some("12345...6789".to_string()));
    }

    /// Unicode token 脱敏
    #[test]
    fn dto07_mask_purchase_token_unicode() {
        let token = "你好世界abcde"; // 9 chars
        let result = mask_purchase_token(token);
        assert!(result.is_some());
    }
}
