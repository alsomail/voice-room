//! 贵族体系 DTO（T-00066 §10.3.1 / §10.3.2）
//!
//! 严格匹配 `nobility_api.md` 中定义的请求/响应字段。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use voice_room_shared::models::nobility::NoblePrivileges;

// ─── Tier DTO ─────────────────────────────────────────────────────────────────

/// 单个 tier 的完整响应对象（§10.3.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierDto {
    pub tier_id: String,
    /// 本地化名称（Accept-Language 决定使用 name_en 或 name_ar）
    pub name: String,
    pub level: i16,
    pub monthly_diamonds: i64,
    pub monthly_usd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usd_sku_id: Option<String>,
    pub icon_url: String,
    pub frame_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrance_animation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bgm_url: Option<String>,
    pub badge_color: String,
    pub bubble_style_id: String,
    pub privileges: NoblePrivileges,
}

/// GET /api/v1/nobles/tiers 响应体
#[derive(Debug, Serialize, Deserialize)]
pub struct ListTiersResponse {
    pub tiers: Vec<TierDto>,
}

// ─── Me DTO ───────────────────────────────────────────────────────────────────

/// GET /api/v1/nobles/me 响应体（持有贵族时）
#[derive(Debug, Serialize, Deserialize)]
pub struct MyNobleResponse {
    pub tier_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_period_start: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_renew: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renew_channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_remaining: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_grace_period: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<TierDto>,
}

impl MyNobleResponse {
    /// 未持有贵族时的空响应
    pub fn none() -> Self {
        Self {
            tier_id: None,
            level: None,
            start_at: None,
            current_period_start: None,
            expire_at: None,
            auto_renew: None,
            renew_channel: None,
            days_remaining: None,
            in_grace_period: None,
            tier: None,
        }
    }
}

// ─── Purchase DTO ─────────────────────────────────────────────────────────────

/// POST /api/v1/nobles/purchase 请求体（§10.3.3）
#[derive(Debug, Deserialize)]
pub struct PurchaseRequest {
    pub tier_id: String,
    pub msg_id: String,
    #[serde(default = "default_auto_renew")]
    pub auto_renew: bool,
    #[serde(default = "default_duration_days")]
    pub duration_days: i64,
}

fn default_auto_renew() -> bool {
    true
}
fn default_duration_days() -> i64 {
    30
}

/// 升级补差详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeProratedDetail {
    pub from_tier: String,
    pub refund_diamonds: i64,
    pub charge_diamonds: i64,
}

/// POST /api/v1/nobles/purchase 响应体
#[derive(Debug, Serialize, Deserialize)]
pub struct PurchaseResponse {
    pub user_noble: MyNobleResponse,
    pub diamonds_charged: i64,
    pub balance_after: i64,
    pub operation: String, // purchase | renew | upgrade
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upgrade_proration: Option<UpgradeProratedDetail>,
}

// ─── Auto-renew DTO ───────────────────────────────────────────────────────────

/// PATCH /api/v1/nobles/me/auto_renew 请求体
#[derive(Debug, Deserialize)]
pub struct AutoRenewRequest {
    pub enabled: bool,
}

/// PATCH /api/v1/nobles/me/auto_renew 响应体
#[derive(Debug, Serialize)]
pub struct AutoRenewResponse {
    pub auto_renew: bool,
}

// ─── Noble field for UserJoined/MemberSnapshot (T-00069) ─────────────────────

/// UserJoined.noble 字段（§10.4.7）及 NobleEntered payload 基础（§10.4.5）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNobleDto {
    pub tier_id: String,
    pub level: i16,
    pub badge_color: String,
    pub frame_url: String,
    pub expire_at: DateTime<Utc>,

    // ── NEW §10.4.5: NobleEntered 进场特效字段 ──────────────────────────────
    /// 进场动画 URL（LV3+ 非空；§10.2.1）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrance_animation_url: Option<String>,
    /// 进场 BGM URL（LV2+ 非空；§10.2.1）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bgm_url: Option<String>,
    /// 进场动画范围（§10.4.5）：`marquee`(LV3) | `half`(LV4) | `fullscreen`(LV5-6)
    #[serde(default = "default_entry_scope")]
    pub scope: String,
    /// 动画时长毫秒（§10.4.5）；默认 5000
    #[serde(default = "default_entry_duration_ms")]
    pub duration_ms: i64,

    // ── NEW §T-00070: 密码房免密特权 ─────────────────────────────────────────
    /// 是否可绕过密码验证（来自 `privileges.bypass_password.enabled`；§10.4.5）
    #[serde(default)]
    pub bypass_password_enabled: bool,
}

fn default_entry_scope() -> String {
    "marquee".to_string()
}
fn default_entry_duration_ms() -> i64 {
    5000
}

#[cfg(test)]
mod tests {
    use super::*;

    // DTO-01: PurchaseRequest 默认字段
    #[test]
    fn dto01_purchase_request_defaults() {
        let json = r#"{"tier_id":"duke","msg_id":"abc123"}"#;
        let req: PurchaseRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.tier_id, "duke");
        assert_eq!(req.msg_id, "abc123");
        assert!(req.auto_renew);
        assert_eq!(req.duration_days, 30);
    }

    // DTO-02: MyNobleResponse::none() 有 tier_id=null
    #[test]
    fn dto02_my_noble_response_none_has_null_tier_id() {
        let resp = MyNobleResponse::none();
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["tier_id"].is_null());
    }

    // DTO-03: TierDto 序列化含 privileges
    #[test]
    fn dto03_tier_dto_serializes_with_privileges() {
        let tier = TierDto {
            tier_id: "duke".to_string(),
            name: "Duke".to_string(),
            level: 5,
            monthly_diamonds: 300000,
            monthly_usd: "999.99".to_string(),
            usd_sku_id: Some("noble_duke_30d".to_string()),
            icon_url: "https://cdn/duke.svg".to_string(),
            frame_url: "https://cdn/duke_frame.png".to_string(),
            entrance_animation_url: Some("https://cdn/duke_entry.json".to_string()),
            bgm_url: Some("https://cdn/duke_bgm.mp3".to_string()),
            badge_color: "#06B6D4".to_string(),
            bubble_style_id: "duke".to_string(),
            privileges: NoblePrivileges {
                badge: None,
                entry_effect: None,
                chat_bubble: None,
                audience_pin: None,
                invisibility: None,
                bypass_password: None,
                mic_priority: Some(voice_room_shared::models::nobility::MicPriorityPrivilege {
                    weight: 3.0,
                }),
                gift_discount: Some(voice_room_shared::models::nobility::GiftDiscountPrivilege {
                    percent: 10,
                }),
                global_broadcast: None,
                vip_support: None,
                monthly_stipend: Some(
                    voice_room_shared::models::nobility::MonthlyStipendPrivilege {
                        percent: 15,          // duke: 15%
                        pay_immediately: true,
                    },
                ),
                expiry: None,
            },
        };
        let json = serde_json::to_value(&tier).unwrap();
        assert_eq!(json["tier_id"], "duke");
        assert_eq!(json["level"], 5);
        assert_eq!(json["monthly_diamonds"], 300000);
        assert_eq!(json["privileges"]["mic_priority"]["weight"], 3.0);
        // T-00067: duke monthly_stipend.percent = 15
        assert_eq!(json["privileges"]["monthly_stipend"]["percent"], 15);
        // T-00067: 15% × 300000 = 45000 (NOT 60000 which was the old wrong diamonds value)
        let percent = json["privileges"]["monthly_stipend"]["percent"].as_i64().unwrap();
        let monthly_diamonds = json["monthly_diamonds"].as_i64().unwrap();
        assert_eq!(percent * monthly_diamonds / 100, 45000, "duke stipend = 45000");
    }

    // DTO-04: UserNobleDto 序列化
    #[test]
    fn dto04_user_noble_dto_serializes() {
        let noble = UserNobleDto {
            tier_id: "king".to_string(),
            level: 6,
            badge_color: "#DC2626".to_string(),
            frame_url: "https://cdn/king_frame.png".to_string(),
            expire_at: Utc::now(),
            entrance_animation_url: Some("https://cdn/king_entry.json".to_string()),
            bgm_url: Some("https://cdn/king_bgm.mp3".to_string()),
            scope: "fullscreen".to_string(),
            duration_ms: 8000,
            bypass_password_enabled: true,
        };
        let json = serde_json::to_value(&noble).unwrap();
        assert_eq!(json["tier_id"], "king");
        assert_eq!(json["level"], 6);
        assert_eq!(json["scope"], "fullscreen");
        assert_eq!(json["duration_ms"], 8000);
        assert_eq!(json["bypass_password_enabled"], true);
    }

    // DTO-05: purchase handler WS signal — NobleChanged 信令格式验证
    #[test]
    fn dto05_noble_changed_signal_format() {
        use serde_json::json;
        // 模拟 NobleChanged 信令构造（purchase_handler 发送给购买用户）
        let user_id = uuid::Uuid::new_v4();
        let msg = json!({
            "type": "NobleChanged",
            "msg_id": uuid::Uuid::new_v4().to_string(),
            "payload": {
                "user_id": user_id.to_string(),
                "from_tier": null,
                "to_tier": "duke",
                "expire_at": "2026-06-01T00:00:00Z",
                "operation": "purchase"
            },
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        assert_eq!(msg["type"], "NobleChanged");
        assert_eq!(msg["payload"]["to_tier"], "duke");
        assert_eq!(msg["payload"]["operation"], "purchase");
    }
}
