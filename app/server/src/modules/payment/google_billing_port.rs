//! GooglePlayBillingPort — 防腐层 Trait
//!
//! 业务层（verify_service.rs）**只**引用此 trait，
//! 不直接调用任何 Google API SDK。
//!
//! 验收红线：
//!   `grep -r "google" app/server/src/modules/payment/verify_service.rs`
//!   不含具体 API 调用（只引用 trait）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::PaymentError;

/// Google Play Purchases.products:get 返回的购买信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductPurchase {
    /// 0=PURCHASED, 1=CANCELLED, 2=PENDING
    pub purchase_state: i32,
    /// 消费状态：0=未消费，1=已消费
    pub consumption_state: i32,
    /// obfuscatedExternalAccountId：必须等于我方 order_id
    pub obfuscated_external_account_id: Option<String>,
    /// obfuscatedExternalProfileId：我方 user_id
    pub obfuscated_external_profile_id: Option<String>,
    /// 支付金额（micros）
    pub price_amount_micros: Option<i64>,
    /// 货币码
    pub price_currency_code: Option<String>,
    /// 国家码
    pub country_code: Option<String>,
    /// 购买时间（Unix millis）
    pub purchase_time_millis: Option<i64>,
    /// Google orderId
    pub order_id: Option<String>,
    /// 是否已 acknowledge
    pub acknowledgement_state: i32,
}

/// Google Play Billing 防腐层 Trait
///
/// 所有对 Google AndroidPublisher API 的调用必须通过此接口，
/// 业务层仅依赖此 trait，不得直接引用 google API 客户端。
#[async_trait]
pub trait GooglePlayBillingPort: Send + Sync {
    /// 调用 `purchases.products.get` 验证购买记录
    ///
    /// - `package_name`：应用包名
    /// - `product_id`：sku_id（与 Google Console 的 productId 一致）
    /// - `purchase_token`：客户端 BillingClient.Purchase.purchaseToken
    async fn get_product_purchase(
        &self,
        package_name: &str,
        product_id: &str,
        purchase_token: &str,
    ) -> Result<ProductPurchase, PaymentError>;

    /// 调用 `purchases.products.acknowledge`
    ///
    /// acknowledge 后钻石才算真正归属用户（3 天内必须完成）。
    async fn acknowledge(
        &self,
        package_name: &str,
        product_id: &str,
        purchase_token: &str,
    ) -> Result<(), PaymentError>;
}

// ─── Fake 实现（仅测试）─────────────────────────────────────────────────────

/// 测试替身 — 可预置 ProductPurchase 响应
#[derive(Default)]
pub struct FakeGooglePlayBillingPort {
    /// 模拟 purchaseState：0=PURCHASED, 1=CANCELLED, 2=PENDING
    pub purchase_state: i32,
    /// 模拟 obfuscatedExternalAccountId（对应 order_id）
    pub obfuscated_account_id: Option<Uuid>,
    /// 模拟 acknowledge 结果：true=成功, false=失败
    pub ack_success: bool,
    /// 是否让 get_product_purchase 返回错误
    pub get_error: bool,
}

impl FakeGooglePlayBillingPort {
    /// 预置成功的购买状态（purchaseState=0，account_id 匹配）
    pub fn success(order_id: Uuid) -> Self {
        Self {
            purchase_state: 0,
            obfuscated_account_id: Some(order_id),
            ack_success: true,
            get_error: false,
        }
    }

    /// 预置 purchaseState=1（CANCELLED）
    pub fn cancelled(order_id: Uuid) -> Self {
        Self {
            purchase_state: 1,
            obfuscated_account_id: Some(order_id),
            ack_success: true,
            get_error: false,
        }
    }

    /// 预置 obfuscatedAccountId 不匹配（模拟 token 欺诈）
    pub fn wrong_account() -> Self {
        Self {
            purchase_state: 0,
            obfuscated_account_id: Some(Uuid::new_v4()), // 随机 UUID，不匹配任何 order_id
            ack_success: true,
            get_error: false,
        }
    }
}

#[async_trait]
impl GooglePlayBillingPort for FakeGooglePlayBillingPort {
    async fn get_product_purchase(
        &self,
        _package_name: &str,
        _product_id: &str,
        _purchase_token: &str,
    ) -> Result<ProductPurchase, PaymentError> {
        if self.get_error {
            return Err(PaymentError::GoogleApiUnavailable);
        }
        Ok(ProductPurchase {
            purchase_state: self.purchase_state,
            consumption_state: 0,
            obfuscated_external_account_id: self
                .obfuscated_account_id
                .map(|id| id.to_string()),
            obfuscated_external_profile_id: None,
            price_amount_micros: Some(9_990_000),
            price_currency_code: Some("USD".to_string()),
            country_code: Some("US".to_string()),
            purchase_time_millis: Some(1_746_788_688_000),
            order_id: Some("GPA.0001-test".to_string()),
            acknowledgement_state: 0,
        })
    }

    async fn acknowledge(
        &self,
        _package_name: &str,
        _product_id: &str,
        _purchase_token: &str,
    ) -> Result<(), PaymentError> {
        if self.ack_success {
            Ok(())
        } else {
            Err(PaymentError::GoogleApiUnavailable)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // G01: FakeGooglePlayBillingPort::success 返回 purchaseState=0
    #[tokio::test]
    async fn g01_fake_billing_port_success_returns_purchased() {
        let order_id = Uuid::new_v4();
        let port = FakeGooglePlayBillingPort::success(order_id);
        let purchase = port
            .get_product_purchase("com.test", "diamond_600", "token123")
            .await
            .unwrap();
        assert_eq!(purchase.purchase_state, 0);
        assert_eq!(
            purchase.obfuscated_external_account_id,
            Some(order_id.to_string())
        );
    }

    // G02: FakeGooglePlayBillingPort::cancelled 返回 purchaseState=1
    #[tokio::test]
    async fn g02_fake_billing_port_cancelled_returns_state_1() {
        let order_id = Uuid::new_v4();
        let port = FakeGooglePlayBillingPort::cancelled(order_id);
        let purchase = port
            .get_product_purchase("com.test", "diamond_600", "token123")
            .await
            .unwrap();
        assert_eq!(purchase.purchase_state, 1);
    }

    // G03: FakeGooglePlayBillingPort acknowledge 成功
    #[tokio::test]
    async fn g03_fake_billing_port_acknowledge_success() {
        let port = FakeGooglePlayBillingPort {
            ack_success: true,
            ..Default::default()
        };
        assert!(
            port.acknowledge("com.test", "diamond_600", "token123")
                .await
                .is_ok()
        );
    }

    // G04: FakeGooglePlayBillingPort::wrong_account 返回不匹配的 account_id
    #[tokio::test]
    async fn g04_fake_billing_port_wrong_account() {
        let port = FakeGooglePlayBillingPort::wrong_account();
        let order_id = Uuid::new_v4();
        let purchase = port
            .get_product_purchase("com.test", "diamond_600", "token123")
            .await
            .unwrap();
        // obfuscated account id 不等于 order_id
        assert_ne!(
            purchase.obfuscated_external_account_id,
            Some(order_id.to_string())
        );
    }

    // G05: get_error=true 返回 GoogleApiUnavailable
    #[tokio::test]
    async fn g05_fake_billing_port_get_error() {
        let port = FakeGooglePlayBillingPort {
            get_error: true,
            ..Default::default()
        };
        let err = port
            .get_product_purchase("com.test", "diamond_600", "token123")
            .await
            .unwrap_err();
        assert!(matches!(err, PaymentError::GoogleApiUnavailable));
    }
}
