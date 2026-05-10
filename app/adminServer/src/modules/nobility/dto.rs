// PROTO-BINDING: doc/protocol/nobility_api.md §10.5 Admin REST
//! DTO 定义：贵族 Tier CRUD、手动赠送/撤销、用户查询。
//!
//! 字段约束来源：[nobility_api.md §10.2](doc/protocol/nobility_api.md#102-数据模型)

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── T-10030 Tier CRUD ────────────────────────────────────────────────────────

/// POST /api/v1/admin/nobles/tiers request body
#[derive(Debug, Deserialize)]
pub struct CreateTierRequest {
    pub tier_id: String,
    pub name_en: String,
    pub name_ar: String,
    /// SMALLINT 1..6，全局 UNIQUE
    pub level: i16,
    /// BIGINT > 0
    pub monthly_diamonds: i64,
    /// NUMERIC(10,2) > 0
    pub monthly_usd: String,
    pub usd_sku_id: Option<String>,
    /// JSONB，必须通过 privileges 校验
    pub privileges: serde_json::Value,
    pub icon_url: String,
    pub frame_url: String,
    pub entrance_animation_url: Option<String>,
    pub bgm_url: Option<String>,
    pub badge_color: String,
    pub bubble_style_id: String,
}

/// PUT /api/v1/admin/nobles/tiers/:id request body
#[derive(Debug, Deserialize)]
pub struct UpdateTierRequest {
    pub name_en: Option<String>,
    pub name_ar: Option<String>,
    pub monthly_diamonds: Option<i64>,
    pub monthly_usd: Option<String>,
    pub usd_sku_id: Option<String>,
    pub privileges: Option<serde_json::Value>,
    pub icon_url: Option<String>,
    pub frame_url: Option<String>,
    pub entrance_animation_url: Option<String>,
    pub bgm_url: Option<String>,
    pub badge_color: Option<String>,
    pub bubble_style_id: Option<String>,
}

/// GET /api/v1/admin/nobles/tiers query params
#[derive(Debug, Deserialize, Default)]
pub struct ListTiersQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub size: i64,
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    20
}

/// Tier 响应体
#[derive(Debug, Clone, Serialize)]
pub struct TierResponse {
    pub tier_id: String,
    pub name_en: String,
    pub name_ar: String,
    pub level: i16,
    pub monthly_diamonds: i64,
    pub monthly_usd: String,
    pub usd_sku_id: Option<String>,
    pub privileges: serde_json::Value,
    pub icon_url: String,
    pub frame_url: String,
    pub entrance_animation_url: Option<String>,
    pub bgm_url: Option<String>,
    pub badge_color: String,
    pub bubble_style_id: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 分页 tier 列表响应
#[derive(Debug, Serialize)]
pub struct ListTiersResponse {
    pub items: Vec<TierResponse>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

// ─── T-10031 Grant/Revoke ─────────────────────────────────────────────────────

/// POST /api/v1/admin/users/:id/noble/grant request body
#[derive(Debug, Deserialize)]
pub struct GrantRequest {
    pub tier_id: String,
    /// 1..365
    pub duration_days: i32,
    pub reason: String,
}

/// POST /api/v1/admin/users/:id/noble/revoke request body
#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    pub reason: String,
}

/// 当前 user_noble 响应
#[derive(Debug, Clone, Serialize)]
pub struct UserNobleResponse {
    pub user_id: Uuid,
    pub tier_id: String,
    pub start_at: DateTime<Utc>,
    pub current_period_start: DateTime<Utc>,
    pub expire_at: DateTime<Utc>,
    pub auto_renew: bool,
    pub renew_channel: String,
    pub total_paid_diamonds: i64,
    pub total_paid_usd_micros: i64,
}

// ─── T-10032 User Query ───────────────────────────────────────────────────────

/// status filter: active | expired
#[derive(Debug, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum NobleStatusFilter {
    Active,
    Expired,
}

/// GET /api/v1/admin/nobles/users query params
#[derive(Debug, Deserialize, Default)]
pub struct ListUsersQuery {
    pub tier_id: Option<String>,
    pub status: Option<NobleStatusFilter>,
    /// ISO 8601 date: YYYY-MM-DD
    pub expire_before: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub size: i64,
}

/// 用户贵族列表项（JOIN noble_tiers）
#[derive(Debug, Clone, Serialize)]
pub struct UserNobleItem {
    pub user_id: Uuid,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub tier_id: String,
    pub tier_name_en: String,
    pub tier_name_ar: String,
    pub tier_level: i16,
    pub badge_color: String,
    pub start_at: DateTime<Utc>,
    pub current_period_start: DateTime<Utc>,
    pub expire_at: DateTime<Utc>,
    pub auto_renew: bool,
    pub renew_channel: String,
    pub total_paid_diamonds: i64,
    pub total_paid_usd_micros: i64,
}

/// 分页用户列表响应
#[derive(Debug, Serialize)]
pub struct ListUsersResponse {
    pub items: Vec<UserNobleItem>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

/// noble_history 条目
#[derive(Debug, Clone, Serialize)]
pub struct NobleHistoryItem {
    pub id: i64,
    pub user_id: Uuid,
    /// purchase | renew_success | renew_failed | upgrade | expire | admin_grant | admin_revoke
    pub event: String,
    pub from_tier: Option<String>,
    pub to_tier: Option<String>,
    /// user:<uuid> | system:cron | admin:<uuid>
    pub actor: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// 解析 expire_before 日期字符串（YYYY-MM-DD）→ DateTime<Utc>
pub fn parse_expire_before(s: &str) -> Result<DateTime<Utc>, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        .map_err(|_| format!("invalid date format: '{s}', expected YYYY-MM-DD"))
}
