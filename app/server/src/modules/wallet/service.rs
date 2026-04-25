//! WalletService — 余额查询、流水列表与余额变更
//!
//! - `WalletServicePort` trait：供 HTTP handler 注入，支持 FakeWalletService 测试替身
//! - `WalletService`：真实 PgPool 实现
//!   - `apply_delta`：接受外部事务 `&mut Transaction<'_, Postgres>`，不自行 begin/commit
//!   - `notify_balance_updated`：事务提交后由调用方调用，通知广播器（带 warn 日志）
//! - `FakeWalletService`：`#[cfg(test)]` 内存替身，供 `AppState::for_test()` 注入

use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};
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

    /// 余额变更：在外部事务内执行 SELECT FOR UPDATE + UPDATE + INSERT。
    ///
    /// **调用方负责事务生命周期**（begin / commit / rollback）。
    /// 事务提交成功后，调用方应调用 `notify_balance_updated` 触发 WS 推送。
    ///
    /// # 设计意图
    /// 接受外部事务允许 T-00020 SendGift 等业务在同一 DB 事务内原子完成
    /// "余额扣减 + 礼物记录写入"，彻底防止跨表数据不一致。
    ///
    /// # 错误
    /// - `AppError::NotFound`：用户不存在
    /// - `AppError::ValidationError("insufficient balance")`：变更后余额 < 0
    /// - `AppError::DatabaseError`：DB 操作失败
    ///
    /// 出错时事务由调用方 drop 触发 ROLLBACK，本函数不 commit。
    #[allow(clippy::too_many_arguments)]
    pub async fn apply_delta<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        user_id: Uuid,
        delta: i64,
        ty: WalletTxnType,
        ref_id: Option<Uuid>,
        reason: Option<String>,
        operator_id: Option<Uuid>,
    ) -> Result<i64, AppError> {
        // SELECT ... FOR UPDATE 行锁防并发
        let current: i64 = sqlx::query_scalar(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut **txn)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

        let new_balance = current + delta;
        if new_balance < 0 {
            // 事务由调用方 drop 触发 ROLLBACK
            return Err(AppError::ValidationError(
                "insufficient balance".to_string(),
            ));
        }

        // 更新用户余额
        sqlx::query("UPDATE users SET diamond_balance = $1, updated_at = now() WHERE id = $2")
            .bind(new_balance)
            .bind(user_id)
            .execute(&mut **txn)
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
        .execute(&mut **txn)
        .await?;

        Ok(new_balance)
    }

    /// 在事务提交成功后调用，通知 BalanceBroadcaster 发送 WS 推送。
    ///
    /// # MEDIUM-1 修复
    /// `try_send` 失败时记录 `warn!` 日志（channel 满时事件丢失但不影响主流程）。
    pub fn notify_balance_updated(&self, event: BalanceEvent) {
        if let Err(e) = self.balance_tx.try_send(event) {
            tracing::warn!(
                "BalanceEvent channel full, WS push dropped (event lost): {:?}",
                e
            );
        }
    }
}

/// 将 WalletTxnType 转换为 snake_case 字符串（用于 reason 字段默认值）。
///
/// T-00020 SendGift 等调用 `apply_delta` 的业务可使用此函数构造 reason 参数。
pub fn txn_type_to_str(ty: &WalletTxnType) -> String {
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
    use std::time::Duration;

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

    // WS05: notify_balance_updated 成功时事件被发送到 channel
    #[tokio::test]
    async fn ws05_notify_balance_updated_sends_event() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<BalanceEvent>(10);
        // 创建 WalletService（pool 用不到，只测 notify 路径）
        // 注意：不会实际连接 DB，只测 channel 逻辑
        let fake_pool = {
            // 使用环境变量提供的 DB URL，如果没有则跳过
            let url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/test".to_string());
            match sqlx::PgPool::connect_lazy(&url) {
                Ok(p) => p,
                Err(_) => {
                    eprintln!("[SKIP] ws05: cannot create PgPool");
                    return;
                }
            }
        };
        let svc = WalletService::new(fake_pool, tx);

        let user_id = Uuid::new_v4();
        svc.notify_balance_updated(BalanceEvent {
            user_id,
            balance_after: 500,
            delta: 500,
            reason: "recharge".to_string(),
            ref_id: None,
        });

        let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(event.user_id, user_id);
        assert_eq!(event.balance_after, 500);
        assert_eq!(event.delta, 500);
    }

    // WS06: notify_balance_updated channel 满时记录 warn 日志（不 panic）
    #[tokio::test]
    async fn ws06_notify_balance_updated_channel_full_no_panic() {
        // channel 容量为 0（unbounded send 不能用；用 capacity=0 的有界 channel 测试满）
        let (tx, _rx) = tokio::sync::mpsc::channel::<BalanceEvent>(1);

        // 先填满 channel
        let _ = tx.try_send(BalanceEvent {
            user_id: Uuid::new_v4(),
            balance_after: 0,
            delta: 0,
            reason: "fill".to_string(),
            ref_id: None,
        });

        let fake_pool = {
            let url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/test".to_string());
            match sqlx::PgPool::connect_lazy(&url) {
                Ok(p) => p,
                Err(_) => return,
            }
        };
        let svc = WalletService::new(fake_pool, tx);

        // channel 已满，notify 不应 panic（应记录 warn）
        svc.notify_balance_updated(BalanceEvent {
            user_id: Uuid::new_v4(),
            balance_after: 100,
            delta: 100,
            reason: "overflow".to_string(),
            ref_id: None,
        });
        // 如果能运行到这里说明没有 panic
    }
}
