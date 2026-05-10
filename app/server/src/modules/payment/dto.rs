//! Payment DTO — 请求/响应结构体
//!
//! 字段严格匹配 doc/protocol/payment_api.md §9.3
//! - §9.3.1 SKU 列表响应
//! - §9.3.2 创建订单请求/响应
//! - §9.3.3 Google 验签请求/响应
//! - §9.4.1 Dev Mock 充值请求/响应

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── SKU ──────────────────────────────────────────────────────────────────────

/// 单个 SKU 的响应字段（严格对齐 payment_api.md §9.3.1）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuDto {
    pub sku_id: String,
    pub provider: String,
    pub diamonds: i64,
    pub display_price_usd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_price_local: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub sort_order: i32,
}

/// GET /api/v1/payments/skus 响应的 data 字段
#[derive(Debug, Serialize)]
pub struct SkuListData {
    pub skus: Vec<SkuDto>,
}

// ─── 创建订单（§9.3.2）────────────────────────────────────────────────────────

/// POST /api/v1/payments/orders 请求体（§9.3.2）
#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub sku_id: String,
    pub provider: String,
    pub client_session_id: Option<String>,
}

/// POST /api/v1/payments/orders 成功响应的 data 字段（§9.3.2）
#[derive(Debug, Serialize)]
pub struct CreateOrderData {
    pub order_id: Uuid,
    pub sku: SkuDto,
    pub expire_at: DateTime<Utc>,
}

// ─── Google 验签（§9.3.3）────────────────────────────────────────────────────

/// POST /api/v1/payments/google/verify 请求体（§9.3.3）
#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub order_id: Uuid,
    pub purchase_token: String,
    pub provider_order_id: Option<String>,
}

/// POST /api/v1/payments/google/verify 成功响应的 data 字段（§9.3.3）
#[derive(Debug, Serialize)]
pub struct VerifyData {
    pub order_id: Uuid,
    pub state: String,
    pub diamonds_credited: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_after: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action: Option<String>,
}

// ─── RTDN Webhook（§9.5.1）────────────────────────────────────────────────────

/// Google RTDN Pub/Sub 包络（§9.5.1）
#[derive(Debug, Deserialize)]
pub struct RtdnEnvelope {
    pub message: RtdnMessage,
    #[serde(default)]
    pub subscription: Option<String>,
}

/// Pub/Sub message 字段
#[derive(Debug, Deserialize)]
pub struct RtdnMessage {
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "publishTime")]
    pub publish_time: String,
    /// base64 编码的 DeveloperNotification JSON
    pub data: String,
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
}

/// base64 解码后的 DeveloperNotification
#[derive(Debug, Deserialize)]
pub struct DeveloperNotification {
    pub version: Option<String>,
    #[serde(rename = "packageName")]
    pub package_name: Option<String>,
    #[serde(rename = "eventTimeMillis")]
    pub event_time_millis: Option<String>,
    #[serde(rename = "oneTimeProductNotification")]
    pub one_time_product_notification: Option<OneTimeProductNotification>,
    #[serde(rename = "voidedPurchaseNotification")]
    pub voided_purchase_notification: Option<VoidedPurchaseNotification>,
    #[serde(rename = "testNotification")]
    pub test_notification: Option<serde_json::Value>,
}

/// oneTimeProductNotification 字段
#[derive(Debug, Deserialize)]
pub struct OneTimeProductNotification {
    pub version: Option<String>,
    #[serde(rename = "notificationType")]
    pub notification_type: i32,
    #[serde(rename = "purchaseToken")]
    pub purchase_token: String,
    pub sku: String,
}

/// voidedPurchaseNotification 字段
#[derive(Debug, Deserialize)]
pub struct VoidedPurchaseNotification {
    #[serde(rename = "purchaseToken")]
    pub purchase_token: String,
    #[serde(rename = "orderId")]
    pub order_id: Option<String>,
    #[serde(rename = "productType")]
    pub product_type: Option<i32>,
    #[serde(rename = "refundType")]
    pub refund_type: Option<i32>,
}

// ─── Dev Mock（§9.4.1）────────────────────────────────────────────────────────

/// POST /api/v1/_dev/mock_recharge 请求体（§9.4.1）
#[derive(Debug, Deserialize)]
pub struct MockRechargeRequest {
    pub sku_id: String,
    pub force_outcome: String, // "success" | "fail" | "pending"
    pub client_note: Option<String>,
}

/// POST /api/v1/_dev/mock_recharge 成功响应（§9.4.1）
#[derive(Debug, Serialize)]
pub struct MockRechargeData {
    pub order_id: Uuid,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diamonds_credited: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance_after: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_transaction_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // D01: CreateOrderRequest 反序列化
    #[test]
    fn d01_create_order_request_deserializes() {
        let json = r#"{"sku_id":"diamond_600","provider":"google_play"}"#;
        let req: CreateOrderRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.sku_id, "diamond_600");
        assert_eq!(req.provider, "google_play");
        assert!(req.client_session_id.is_none());
    }

    // D02: VerifyRequest 反序列化
    #[test]
    fn d02_verify_request_deserializes() {
        let order_id = Uuid::new_v4();
        let json = format!(
            r#"{{"order_id":"{}","purchase_token":"abc123"}}"#,
            order_id
        );
        let req: VerifyRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.order_id, order_id);
        assert_eq!(req.purchase_token, "abc123");
        assert!(req.provider_order_id.is_none());
    }

    // D03: SkuDto 序列化 skip_serializing_if None
    #[test]
    fn d03_sku_dto_omits_null_fields() {
        let sku = SkuDto {
            sku_id: "diamond_60".to_string(),
            provider: "google_play".to_string(),
            diamonds: 60,
            display_price_usd: "0.99".to_string(),
            display_price_local: None,
            display_currency: None,
            tag: None,
            sort_order: 10,
        };
        let json = serde_json::to_value(&sku).unwrap();
        assert!(json.get("display_price_local").is_none());
        assert!(json.get("tag").is_none());
    }

    // D04: MockRechargeRequest force_outcome 可接受 3 种值
    #[test]
    fn d04_mock_recharge_request_all_outcomes() {
        for outcome in &["success", "fail", "pending"] {
            let json = format!(r#"{{"sku_id":"diamond_60","force_outcome":"{}"}}"#, outcome);
            let req: MockRechargeRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(req.force_outcome, *outcome);
        }
    }

    // D05: RtdnEnvelope 反序列化
    #[test]
    fn d05_rtdn_envelope_deserializes() {
        let json = r#"{
            "message": {
                "messageId": "136969346945",
                "publishTime": "2026-05-09T10:24:48.690Z",
                "data": "dGVzdA=="
            },
            "subscription": "projects/test/subscriptions/rtdn-sub"
        }"#;
        let env: RtdnEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(env.message.message_id, "136969346945");
        assert_eq!(env.message.data, "dGVzdA==");
    }
}
