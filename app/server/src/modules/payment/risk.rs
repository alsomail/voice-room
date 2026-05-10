//! Risk 风控服务
//!
//! 检查用户是否触发风控规则：
//! - 24h 内 FAILED 订单数 > 10 → 40903
//! - 设备黑名单命中 → 40903（当前版本通过 risk_flags 实现）

use sqlx::PgPool;
use uuid::Uuid;

use super::error::PaymentError;

/// 风控检查服务
pub struct RiskCheckService {
    pool: PgPool,
}

impl RiskCheckService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 对用户执行风控检查
    ///
    /// # 规则
    /// 1. 用户 24h 内 FAILED 订单数 > 10 → 返回 OrderRiskBlocked
    /// 2. 设备黑名单（通过 device_id 查询，当前版本暂不实现）
    ///
    /// # 返回
    /// - `Ok(())` 表示风控通过
    /// - `Err(PaymentError::OrderRiskBlocked)` 表示风控拦截
    pub async fn evaluate(&self, user_id: Uuid) -> Result<(), PaymentError> {
        // 规则 1：24h FAILED 订单数 > 10
        let failed_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_orders \
             WHERE user_id = $1 AND state = 'FAILED' \
             AND created_at > now() - INTERVAL '24 hours'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| PaymentError::Database(e.to_string()))?;

        if failed_count > 10 {
            tracing::warn!(
                user_id = %user_id,
                failed_count = failed_count,
                "risk check: user exceeded 24h FAILED order limit"
            );
            return Err(PaymentError::OrderRiskBlocked);
        }

        Ok(())
    }
}

/// 风控检查 Trait（供测试替身注入）
#[async_trait::async_trait]
pub trait RiskCheckPort: Send + Sync {
    async fn evaluate(&self, user_id: Uuid) -> Result<(), PaymentError>;
}

#[async_trait::async_trait]
impl RiskCheckPort for RiskCheckService {
    async fn evaluate(&self, user_id: Uuid) -> Result<(), PaymentError> {
        self.evaluate(user_id).await
    }
}

/// 测试替身：永远通过风控
pub struct FakeRiskCheckService {
    pub block: bool,
}

impl Default for FakeRiskCheckService {
    fn default() -> Self {
        Self { block: false }
    }
}

#[async_trait::async_trait]
impl RiskCheckPort for FakeRiskCheckService {
    async fn evaluate(&self, _user_id: Uuid) -> Result<(), PaymentError> {
        if self.block {
            Err(PaymentError::OrderRiskBlocked)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // R01: FakeRiskCheckService block=false 通过风控
    #[tokio::test]
    async fn r01_fake_risk_pass() {
        let svc = FakeRiskCheckService { block: false };
        assert!(svc.evaluate(Uuid::new_v4()).await.is_ok());
    }

    // R02: FakeRiskCheckService block=true 触发风控
    #[tokio::test]
    async fn r02_fake_risk_block() {
        let svc = FakeRiskCheckService { block: true };
        let err = svc.evaluate(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, PaymentError::OrderRiskBlocked));
    }

    // R03: FakeRiskCheckService 满足 Send + Sync
    #[test]
    fn r03_fake_risk_is_send_sync() {
        let _: Arc<dyn RiskCheckPort> = Arc::new(FakeRiskCheckService::default());
    }
}
