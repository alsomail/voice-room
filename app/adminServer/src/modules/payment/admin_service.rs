//! T-10026: PaymentAdminService — 补单/退款原子事务
//!
//! 状态机：
//!   recredit: FAILED/CANCELLED → CREDITED（禁止对 ACKED/CREDITED/REFUNDED 再补单）
//!   refund:   ACKED/CREDITED → REFUNDED

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::common::error::AppError;
use crate::modules::event::publisher::{EventPublisher, RawEvent};
use voice_room_shared::events::BalanceUpdatedEvent;
use voice_room_shared::payment::OrderState;

// ─── 结果结构 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RecreditResult {
    pub order_id: Uuid,
    pub new_state: String,
    pub diamonds_credited: i64,
}

#[derive(Debug, Clone)]
pub struct RefundResult {
    pub order_id: Uuid,
    pub new_state: String,
    pub diamonds_deducted: i64,
}

// ─── Repo trait ───────────────────────────────────────────────────────────────

/// 订单 + 钱包 + 审计 原子操作仓库（SELECT FOR UPDATE 锁语义）。
#[async_trait]
pub trait PaymentAdminRepository: Send + Sync {
    /// 获取订单当前状态（用于校验，不加锁；实际原子操作在 recredit/refund_atomic 中做 FOR UPDATE）
    async fn get_order_state(
        &self,
        order_id: Uuid,
    ) -> Result<Option<(OrderState, i64)>, AppError>;
    // ^ returns (state, diamonds)

    /// 原子：更新状态 FAILED/CANCELLED → CREDITED + 余额 + wallet_tx + admin_log
    async fn recredit_atomic(
        &self,
        order_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<RecreditResult, AppError>;

    /// 原子：更新状态 ACKED/CREDITED → REFUNDED + 余额 - diamonds + wallet_tx + admin_log
    async fn refund_atomic(
        &self,
        order_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<RefundResult, AppError>;
}

// ─── Service ─────────────────────────────────────────────────────────────────

pub struct PaymentAdminService {
    pub repo: Arc<dyn PaymentAdminRepository>,
    pub event_publisher: Arc<dyn EventPublisher>,
}

impl PaymentAdminService {
    pub fn new(
        repo: Arc<dyn PaymentAdminRepository>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            repo,
            event_publisher,
        }
    }

    /// 校验 reason 非空。
    pub fn validate_reason(reason: &str) -> Result<(), AppError> {
        if reason.trim().is_empty() {
            Err(AppError::ValidationError(
                "reason must not be empty".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// 执行补单（FAILED/CANCELLED → CREDITED）。
    ///
    /// 幂等：ACKED/CREDITED/REFUNDED 状态的订单返回 40905。
    pub async fn recredit_order(
        &self,
        order_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<RecreditResult, AppError> {
        Self::validate_reason(reason)?;

        // 检查订单当前状态（原子操作内部会再次 FOR UPDATE 校验）
        let (state, _diamonds) = self
            .repo
            .get_order_state(order_id)
            .await?
            .ok_or_else(|| AppError::OrderNotFound(order_id.to_string()))?;

        // 终态检查
        match state {
            OrderState::Acked | OrderState::Credited | OrderState::Refunded => {
                return Err(AppError::OrderAlreadyFinalized);
            }
            OrderState::Failed | OrderState::Cancelled => {
                // OK to recredit
            }
            _ => {
                return Err(AppError::ValidationError(format!(
                    "cannot recredit order in state {}",
                    state
                )));
            }
        }

        // 原子操作
        let result = self.repo.recredit_atomic(order_id, admin_id, reason).await?;

        // Redis PUBLISH（fire-and-forget）
        self.publish_balance_event(admin_id, result.diamonds_credited)
            .await;

        Ok(result)
    }

    /// 执行退款（ACKED/CREDITED → REFUNDED）。
    pub async fn refund_order(
        &self,
        order_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<RefundResult, AppError> {
        Self::validate_reason(reason)?;

        let (state, _diamonds) = self
            .repo
            .get_order_state(order_id)
            .await?
            .ok_or_else(|| AppError::OrderNotFound(order_id.to_string()))?;

        match state {
            OrderState::Acked | OrderState::Credited => {
                // OK to refund
            }
            _ => {
                return Err(AppError::ValidationError(format!(
                    "can only refund ACKED/CREDITED orders, got {}",
                    state
                )));
            }
        }

        let result = self.repo.refund_atomic(order_id, admin_id, reason).await?;

        // Redis PUBLISH（fire-and-forget）
        self.publish_balance_event(admin_id, -result.diamonds_deducted)
            .await;

        Ok(result)
    }

    async fn publish_balance_event(&self, admin_id: Uuid, delta: i64) {
        let payload = BalanceUpdatedEvent {
            user_id: admin_id, // placeholder; real user_id comes from repo
            balance_after: 0,  // placeholder
            delta,
            reason: if delta >= 0 {
                "admin_recredit".to_string()
            } else {
                "admin_refund".to_string()
            },
            ref_id: None,
        };
        let event = RawEvent {
            event_type: "balance_updated".to_string(),
            payload: serde_json::to_value(&payload).unwrap_or_default(),
            admin_id: admin_id.to_string(),
            ts: Utc::now().timestamp(),
        };
        if let Err(e) = self
            .event_publisher
            .publish_raw("admin:events", event)
            .await
        {
            tracing::warn!(error = %e, "payment admin: failed to publish Redis event");
        }
    }
}

// ─── Fake 实现 ────────────────────────────────────────────────────────────────

#[cfg(any(test, feature = "test-utils"))]
#[derive(Default)]
pub struct FakePaymentAdminRepository {
    /// order_id → (state, diamonds, user_id, balance)
    orders: Mutex<Vec<FakeOrderEntry>>,
    /// 记录已执行的补单
    recredit_calls: Mutex<Vec<(Uuid, String)>>,
    /// 记录已执行的退款
    refund_calls: Mutex<Vec<(Uuid, String)>>,
    /// 注入错误到 recredit_atomic
    inject_recredit_error: Mutex<bool>,
}

#[cfg(any(test, feature = "test-utils"))]
#[derive(Clone)]
pub struct FakeOrderEntry {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub state: OrderState,
    pub diamonds: i64,
    pub balance: i64,
}

#[cfg(any(test, feature = "test-utils"))]
impl FakePaymentAdminRepository {
    pub fn seed_order(
        &self,
        order_id: Uuid,
        user_id: Uuid,
        state: OrderState,
        diamonds: i64,
        balance: i64,
    ) {
        self.orders.lock().unwrap().push(FakeOrderEntry {
            order_id,
            user_id,
            state,
            diamonds,
            balance,
        });
    }

    pub fn get_order(&self, order_id: Uuid) -> Option<FakeOrderEntry> {
        self.orders
            .lock()
            .unwrap()
            .iter()
            .find(|o| o.order_id == order_id)
            .cloned()
    }

    pub fn get_recredit_calls(&self) -> Vec<(Uuid, String)> {
        self.recredit_calls.lock().unwrap().clone()
    }

    pub fn get_refund_calls(&self) -> Vec<(Uuid, String)> {
        self.refund_calls.lock().unwrap().clone()
    }

    pub fn set_inject_recredit_error(&self, v: bool) {
        *self.inject_recredit_error.lock().unwrap() = v;
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl PaymentAdminRepository for FakePaymentAdminRepository {
    async fn get_order_state(
        &self,
        order_id: Uuid,
    ) -> Result<Option<(OrderState, i64)>, AppError> {
        let guard = self.orders.lock().unwrap();
        Ok(guard
            .iter()
            .find(|o| o.order_id == order_id)
            .map(|o| (o.state.clone(), o.diamonds)))
    }

    async fn recredit_atomic(
        &self,
        order_id: Uuid,
        _admin_id: Uuid,
        reason: &str,
    ) -> Result<RecreditResult, AppError> {
        if *self.inject_recredit_error.lock().unwrap() {
            return Err(AppError::Internal("injected recredit error".to_string()));
        }
        let mut guard = self.orders.lock().unwrap();
        let entry = guard
            .iter_mut()
            .find(|o| o.order_id == order_id)
            .ok_or_else(|| AppError::OrderNotFound(order_id.to_string()))?;

        let diamonds = entry.diamonds;
        entry.state = OrderState::Credited;
        entry.balance += diamonds;

        self.recredit_calls
            .lock()
            .unwrap()
            .push((order_id, reason.to_string()));

        Ok(RecreditResult {
            order_id,
            new_state: "CREDITED".to_string(),
            diamonds_credited: diamonds,
        })
    }

    async fn refund_atomic(
        &self,
        order_id: Uuid,
        _admin_id: Uuid,
        reason: &str,
    ) -> Result<RefundResult, AppError> {
        let mut guard = self.orders.lock().unwrap();
        let entry = guard
            .iter_mut()
            .find(|o| o.order_id == order_id)
            .ok_or_else(|| AppError::OrderNotFound(order_id.to_string()))?;

        let diamonds = entry.diamonds;
        entry.state = OrderState::Refunded;
        entry.balance -= diamonds;

        self.refund_calls
            .lock()
            .unwrap()
            .push((order_id, reason.to_string()));

        Ok(RefundResult {
            order_id,
            new_state: "REFUNDED".to_string(),
            diamonds_deducted: diamonds,
        })
    }
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

use sqlx::PgPool;

pub struct PgPaymentAdminRepository {
    pool: PgPool,
}

impl PgPaymentAdminRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PaymentAdminRepository for PgPaymentAdminRepository {
    async fn get_order_state(
        &self,
        order_id: Uuid,
    ) -> Result<Option<(OrderState, i64)>, AppError> {
        let row: Option<(OrderState, i64)> = sqlx::query_as(
            "SELECT o.state, s.diamonds \
             FROM payment_orders o \
             JOIN payment_skus s ON o.sku_id = s.sku_id \
             WHERE o.order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn recredit_atomic(
        &self,
        order_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<RecreditResult, AppError> {
        let mut tx = self.pool.begin().await?;

        // SELECT FOR UPDATE — 行锁
        let row: Option<(OrderState, i64, Uuid, String)> = sqlx::query_as(
            "SELECT o.state, s.diamonds, o.user_id, o.sku_id \
             FROM payment_orders o \
             JOIN payment_skus s ON o.sku_id = s.sku_id \
             WHERE o.order_id = $1 FOR UPDATE",
        )
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await?;

        let (state, diamonds, user_id, sku_id) = match row {
            Some(r) => r,
            None => {
                let _ = tx.rollback().await;
                return Err(AppError::OrderNotFound(order_id.to_string()));
            }
        };

        // 终态校验（双重检查）
        match state {
            OrderState::Acked | OrderState::Credited | OrderState::Refunded => {
                let _ = tx.rollback().await;
                return Err(AppError::OrderAlreadyFinalized);
            }
            OrderState::Failed | OrderState::Cancelled => {}
            _ => {
                let _ = tx.rollback().await;
                return Err(AppError::ValidationError(format!(
                    "cannot recredit order in state {}",
                    state
                )));
            }
        }

        let history_entry = serde_json::json!({
            "state": "CREDITED",
            "ts": Utc::now().to_rfc3339(),
            "source": "admin_recredit"
        });

        // UPDATE order state
        sqlx::query(
            "UPDATE payment_orders \
             SET state = 'CREDITED', credited_at = now(), \
                 state_history = state_history || $1::jsonb \
             WHERE order_id = $2",
        )
        .bind(serde_json::json!([history_entry]))
        .bind(order_id)
        .execute(&mut *tx)
        .await?;

        // UPDATE user balance
        sqlx::query(
            "UPDATE users SET diamond_balance = diamond_balance + $1, updated_at = now() \
             WHERE id = $2",
        )
        .bind(diamonds)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Get new balance
        let (new_balance,): (i64,) =
            sqlx::query_as("SELECT diamond_balance FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;

        // INSERT wallet_transactions
        sqlx::query(
            "INSERT INTO wallet_transactions \
               (user_id, type, amount, balance_after, reason) \
             VALUES ($1, 'recharge', $2, $3, $4)",
        )
        .bind(user_id)
        .bind(diamonds)
        .bind(new_balance)
        .bind(format!("admin_recredit: {reason}"))
        .execute(&mut *tx)
        .await?;

        // INSERT admin_logs
        let detail = serde_json::json!({
            "reason": reason,
            "diamonds": diamonds,
            "sku_id": sku_id,
        });
        sqlx::query(
            "INSERT INTO admin_logs (admin_id, action, target_type, target_id, detail) \
             VALUES ($1, 'order_recredit', 'payment_order', $2, $3)",
        )
        .bind(admin_id)
        .bind(order_id)
        .bind(detail)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(RecreditResult {
            order_id,
            new_state: "CREDITED".to_string(),
            diamonds_credited: diamonds,
        })
    }

    async fn refund_atomic(
        &self,
        order_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<RefundResult, AppError> {
        let mut tx = self.pool.begin().await?;

        let row: Option<(OrderState, i64, Uuid, String)> = sqlx::query_as(
            "SELECT o.state, s.diamonds, o.user_id, o.sku_id \
             FROM payment_orders o \
             JOIN payment_skus s ON o.sku_id = s.sku_id \
             WHERE o.order_id = $1 FOR UPDATE",
        )
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await?;

        let (state, diamonds, user_id, sku_id) = match row {
            Some(r) => r,
            None => {
                let _ = tx.rollback().await;
                return Err(AppError::OrderNotFound(order_id.to_string()));
            }
        };

        match state {
            OrderState::Acked | OrderState::Credited => {}
            _ => {
                let _ = tx.rollback().await;
                return Err(AppError::ValidationError(format!(
                    "can only refund ACKED/CREDITED orders, got {}",
                    state
                )));
            }
        }

        let history_entry = serde_json::json!({
            "state": "REFUNDED",
            "ts": Utc::now().to_rfc3339(),
            "source": "admin_refund"
        });

        sqlx::query(
            "UPDATE payment_orders \
             SET state = 'REFUNDED', failed_at = now(), \
                 state_history = state_history || $1::jsonb \
             WHERE order_id = $2",
        )
        .bind(serde_json::json!([history_entry]))
        .bind(order_id)
        .execute(&mut *tx)
        .await?;

        // 余额允许变负（协议规定）
        sqlx::query(
            "UPDATE users SET diamond_balance = diamond_balance - $1, updated_at = now() \
             WHERE id = $2",
        )
        .bind(diamonds)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        let (new_balance,): (i64,) =
            sqlx::query_as("SELECT diamond_balance FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?;

        // 退款 amount 为负值
        sqlx::query(
            "INSERT INTO wallet_transactions \
               (user_id, type, amount, balance_after, reason) \
             VALUES ($1, 'refund', $2, $3, $4)",
        )
        .bind(user_id)
        .bind(-diamonds)
        .bind(new_balance)
        .bind(format!("admin_refund: {reason}"))
        .execute(&mut *tx)
        .await?;

        let detail = serde_json::json!({
            "reason": reason,
            "diamonds": diamonds,
            "sku_id": sku_id,
        });
        sqlx::query(
            "INSERT INTO admin_logs (admin_id, action, target_type, target_id, detail) \
             VALUES ($1, 'order_refund', 'payment_order', $2, $3)",
        )
        .bind(admin_id)
        .bind(order_id)
        .bind(detail)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(RefundResult {
            order_id,
            new_state: "REFUNDED".to_string(),
            diamonds_deducted: diamonds,
        })
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::event::publisher::NoopEventPublisher;

    fn make_service(
        repo: Arc<FakePaymentAdminRepository>,
    ) -> PaymentAdminService {
        PaymentAdminService::new(repo, Arc::new(NoopEventPublisher::default()))
    }

    // ── RC-01: super_admin recredit FAILED → CREDITED, 余额+ ─────────────

    #[tokio::test]
    async fn rc01_recredit_failed_order_succeeds() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let order_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        repo.seed_order(order_id, user_id, OrderState::Failed, 600, 1000);

        let svc = make_service(repo.clone());
        let result = svc
            .recredit_order(order_id, Uuid::new_v4(), "客诉核实")
            .await
            .unwrap();

        assert_eq!(result.new_state, "CREDITED");
        assert_eq!(result.diamonds_credited, 600);

        let entry = repo.get_order(order_id).unwrap();
        assert_eq!(entry.state, OrderState::Credited);
        assert_eq!(entry.balance, 1600, "balance should be 1000+600=1600");
    }

    // ── RC-02: recredit 成功后 recredit_calls 有记录 ──────────────────────

    #[tokio::test]
    async fn rc02_recredit_records_call() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let order_id = Uuid::new_v4();
        repo.seed_order(order_id, Uuid::new_v4(), OrderState::Failed, 600, 0);

        let svc = make_service(repo.clone());
        svc.recredit_order(order_id, Uuid::new_v4(), "test reason")
            .await
            .unwrap();

        let calls = repo.get_recredit_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, "test reason");
    }

    // ── RC-03: reason 为空 → ValidationError ─────────────────────────────

    #[tokio::test]
    async fn rc03_empty_reason_returns_validation_error() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let order_id = Uuid::new_v4();
        repo.seed_order(order_id, Uuid::new_v4(), OrderState::Failed, 600, 0);

        let svc = make_service(repo);
        let err = svc
            .recredit_order(order_id, Uuid::new_v4(), "")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "{err:?}");
    }

    // ── RC-04: recredit 已 CREDITED 订单 → OrderAlreadyFinalized ─────────

    #[tokio::test]
    async fn rc04_recredit_credited_order_returns_already_finalized() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let order_id = Uuid::new_v4();
        repo.seed_order(order_id, Uuid::new_v4(), OrderState::Credited, 600, 0);

        let svc = make_service(repo);
        let err = svc
            .recredit_order(order_id, Uuid::new_v4(), "reason")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::OrderAlreadyFinalized),
            "RC-04: expected OrderAlreadyFinalized, got {err:?}"
        );
    }

    // ── RC-05: recredit 已 ACKED 订单 → OrderAlreadyFinalized ────────────

    #[tokio::test]
    async fn rc05_recredit_acked_returns_finalized() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let order_id = Uuid::new_v4();
        repo.seed_order(order_id, Uuid::new_v4(), OrderState::Acked, 600, 0);

        let svc = make_service(repo);
        let err = svc
            .recredit_order(order_id, Uuid::new_v4(), "reason")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::OrderAlreadyFinalized));
    }

    // ── RC-06: order_id 不存在 → OrderNotFound ───────────────────────────

    #[tokio::test]
    async fn rc06_recredit_nonexistent_order_returns_not_found() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let svc = make_service(repo);
        let err = svc
            .recredit_order(Uuid::new_v4(), Uuid::new_v4(), "reason")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::OrderNotFound(_)));
    }

    // ── RC-07: refund ACKED 订单 → REFUNDED, 余额-，diamonds_deducted 正确

    #[tokio::test]
    async fn rc07_refund_acked_order_succeeds() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let order_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        repo.seed_order(order_id, user_id, OrderState::Acked, 600, 2000);

        let svc = make_service(repo.clone());
        let result = svc
            .refund_order(order_id, Uuid::new_v4(), "退款原因")
            .await
            .unwrap();

        assert_eq!(result.new_state, "REFUNDED");
        assert_eq!(result.diamonds_deducted, 600);

        let entry = repo.get_order(order_id).unwrap();
        assert_eq!(entry.state, OrderState::Refunded);
        assert_eq!(entry.balance, 1400, "balance 2000-600=1400");
    }

    // ── RC-08: refund PENDING 订单 → ValidationError ───────────────────

    #[tokio::test]
    async fn rc08_refund_pending_order_returns_validation_error() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let order_id = Uuid::new_v4();
        repo.seed_order(order_id, Uuid::new_v4(), OrderState::Pending, 600, 0);

        let svc = make_service(repo);
        let err = svc
            .refund_order(order_id, Uuid::new_v4(), "reason")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "RC-08: expected ValidationError for PENDING order"
        );
    }

    // ── RC-09: validate_reason 单元测试 ─────────────────────────────────

    #[test]
    fn rc09_validate_reason_empty_returns_err() {
        assert!(PaymentAdminService::validate_reason("").is_err());
        assert!(PaymentAdminService::validate_reason("   ").is_err());
    }

    #[test]
    fn rc10_validate_reason_nonempty_returns_ok() {
        assert!(PaymentAdminService::validate_reason("valid reason").is_ok());
    }

    // ── RC-11: refund 余额变负（允许）────────────────────────────────────

    #[tokio::test]
    async fn rc11_refund_allows_negative_balance() {
        let repo = Arc::new(FakePaymentAdminRepository::default());
        let order_id = Uuid::new_v4();
        repo.seed_order(order_id, Uuid::new_v4(), OrderState::Credited, 600, 100);

        let svc = make_service(repo.clone());
        let _ = svc
            .refund_order(order_id, Uuid::new_v4(), "reason")
            .await
            .unwrap();

        let entry = repo.get_order(order_id).unwrap();
        assert_eq!(entry.balance, -500, "balance 100-600=-500 (negative allowed)");
    }
}
