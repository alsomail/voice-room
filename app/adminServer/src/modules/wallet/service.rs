use std::sync::Arc;

use uuid::Uuid;

use crate::common::error::AppError;
use crate::modules::event::publisher::{AdminEvent, EventPublisher};

use super::repository::WalletRepository;

// ─── WalletService ────────────────────────────────────────────────────────────

/// 钱包业务层：校验输入 → 调用仓库原子性调整 → 发布 Redis 事件（fire-and-forget）。
pub struct WalletService {
    wallet_repo: Arc<dyn WalletRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl WalletService {
    pub fn new(
        wallet_repo: Arc<dyn WalletRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            wallet_repo,
            event_publisher,
        }
    }

    /// 手动调整用户钻石余额。
    ///
    /// # 参数
    /// - `admin_id`：操作者（管理员）ID
    /// - `user_id`：被操作用户 ID
    /// - `amount`：调整量（正=加，负=扣）
    /// - `reason`：调整原因（已在 handler 层完成参数校验）
    ///
    /// # 返回
    /// `(new_balance, delta)` — 调整后余额与变化量（=amount）
    ///
    /// # 错误
    /// - `UserNotFound` → HTTP 404
    /// - `InsufficientBalance` → HTTP 400 / 40204
    /// - `DatabaseError` → HTTP 500（含事务失败场景）
    pub async fn adjust_balance(
        &self,
        admin_id: Uuid,
        user_id: Uuid,
        amount: i64,
        reason: &str,
    ) -> Result<(i64, i64), AppError> {
        // 原子性调整（仓库层负责全部 DB 操作）
        let new_balance = self
            .wallet_repo
            .adjust_balance_atomic(user_id, amount, admin_id, reason)
            .await?;

        // Redis PUBLISH admin:events（fire-and-forget）
        let event = AdminEvent {
            r#type: "balance_updated".to_string(),
            payload: serde_json::json!({
                "user_id":    user_id.to_string(),
                "new_balance": new_balance,
                "delta":       amount,
                "reason":      reason,
            }),
            admin_id: admin_id.to_string(),
            ts: chrono::Utc::now().timestamp(),
        };
        if let Err(e) = self.event_publisher.publish("admin:events", event).await {
            tracing::warn!(error = %e, user_id = %user_id, "wallet adjust: failed to publish Redis event");
        }

        Ok((new_balance, amount))
    }
}

// ─── Service 单元测试 ─────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::event::publisher::NoopEventPublisher;
    use crate::modules::wallet::repository::FakeWalletRepository;

    fn make_service(
        fake_repo: Arc<FakeWalletRepository>,
        fake_pub: Arc<NoopEventPublisher>,
    ) -> WalletService {
        WalletService::new(fake_repo, fake_pub)
    }

    // ── WS-01: 正常加余额 → new_balance 正确，Redis publish 命中 ─────────────
    #[tokio::test]
    async fn ws01_adjust_positive_publishes_event() {
        let user_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        let repo = Arc::new(FakeWalletRepository::default());
        repo.seed_user(user_id, 1000);
        let publisher = Arc::new(NoopEventPublisher::default());

        let svc = make_service(repo.clone(), publisher.clone());
        let (new_balance, delta) = svc
            .adjust_balance(admin_id, user_id, 500, "测试加余额")
            .await
            .unwrap();

        assert_eq!(new_balance, 1500, "WS-01: new_balance 应为 1500");
        assert_eq!(delta, 500, "WS-01: delta 应为 500");

        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "WS-01: Redis publish 应调用 1 次");
        assert_eq!(
            calls[0].0, "admin:events",
            "WS-01: channel 应为 admin:events"
        );
        assert_eq!(
            calls[0].1.r#type, "balance_updated",
            "WS-01: event.type 应为 balance_updated"
        );
    }

    // ── WS-02: 余额不足 → InsufficientBalance，不 publish ─────────────────────
    #[tokio::test]
    async fn ws02_insufficient_balance_no_publish() {
        let user_id = Uuid::new_v4();
        let repo = Arc::new(FakeWalletRepository::default());
        repo.seed_user(user_id, 100);
        let publisher = Arc::new(NoopEventPublisher::default());

        let svc = make_service(repo, publisher.clone());
        let err = svc
            .adjust_balance(Uuid::new_v4(), user_id, -500, "扣减")
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::InsufficientBalance));
        assert_eq!(
            publisher.calls.lock().unwrap().len(),
            0,
            "WS-02: 失败时不应 publish"
        );
    }

    // ── WS-03: 用户不存在 → UserNotFound ─────────────────────────────────────
    #[tokio::test]
    async fn ws03_user_not_found_returns_error() {
        let repo = Arc::new(FakeWalletRepository::default());
        let publisher = Arc::new(NoopEventPublisher::default());

        let svc = make_service(repo, publisher);
        let err = svc
            .adjust_balance(Uuid::new_v4(), Uuid::new_v4(), 100, "test")
            .await
            .unwrap_err();

        assert!(matches!(err, AppError::UserNotFound(_)));
    }

    // ── WS-04: Redis publish 失败（ErrorEventPublisher）不影响主业务返回 ──────
    #[tokio::test]
    async fn ws04_redis_error_does_not_affect_service_result() {
        use crate::modules::event::publisher::ErrorEventPublisher;

        let user_id = Uuid::new_v4();
        let repo = Arc::new(FakeWalletRepository::default());
        repo.seed_user(user_id, 1000);
        let err_pub = Arc::new(ErrorEventPublisher);

        let svc = WalletService::new(repo.clone(), err_pub);
        // 即使 Redis 失败，服务仍应返回成功
        let (new_balance, _) = svc
            .adjust_balance(Uuid::new_v4(), user_id, 200, "test")
            .await
            .unwrap();

        assert_eq!(new_balance, 1200, "WS-04: Redis 失败不影响主业务结果");
    }
}
