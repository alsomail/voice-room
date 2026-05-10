//! Payment Verify Service — Google Play 验签 + 入账强事务（T-00052）
//!
//! **防腐层保证**：本文件 **只引用** `GooglePlayBillingPort` trait，
//! 不直接调用任何 Google API SDK。
//!
//! 验收红线：
//!   `grep -r "google" app/server/src/modules/payment/verify_service.rs`
//!   不应出现具体 HTTP client / SDK 调用，只有 trait 方法调用。

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;
use voice_room_shared::payment::Provider;

use crate::modules::wallet::broadcaster::BalanceEvent;

use super::dto::VerifyData;
use super::error::PaymentError;
use super::google_billing_port::GooglePlayBillingPort;
use super::repo::PaymentRepo;

const PACKAGE_NAME: &str = "com.voiceroom.android";

/// 验签 + 入账服务 Trait
#[async_trait]
pub trait PaymentVerifyServicePort: Send + Sync {
    /// 验签并入账
    ///
    /// # 流程
    /// 1. PENDING → VERIFYING（事务1）
    /// 2. 调用 GooglePlayBillingPort::get_product_purchase（trait 调用）
    /// 3. 校验 purchaseState / obfuscatedAccountId
    /// 4. VERIFIED → CREDITED + balance+=diamonds + wallet_transactions（事务2，原子）
    /// 5. acknowledge（trait 调用）→ ACKED
    /// 6. 发送 WS BalanceUpdated
    async fn verify_and_credit(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        purchase_token: &str,
        provider_order_id: Option<&str>,
    ) -> Result<VerifyData, PaymentError>;
}

/// Payment Verify Service 真实实现
pub struct PaymentVerifyService {
    repo: PaymentRepo,
    billing_port: Arc<dyn GooglePlayBillingPort>,
    pool: PgPool,
    balance_tx: mpsc::Sender<BalanceEvent>,
}

impl PaymentVerifyService {
    pub fn new(
        pool: PgPool,
        billing_port: Arc<dyn GooglePlayBillingPort>,
        balance_tx: mpsc::Sender<BalanceEvent>,
    ) -> Self {
        Self {
            repo: PaymentRepo::new(pool.clone()),
            billing_port,
            pool,
            balance_tx,
        }
    }
}

#[async_trait]
impl PaymentVerifyServicePort for PaymentVerifyService {
    async fn verify_and_credit(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        purchase_token: &str,
        provider_order_id: Option<&str>,
    ) -> Result<VerifyData, PaymentError> {
        // ─── 幂等检查：同一 purchase_token 是否已处理 ───────────────────────
        if let Some(existing) = self
            .repo
            .find_order_by_purchase_token(purchase_token, &Provider::GooglePlay)
            .await?
        {
            // 已处理过：直接返回已有结果
            return Ok(VerifyData {
                order_id: existing.order_id,
                state: existing.state.to_string(),
                diamonds_credited: None, // 已有流水，不重复返回
                balance_after: None,
                next_action: None,
            });
        }

        // ─── 事务1：PENDING → VERIFYING（悲观锁）────────────────────────────
        let mut txn1 = self
            .pool
            .begin()
            .await
            .map_err(|e| PaymentError::Database(e.to_string()))?;

        let order = self
            .repo
            .transition_to_verifying(&mut txn1, order_id, purchase_token, provider_order_id)
            .await?;

        // 校验订单归属
        if order.user_id != user_id {
            return Err(PaymentError::OrderNotFound);
        }

        txn1.commit()
            .await
            .map_err(|e| PaymentError::Database(e.to_string()))?;

        // ─── 调用 Google 验签（防腐层 trait 调用，不含具体 SDK）────────────
        let purchase = self
            .billing_port
            .get_product_purchase(PACKAGE_NAME, &order.sku_id, purchase_token)
            .await?;

        // 校验 purchaseState
        if purchase.purchase_state != 0 {
            // PENDING_GOOGLE 场景
            if purchase.purchase_state == 2 {
                return Ok(VerifyData {
                    order_id,
                    state: "PENDING_GOOGLE".to_string(),
                    diamonds_credited: None,
                    balance_after: None,
                    next_action: Some("wait_rtdn".to_string()),
                });
            }
            // CANCELLED 或其他
            let mut fail_txn = self
                .pool
                .begin()
                .await
                .map_err(|e| PaymentError::Database(e.to_string()))?;
            let _ = self
                .repo
                .transition_to_failed(&mut fail_txn, order_id, "invalid_purchase_state")
                .await;
            let _ = fail_txn.commit().await;
            return Err(PaymentError::InvalidPurchase);
        }

        // 校验 obfuscatedExternalAccountId 必须等于 order_id
        let expected_account_id = order_id.to_string();
        if purchase
            .obfuscated_external_account_id
            .as_deref()
            .map(|id| id != expected_account_id)
            .unwrap_or(true)
        {
            let mut fail_txn = self
                .pool
                .begin()
                .await
                .map_err(|e| PaymentError::Database(e.to_string()))?;
            let _ = self
                .repo
                .transition_to_failed(&mut fail_txn, order_id, "obfuscated_account_id_mismatch")
                .await;
            let _ = fail_txn.commit().await;
            return Err(PaymentError::InvalidPurchase);
        }

        // ─── 查询 SKU 获取 diamonds 数量 ─────────────────────────────────────
        let sku = self
            .repo
            .find_sku_by_id(&order.sku_id)
            .await?
            .ok_or(PaymentError::SkuDisabled)?;

        // ─── 事务2：VERIFIED → CREDITED + balance += diamonds（原子）────────
        let mut txn2 = self
            .pool
            .begin()
            .await
            .map_err(|e| PaymentError::Database(e.to_string()))?;

        let balance_after = self
            .repo
            .credit_balance(
                &mut txn2,
                user_id,
                sku.diamonds,
                order_id,
                "recharge_google_play",
            )
            .await?;

        self.repo
            .transition_to_credited(
                &mut txn2,
                order_id,
                purchase.price_amount_micros,
                purchase.price_currency_code.as_deref(),
                purchase.country_code.as_deref(),
            )
            .await?;

        txn2.commit()
            .await
            .map_err(|e| PaymentError::Database(e.to_string()))?;

        // ─── WS 广播 BalanceUpdated（reason='payment_credit'）────────────────
        let ws_event = BalanceEvent {
            user_id,
            balance_after,
            delta: sku.diamonds,
            reason: "payment_credit".to_string(),
            ref_id: Some(order_id),
        };
        if let Err(e) = self.balance_tx.try_send(ws_event) {
            tracing::warn!(
                order_id = %order_id,
                "BalanceUpdated WS event dropped (channel full): {:?}",
                e
            );
        }

        // ─── Acknowledge（trait 调用）→ ACKED ────────────────────────────────
        match self
            .billing_port
            .acknowledge(PACKAGE_NAME, &order.sku_id, purchase_token)
            .await
        {
            Ok(()) => {
                let _ = self.repo.transition_to_acked(order_id).await;
                Ok(VerifyData {
                    order_id,
                    state: "ACKED".to_string(),
                    diamonds_credited: Some(sku.diamonds),
                    balance_after: Some(balance_after),
                    next_action: None,
                })
            }
            Err(e) => {
                // acknowledge 失败不回滚（钻石已入账），cron 会重试
                tracing::error!(
                    order_id = %order_id,
                    error = %e,
                    "acknowledge failed, will retry via cron"
                );
                Ok(VerifyData {
                    order_id,
                    state: "CREDITED".to_string(),
                    diamonds_credited: Some(sku.diamonds),
                    balance_after: Some(balance_after),
                    next_action: None,
                })
            }
        }
    }
}

// ─── Fake（仅测试）──────────────────────────────────────────────────────────

pub struct FakePaymentVerifyService;

#[async_trait]
impl PaymentVerifyServicePort for FakePaymentVerifyService {
    async fn verify_and_credit(
        &self,
        _user_id: Uuid,
        order_id: Uuid,
        _purchase_token: &str,
        _provider_order_id: Option<&str>,
    ) -> Result<VerifyData, PaymentError> {
        Ok(VerifyData {
            order_id,
            state: "ACKED".to_string(),
            diamonds_credited: Some(60),
            balance_after: Some(60),
            next_action: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // V01: FakePaymentVerifyService 返回 ACKED
    #[tokio::test]
    async fn v01_fake_verify_service_returns_acked() {
        let svc = FakePaymentVerifyService;
        let order_id = Uuid::new_v4();
        let result = svc
            .verify_and_credit(Uuid::new_v4(), order_id, "token123", None)
            .await
            .unwrap();
        assert_eq!(result.state, "ACKED");
        assert_eq!(result.order_id, order_id);
    }

    // V02: FakePaymentVerifyService 是 Send+Sync
    #[test]
    fn v02_fake_verify_service_is_send_sync() {
        let _: Arc<dyn PaymentVerifyServicePort> = Arc::new(FakePaymentVerifyService);
    }
}
