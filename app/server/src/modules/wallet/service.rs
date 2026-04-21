//! WalletService — 余额查询、流水列表与余额变更
//!
//! - `WalletServicePort` trait：供 HTTP handler 注入，支持 FakeWalletService 测试替身
//! - `WalletService`：真实 PgPool 实现，`apply_delta` 管理完整事务生命周期
//! - `FakeWalletService`：`#[cfg(test)]` 内存替身，供 `AppState::for_test()` 注入

use async_trait::async_trait;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;
use voice_room_shared::models::wallet::{WalletTransactionModel, WalletTxnType};

use crate::common::error::AppError;

use super::broadcaster::BalanceEvent;
use super::dto::Paginated;

// ─── WalletServicePort trait ──────────────────────────────────────────────────

/// HTTP handler 依赖的钱包服务抽象，隔离真实 DB 与测试 Fake。
#[async_trait]
pub trait WalletServicePort: Send + Sync {
    /// 查询用户当前 diamond_balance
    async fn get_balance(&self, user_id: Uuid) -> Result<i64, AppError>;

    /// 分页查询用户流水（按 created_at DESC），支持可选类型过滤
    async fn list_txns(
        &self,
        user_id: Uuid,
        page: u32,
        size: u32,
        ty: Option<WalletTxnType>,
    ) -> Result<Paginated<WalletTransactionModel>, AppError>;
}

// ─── WalletService ────────────────────────────────────────────────────────────

/// 钱包服务实现（PgPool + BalanceEvent channel）
pub struct WalletService {
    pool: PgPool,
    balance_tx: mpsc::Sender<BalanceEvent>,
}

impl WalletService {
    /// 创建 WalletService。
    /// `balance_tx` 由调用方管理（通常与 `BalanceBroadcaster::run(rx)` 配对）。
    pub fn new(pool: PgPool, balance_tx: mpsc::Sender<BalanceEvent>) -> Self {
        Self { pool, balance_tx }
    }

    /// 余额变更：在独立事务中执行 SELECT FOR UPDATE + UPDATE + INSERT，
    /// 事务提交后通过 channel 通知 BalanceBroadcaster。
    ///
    /// # 错误
    /// - `AppError::NotFound`：用户不存在
    /// - `AppError::ValidationError("insufficient balance")`：变更后余额 < 0（事务回滚）
    /// - `AppError::DatabaseError`：DB 操作失败
    pub async fn apply_delta(
        &self,
        user_id: Uuid,
        delta: i64,
        ty: WalletTxnType,
        ref_id: Option<Uuid>,
        reason: Option<String>,
        operator_id: Option<Uuid>,
    ) -> Result<i64, AppError> {
        let mut txn = self.pool.begin().await?;

        // SELECT ... FOR UPDATE 行锁防并发
        let current: i64 = sqlx::query_scalar(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut *txn)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

        let new_balance = current + delta;
        if new_balance < 0 {
            // 事务自动回滚（txn 离开作用域时 drop 触发 ROLLBACK）
            return Err(AppError::ValidationError(
                "insufficient balance".to_string(),
            ));
        }

        // 更新用户余额
        sqlx::query(
            "UPDATE users SET diamond_balance = $1, updated_at = now() WHERE id = $2",
        )
        .bind(new_balance)
        .bind(user_id)
        .execute(&mut *txn)
        .await?;

        // 写入流水记录
        sqlx::query(
            "INSERT INTO wallet_transactions \
             (user_id, type, amount, balance_after, ref_id, reason, operator_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(user_id)
        .bind(&ty)
        .bind(delta)
        .bind(new_balance)
        .bind(ref_id)
        .bind(&reason)
        .bind(operator_id)
        .execute(&mut *txn)
        .await?;

        // 提交事务
        txn.commit().await?;

        // 事务成功后，异步通知广播器（fire-and-forget，失败不影响主流程）
        let event_reason = reason.unwrap_or_else(|| txn_type_to_str(&ty));
        let _ = self.balance_tx.try_send(BalanceEvent {
            user_id,
            balance_after: new_balance,
            delta,
            reason: event_reason,
            ref_id,
        });

        Ok(new_balance)
    }
}

/// 将 WalletTxnType 转换为 snake_case 字符串（用于 reason 默认值）
pub(crate) fn txn_type_to_str(ty: &WalletTxnType) -> String {
    match ty {
        WalletTxnType::GiftSend => "gift_send".to_string(),
        WalletTxnType::GiftReceive => "gift_receive".to_string(),
        WalletTxnType::AdminAdjust => "admin_adjust".to_string(),
        WalletTxnType::Recharge => "recharge".to_string(),
        WalletTxnType::Refund => "refund".to_string(),
    }
}

// ─── WalletServicePort impl for WalletService ────────────────────────────────

#[async_trait]
impl WalletServicePort for WalletService {
    async fn get_balance(&self, user_id: Uuid) -> Result<i64, AppError> {
        let balance = sqlx::query_scalar::<_, i64>(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;
        Ok(balance)
    }

    async fn list_txns(
        &self,
        user_id: Uuid,
        page: u32,
        size: u32,
        ty: Option<WalletTxnType>,
    ) -> Result<Paginated<WalletTransactionModel>, AppError> {
        let offset = ((page - 1) * size) as i64;
        let limit = size as i64;

        let (items, total) = if let Some(ref txn_type) = ty {
            let total = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM wallet_transactions WHERE user_id = $1 AND type = $2",
            )
            .bind(user_id)
            .bind(txn_type)
            .fetch_one(&self.pool)
            .await?;

            let items = sqlx::query_as::<_, WalletTransactionModel>(
                "SELECT id, user_id, type, amount, balance_after, ref_id, reason, \
                 operator_id, created_at \
                 FROM wallet_transactions WHERE user_id = $1 AND type = $2 \
                 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
            )
            .bind(user_id)
            .bind(txn_type)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

            (items, total)
        } else {
            let total = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM wallet_transactions WHERE user_id = $1",
            )
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

            let items = sqlx::query_as::<_, WalletTransactionModel>(
                "SELECT id, user_id, type, amount, balance_after, ref_id, reason, \
                 operator_id, created_at \
                 FROM wallet_transactions WHERE user_id = $1 \
                 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

            (items, total)
        };

        Ok(Paginated {
            total: total as u64,
            page,
            size,
            items,
        })
    }
}

// ─── FakeWalletService（仅测试）──────────────────────────────────────────────

/// 内存测试替身，供 `AppState::for_test()` 注入。
/// - `get_balance` 始终返回 0
/// - `list_txns` 始终返回空分页
#[cfg(any(test, feature = "test-utils"))]
pub struct FakeWalletService;

#[cfg(any(test, feature = "test-utils"))]
impl Default for FakeWalletService {
    fn default() -> Self {
        Self
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl WalletServicePort for FakeWalletService {
    async fn get_balance(&self, _user_id: Uuid) -> Result<i64, AppError> {
        Ok(0)
    }

    async fn list_txns(
        &self,
        _user_id: Uuid,
        page: u32,
        size: u32,
        _ty: Option<WalletTxnType>,
    ) -> Result<Paginated<WalletTransactionModel>, AppError> {
        Ok(Paginated {
            total: 0,
            page,
            size,
            items: Vec::new(),
        })
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    // WS01: FakeWalletService 返回 balance=0
    #[tokio::test]
    async fn ws01_fake_wallet_service_returns_zero_balance() {
        let svc = FakeWalletService;
        let balance = svc.get_balance(Uuid::new_v4()).await.unwrap();
        assert_eq!(balance, 0);
    }

    // WS02: FakeWalletService 返回空 paginated
    #[tokio::test]
    async fn ws02_fake_wallet_service_returns_empty_txns() {
        let svc = FakeWalletService;
        let result = svc.list_txns(Uuid::new_v4(), 1, 20, None).await.unwrap();
        assert_eq!(result.total, 0);
        assert!(result.items.is_empty());
    }

    // WS03: txn_type_to_str 覆盖所有变体
    #[test]
    fn ws03_txn_type_to_str_all_variants() {
        assert_eq!(txn_type_to_str(&WalletTxnType::GiftSend), "gift_send");
        assert_eq!(txn_type_to_str(&WalletTxnType::GiftReceive), "gift_receive");
        assert_eq!(txn_type_to_str(&WalletTxnType::AdminAdjust), "admin_adjust");
        assert_eq!(txn_type_to_str(&WalletTxnType::Recharge), "recharge");
        assert_eq!(txn_type_to_str(&WalletTxnType::Refund), "refund");
    }

    // WS04: Arc<FakeWalletService> 满足 WalletServicePort 的 Send+Sync+dyn 约束
    #[test]
    fn ws04_fake_service_is_send_sync() {
        let _svc: Arc<dyn WalletServicePort> = Arc::new(FakeWalletService);
    }
}
