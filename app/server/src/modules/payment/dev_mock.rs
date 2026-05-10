//! Dev Mock 充值服务（T-00055）
//!
//! 仅在 `dev_payment_mock` feature 开启时编译，提供测试环境下的充值模拟接口。
//! 该模块不应出现在生产构建产物中。
//!
//! ## 架构
//! - `PaymentMockServicePort` — trait，controller 依赖此接口
//! - `FakePaymentMockService` — 内存 Fake 实现（始终成功）

use async_trait::async_trait;

use super::{
    dto::{MockRechargeData, MockRechargeRequest},
    error::PaymentError,
};

/// Dev Mock 充值服务 Trait（T-00055）
#[async_trait]
pub trait PaymentMockServicePort: Send + Sync {
    /// 模拟充值入账
    async fn mock_recharge(
        &self,
        user_id: i64,
        req: MockRechargeRequest,
    ) -> Result<MockRechargeData, PaymentError>;
}

/// 内存 Fake 实现 — 始终成功，不操作 DB
///
/// 用于：
/// - `AppState::new()` 测试构造器
/// - 单元测试中 `Arc<dyn PaymentMockServicePort>` 注入
pub struct FakePaymentMockService;

#[async_trait]
impl PaymentMockServicePort for FakePaymentMockService {
    async fn mock_recharge(
        &self,
        _user_id: i64,
        req: MockRechargeRequest,
    ) -> Result<MockRechargeData, PaymentError> {
        match req.force_outcome.as_str() {
            "fail" => Err(PaymentError::OrderAlreadyFinalized),
            _ => Ok(MockRechargeData {
                order_id: uuid::Uuid::new_v4(),
                state: if req.force_outcome == "pending" {
                    "PENDING".to_string()
                } else {
                    "CREDITED".to_string()
                },
                diamonds_credited: Some(60),
                balance_after: None,
                wallet_transaction_id: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // DM01: FakePaymentMockService 满足 Send + Sync
    #[test]
    fn dm01_fake_mock_service_is_send_sync() {
        let _: Arc<dyn PaymentMockServicePort> = Arc::new(FakePaymentMockService);
    }

    // DM02 (RED→GREEN): mock_recharge 成功返回 CREDITED 状态
    #[tokio::test]
    async fn dm02_mock_recharge_success_outcome() {
        let svc = FakePaymentMockService;
        let req = MockRechargeRequest {
            sku_id: "diamond_600".to_string(),
            force_outcome: "success".to_string(),
            client_note: None,
        };
        let result = svc.mock_recharge(42, req).await.unwrap();
        assert_eq!(result.state, "CREDITED");
    }

    // DM03 (RED→GREEN): force_outcome="fail" 返回错误
    #[tokio::test]
    async fn dm03_mock_recharge_fail_outcome() {
        let svc = FakePaymentMockService;
        let req = MockRechargeRequest {
            sku_id: "diamond_60".to_string(),
            force_outcome: "fail".to_string(),
            client_note: None,
        };
        let result = svc.mock_recharge(1, req).await;
        assert!(result.is_err());
    }

    // DM04 (RED→GREEN): force_outcome="pending" 返回 PENDING 状态
    #[tokio::test]
    async fn dm04_mock_recharge_pending_outcome() {
        let svc = FakePaymentMockService;
        let req = MockRechargeRequest {
            sku_id: "diamond_300".to_string(),
            force_outcome: "pending".to_string(),
            client_note: None,
        };
        let result = svc.mock_recharge(7, req).await.unwrap();
        assert_eq!(result.state, "PENDING");
    }
}
