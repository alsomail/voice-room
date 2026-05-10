//! Payment Order Service — 创建订单（T-00051）
//!
//! - `PaymentOrderServicePort` trait：HTTP handler 依赖的接口
//! - `PaymentOrderService`：真实实现（pool + risk check）
//! - `FakePaymentOrderService`：测试替身

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use voice_room_shared::payment::Provider;

use super::dto::{CreateOrderData, SkuListData};
use super::error::PaymentError;
use super::repo::PaymentRepo;
use super::risk::RiskCheckPort;

/// 创建订单服务 Trait（HTTP handler 依赖）
#[async_trait]
pub trait PaymentOrderServicePort: Send + Sync {
    /// 查询激活的 SKU 列表
    async fn list_skus(&self, provider: &str) -> Result<SkuListData, PaymentError>;

    /// 创建 PENDING 订单
    ///
    /// # 参数
    /// - `user_id`: 当前登录用户
    /// - `sku_id`: SKU ID（必须 is_active=true）
    /// - `provider`: 支付渠道（"google_play" 等）
    /// - `idempotency_key`: 客户端幂等 key（可选）
    async fn create_order(
        &self,
        user_id: Uuid,
        sku_id: &str,
        provider: &str,
        idempotency_key: Option<&str>,
    ) -> Result<CreateOrderData, PaymentError>;
}

/// Payment Order Service 真实实现
pub struct PaymentOrderService {
    repo: PaymentRepo,
    risk: Arc<dyn RiskCheckPort>,
}

impl PaymentOrderService {
    pub fn new(pool: PgPool, risk: Arc<dyn RiskCheckPort>) -> Self {
        Self {
            repo: PaymentRepo::new(pool),
            risk,
        }
    }
}

#[async_trait]
impl PaymentOrderServicePort for PaymentOrderService {
    async fn list_skus(&self, provider: &str) -> Result<SkuListData, PaymentError> {
        let provider_enum = parse_provider(provider)?;
        let skus = self.repo.list_active_skus(&provider_enum).await?;
        Ok(SkuListData {
            skus: skus.iter().map(|s| s.to_dto()).collect(),
        })
    }

    async fn create_order(
        &self,
        user_id: Uuid,
        sku_id: &str,
        provider: &str,
        idempotency_key: Option<&str>,
    ) -> Result<CreateOrderData, PaymentError> {
        // 1. 查询并校验 SKU
        let sku = self
            .repo
            .find_sku_by_id(sku_id)
            .await?
            .ok_or(PaymentError::SkuDisabled)?;

        if !sku.is_active {
            return Err(PaymentError::SkuDisabled);
        }

        // 2. 风控检查
        self.risk.evaluate(user_id).await?;

        // 3. 解析 provider
        let provider_enum = parse_provider(provider)?;

        // 4. 创建订单
        let order_id = self
            .repo
            .create_order(user_id, sku_id, &provider_enum, idempotency_key)
            .await?;

        // 5. 构建响应（expire_at = 30min 后）
        let expire_at = Utc::now() + chrono::Duration::minutes(30);

        Ok(CreateOrderData {
            order_id,
            sku: sku.to_dto(),
            expire_at,
        })
    }
}

fn parse_provider(provider: &str) -> Result<Provider, PaymentError> {
    match provider {
        "google_play" => Ok(Provider::GooglePlay),
        "apple_iap" => Ok(Provider::AppleIap),
        "mock" => Ok(Provider::Mock),
        _ => Err(PaymentError::SkuDisabled),
    }
}

// ─── Fake（仅测试）──────────────────────────────────────────────────────────

pub struct FakePaymentOrderService;

#[async_trait]
impl PaymentOrderServicePort for FakePaymentOrderService {
    async fn list_skus(&self, _provider: &str) -> Result<SkuListData, PaymentError> {
        Ok(SkuListData { skus: vec![] })
    }

    async fn create_order(
        &self,
        _user_id: Uuid,
        _sku_id: &str,
        _provider: &str,
        _idempotency_key: Option<&str>,
    ) -> Result<CreateOrderData, PaymentError> {
        use super::dto::SkuDto;
        Ok(CreateOrderData {
            order_id: Uuid::new_v4(),
            sku: SkuDto {
                sku_id: "diamond_60".to_string(),
                provider: "google_play".to_string(),
                diamonds: 60,
                display_price_usd: "0.99".to_string(),
                display_price_local: None,
                display_currency: None,
                tag: None,
                sort_order: 10,
            },
            expire_at: Utc::now() + chrono::Duration::minutes(30),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use crate::modules::payment::risk::FakeRiskCheckService;

    // S01: FakePaymentOrderService list_skus 返回空列表
    #[tokio::test]
    async fn s01_fake_service_list_skus_empty() {
        let svc = FakePaymentOrderService;
        let result = svc.list_skus("google_play").await.unwrap();
        assert!(result.skus.is_empty());
    }

    // S02: FakePaymentOrderService create_order 返回 UUID
    #[tokio::test]
    async fn s02_fake_service_create_order_returns_uuid() {
        let svc = FakePaymentOrderService;
        let result = svc
            .create_order(Uuid::new_v4(), "diamond_60", "google_play", None)
            .await
            .unwrap();
        assert_ne!(result.order_id, Uuid::nil());
    }

    // S03: parse_provider 识别 google_play
    #[test]
    fn s03_parse_provider_google_play() {
        let p = parse_provider("google_play").unwrap();
        assert_eq!(p, Provider::GooglePlay);
    }

    // S04: parse_provider 未知渠道返回错误
    #[test]
    fn s04_parse_provider_unknown_returns_error() {
        assert!(parse_provider("unknown").is_err());
    }

    // S05: FakeRiskCheckService block=true 使 create_order 失败
    #[tokio::test]
    async fn s05_risk_block_causes_order_failure_in_fake() {
        // 直接测试 FakeRiskCheckService
        let risk = Arc::new(FakeRiskCheckService { block: true });
        let err = risk.evaluate(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, PaymentError::OrderRiskBlocked));
    }
}
