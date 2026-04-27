//! `FakeSendGiftService` — 内存测试替身，供 `AppState::for_test()` 注入

use async_trait::async_trait;
use uuid::Uuid;

use super::types::{SendGiftError, SendGiftPayload, SendGiftResult, SendGiftServicePort};

/// `send()` 始终返回 `Ok(SendGiftResult { ... })`；不触碰 DB / Redis / registry。
/// 执行基本参数验证以支持测试。
#[derive(Default)]
pub struct FakeSendGiftService;

#[async_trait]
impl SendGiftServicePort for FakeSendGiftService {
    async fn send(
        &self,
        _sender_id: Uuid,
        _room_id: Uuid,
        payload: SendGiftPayload,
    ) -> Result<SendGiftResult, SendGiftError> {
        // 基本参数验证（与真实服务一致）
        if !(1..=9999).contains(&payload.count) {
            return Err(SendGiftError::InvalidCount);
        }

        Ok(SendGiftResult {
            gift_record_id: Uuid::new_v4(),
            total_price: 0,
            sender_new_balance: 1000,
            receiver_new_charm: 100,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // SGF-01: FakeSendGiftService 满足 Arc<dyn SendGiftServicePort> 约束
    #[test]
    fn sgf01_fake_service_is_send_gift_service_port() {
        let _: Arc<dyn SendGiftServicePort> = Arc::new(FakeSendGiftService);
    }

    // SGF-02: FakeSendGiftService::send 始终返回 Ok
    #[tokio::test]
    async fn sgf02_fake_service_returns_ok() {
        let svc = FakeSendGiftService;
        let result = svc
            .send(
                Uuid::new_v4(),
                Uuid::new_v4(),
                SendGiftPayload {
                    gift_id: Uuid::new_v4(),
                    receiver_id: Uuid::new_v4(),
                    count: 1,
                    msg_id: "test".to_string(),
                },
            )
            .await;
        assert!(result.is_ok());
    }
}
