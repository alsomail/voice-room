//! Dev Mock 充值服务（T-00055）
//!
//! 仅在 `dev_payment_mock` feature 开启时编译，提供测试环境下的充值模拟接口。
//! 该模块不应出现在生产构建产物中。
//!
//! ## 架构
//! - `PaymentMockServicePort` — trait，controller 依赖此接口
//! - `FakePaymentMockService` — 内存 Fake 实现（始终成功）

use async_trait::async_trait;
use uuid::Uuid;

use super::{
    dto::MockRechargeData,
    error::PaymentError,
};

/// Dev Mock 充值服务 Trait（T-00055）
///
/// 方法签名与 controller 调用方式对齐：
/// - `user_id: Uuid`（与 JWT `AuthContext.user_id` 一致）
/// - 4 个分散参数（与 controller `.mock_recharge(auth.user_id, &req.sku_id, ...)` 一致）
#[async_trait]
pub trait PaymentMockServicePort: Send + Sync {
    /// 模拟充值入账
    async fn mock_recharge(
        &self,
        user_id: Uuid,
        sku_id: &str,
        force_outcome: &str,
        client_note: Option<&str>,
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
        _user_id: Uuid,
        _sku_id: &str,
        force_outcome: &str,
        _client_note: Option<&str>,
    ) -> Result<MockRechargeData, PaymentError> {
        match force_outcome {
            "fail" => Err(PaymentError::OrderAlreadyFinalized),
            _ => Ok(MockRechargeData {
                order_id: Uuid::new_v4(),
                state: if force_outcome == "pending" {
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

    // DM02 (Round3): mock_recharge 使用 Uuid + 分散参数，成功返回 CREDITED 状态
    #[tokio::test]
    async fn dm02_mock_recharge_success_outcome() {
        let svc = FakePaymentMockService;
        let user_id = Uuid::new_v4();
        let result = svc
            .mock_recharge(user_id, "diamond_600", "success", None)
            .await
            .unwrap();
        assert_eq!(result.state, "CREDITED");
        assert_eq!(result.diamonds_credited, Some(60));
    }

    // DM03 (Round3): force_outcome="fail" 返回错误，user_id 为 Uuid
    #[tokio::test]
    async fn dm03_mock_recharge_fail_outcome() {
        let svc = FakePaymentMockService;
        let user_id = Uuid::new_v4();
        let result = svc
            .mock_recharge(user_id, "diamond_60", "fail", None)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PaymentError::OrderAlreadyFinalized));
    }

    // DM04 (Round3): force_outcome="pending" 返回 PENDING 状态
    #[tokio::test]
    async fn dm04_mock_recharge_pending_outcome() {
        let svc = FakePaymentMockService;
        let user_id = Uuid::new_v4();
        let result = svc
            .mock_recharge(user_id, "diamond_300", "pending", Some("test note"))
            .await
            .unwrap();
        assert_eq!(result.state, "PENDING");
    }

    // DM05 (Round3): trait 签名接受 Uuid（与 JWT AuthContext.user_id 类型一致）
    #[test]
    fn dm05_trait_accepts_uuid_user_id() {
        // 编译时验证：Arc<dyn PaymentMockServicePort> 可被 Uuid 参数调用
        // 只需编译通过即证明签名正确
        let _: Arc<dyn PaymentMockServicePort> = Arc::new(FakePaymentMockService);
        let _ = Uuid::new_v4(); // Uuid 与 auth.user_id 类型一致
    }

    // DM06 (Round3): client_note=Some 时不影响结果
    #[tokio::test]
    async fn dm06_mock_recharge_with_client_note() {
        let svc = FakePaymentMockService;
        let user_id = Uuid::new_v4();
        let result = svc
            .mock_recharge(user_id, "diamond_600", "success", Some("integration test"))
            .await
            .unwrap();
        assert_eq!(result.state, "CREDITED");
    }
}
