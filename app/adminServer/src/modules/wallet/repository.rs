use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::error::AppError;

// ─── 数据行（仅供测试断言）────────────────────────────────────────────────────

/// 内存 Fake 中记录的钱包流水行
#[derive(Debug, Clone)]
pub struct FakeWalletTxRow {
    pub user_id: Uuid,
    pub amount: i64,
    pub balance_after: i64,
    pub operator_id: Uuid,
    pub reason: String,
}

/// 内存 Fake 中记录的 admin_log 行（wallet_adjust）
#[derive(Debug, Clone)]
pub struct FakeWalletAdminLogRow {
    pub admin_id: Uuid,
    pub action: String, // "wallet_adjust"
    pub target_id: Uuid,
    pub amount: i64,
    pub new_balance: i64,
    pub reason: String,
}

// ─── Trait ───────────────────────────────────────────────────────────────────

/// 钱包仓库抽象：原子性执行余额调整（余额更新 + 流水 + admin_log 三步同一事务）。
#[async_trait]
pub trait WalletRepository: Send + Sync {
    /// 查询用户当前 diamond_balance，返回 None 表示用户不存在或已软删除。
    async fn find_user_balance(&self, user_id: Uuid) -> Result<Option<i64>, AppError>;

    /// 原子性调整余额：
    ///   1. SELECT ... FOR UPDATE（锁行）
    ///   2. 验证 new_balance >= 0
    ///   3. UPDATE users SET diamond_balance
    ///   4. INSERT wallet_transactions
    ///   5. INSERT admin_logs
    ///   全部在同一 DB 事务中，任一步失败整体回滚。
    ///
    /// 返回调整后的 new_balance。
    async fn adjust_balance_atomic(
        &self,
        user_id: Uuid,
        amount: i64,
        operator_id: Uuid,
        reason: &str,
    ) -> Result<i64, AppError>;
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

pub struct PgWalletRepository {
    pool: PgPool,
}

impl PgWalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WalletRepository for PgWalletRepository {
    async fn find_user_balance(&self, user_id: Uuid) -> Result<Option<i64>, AppError> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(b,)| b))
    }

    async fn adjust_balance_atomic(
        &self,
        user_id: Uuid,
        amount: i64,
        operator_id: Uuid,
        reason: &str,
    ) -> Result<i64, AppError> {
        let mut tx = self.pool.begin().await?;

        // Step 1: SELECT FOR UPDATE（行锁 + 存在性检查）
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

        let current_balance = match row {
            Some((b,)) => b,
            None => {
                tx.rollback().await.ok();
                return Err(AppError::UserNotFound("用户不存在".to_string()));
            }
        };

        // Step 2: 验证余额充足
        let new_balance = current_balance + amount;
        if new_balance < 0 {
            tx.rollback().await.ok();
            return Err(AppError::InsufficientBalance);
        }

        // Step 3: 更新余额
        sqlx::query("UPDATE users SET diamond_balance = $1, updated_at = now() WHERE id = $2")
            .bind(new_balance)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Step 4: 插入 wallet_transactions
        sqlx::query(
            "INSERT INTO wallet_transactions \
               (user_id, type, amount, balance_after, operator_id, reason) \
             VALUES ($1, 'admin_adjust', $2, $3, $4, $5)",
        )
        .bind(user_id)
        .bind(amount)
        .bind(new_balance)
        .bind(operator_id)
        .bind(reason)
        .execute(&mut *tx)
        .await?;

        // Step 5: 插入 admin_logs（在事务内，任何失败均触发回滚）
        let detail = serde_json::json!({
            "amount": amount,
            "reason": reason,
            "new_balance": new_balance,
            "delta": amount,
        });
        sqlx::query(
            "INSERT INTO admin_logs (admin_id, action, target_type, target_id, detail) \
             VALUES ($1, 'wallet_adjust', 'user', $2, $3)",
        )
        .bind(operator_id)
        .bind(user_id)
        .bind(detail)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(new_balance)
    }
}

// ─── Fake 实现（内存，用于单元/集成测试）─────────────────────────────────────

/// 测试专用：内存 WalletRepository，支持错误注入模拟 admin_log 步骤失败。
pub struct FakeWalletRepository {
    /// user_id → 当前 diamond_balance（初始化为 seed 值）
    balances: Arc<Mutex<HashMap<Uuid, i64>>>,
    /// 已记录的钱包流水
    transactions: Arc<Mutex<Vec<FakeWalletTxRow>>>,
    /// 已记录的 wallet_adjust admin_log（事务内写入）
    admin_logs_written: Arc<Mutex<Vec<FakeWalletAdminLogRow>>>,
    /// 模拟 admin_log INSERT 失败（触发事务回滚），不影响前序验证逻辑
    inject_admin_log_error: Arc<AtomicBool>,
}

impl Default for FakeWalletRepository {
    fn default() -> Self {
        Self {
            balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(vec![])),
            admin_logs_written: Arc::new(Mutex::new(vec![])),
            inject_admin_log_error: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl FakeWalletRepository {
    /// 预置用户余额（供测试使用）
    pub fn seed_user(&self, user_id: Uuid, balance: i64) {
        self.balances.lock().unwrap().insert(user_id, balance);
    }

    /// 获取当前用户余额（供断言使用）
    pub fn get_balance(&self, user_id: Uuid) -> Option<i64> {
        self.balances.lock().unwrap().get(&user_id).copied()
    }

    /// 获取已记录的钱包流水列表（供断言使用）
    pub fn get_transactions(&self) -> Vec<FakeWalletTxRow> {
        self.transactions.lock().unwrap().clone()
    }

    /// 获取已记录的 wallet_adjust admin_logs（供断言使用）
    pub fn get_admin_logs_written(&self) -> Vec<FakeWalletAdminLogRow> {
        self.admin_logs_written.lock().unwrap().clone()
    }

    /// 注入 admin_log 步骤失败，模拟事务回滚（WA08）
    pub fn set_inject_admin_log_error(&self, v: bool) {
        self.inject_admin_log_error.store(v, Ordering::SeqCst);
    }
}

#[async_trait]
impl WalletRepository for FakeWalletRepository {
    async fn find_user_balance(&self, user_id: Uuid) -> Result<Option<i64>, AppError> {
        let guard = self.balances.lock().unwrap();
        Ok(guard.get(&user_id).copied())
    }

    async fn adjust_balance_atomic(
        &self,
        user_id: Uuid,
        amount: i64,
        operator_id: Uuid,
        reason: &str,
    ) -> Result<i64, AppError> {
        // 注意：所有状态修改必须在 inject_admin_log_error 检查之后进行
        // 以正确模拟"事务中 admin_log 失败 → 整体回滚"的原子性。

        let mut balances = self.balances.lock().unwrap();

        // Step 1: 存在性检查
        let current_balance = match balances.get(&user_id) {
            Some(&b) => b,
            None => return Err(AppError::UserNotFound("用户不存在".to_string())),
        };

        // Step 2: 余额充足性校验
        let new_balance = current_balance + amount;
        if new_balance < 0 {
            return Err(AppError::InsufficientBalance);
        }

        // Step 5 预检：模拟 admin_log INSERT 失败（相当于事务在 commit 前回滚）
        // 此时 balances 仍未修改，符合"任一步失败整体回滚"的语义。
        if self.inject_admin_log_error.load(Ordering::SeqCst) {
            return Err(AppError::DatabaseError(
                "injected: admin_log INSERT failed, transaction rolled back".to_string(),
            ));
        }

        // Steps 3-5: 原子性提交（Fake 中模拟为内存操作）
        balances.insert(user_id, new_balance);
        drop(balances);

        // Step 4: 记录流水
        self.transactions.lock().unwrap().push(FakeWalletTxRow {
            user_id,
            amount,
            balance_after: new_balance,
            operator_id,
            reason: reason.to_string(),
        });

        // Step 5: 记录 admin_log（事务内写入）
        self.admin_logs_written
            .lock()
            .unwrap()
            .push(FakeWalletAdminLogRow {
                admin_id: operator_id,
                action: "wallet_adjust".to_string(),
                target_id: user_id,
                amount,
                new_balance,
                reason: reason.to_string(),
            });

        Ok(new_balance)
    }
}

// ─── Repository 单元测试 ──────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // ── WR-01: seed_user + find_user_balance 返回预置值 ──────────────────────
    #[tokio::test]
    async fn wr01_find_user_balance_returns_seeded_value() {
        let repo = FakeWalletRepository::default();
        let uid = Uuid::new_v4();
        repo.seed_user(uid, 1000);

        let result = repo.find_user_balance(uid).await.unwrap();
        assert_eq!(result, Some(1000), "WR-01: 应返回预置的余额 1000");
    }

    // ── WR-02: 用户不存在 → find_user_balance 返回 None ─────────────────────
    #[tokio::test]
    async fn wr02_find_user_balance_returns_none_for_unknown_user() {
        let repo = FakeWalletRepository::default();
        let result = repo.find_user_balance(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none(), "WR-02: 未知用户应返回 None");
    }

    // ── WR-03: adjust_balance_atomic 正数 → 余额增加，流水记录 +1 ───────────
    #[tokio::test]
    async fn wr03_adjust_positive_amount_increases_balance() {
        let repo = FakeWalletRepository::default();
        let uid = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        repo.seed_user(uid, 500);

        let new_balance = repo
            .adjust_balance_atomic(uid, 200, admin_id, "运营补偿")
            .await
            .unwrap();

        assert_eq!(new_balance, 700, "WR-03: 余额应为 700");
        assert_eq!(
            repo.get_balance(uid),
            Some(700),
            "WR-03: get_balance 应反映更新"
        );
        assert_eq!(repo.get_transactions().len(), 1, "WR-03: 流水记录应有 1 条");
        assert_eq!(
            repo.get_admin_logs_written().len(),
            1,
            "WR-03: admin_log 应有 1 条"
        );
    }

    // ── WR-04: adjust_balance_atomic 负数 → 余额减少 ────────────────────────
    #[tokio::test]
    async fn wr04_adjust_negative_amount_decreases_balance() {
        let repo = FakeWalletRepository::default();
        let uid = Uuid::new_v4();
        repo.seed_user(uid, 1000);

        let new_balance = repo
            .adjust_balance_atomic(uid, -300, Uuid::new_v4(), "扣减")
            .await
            .unwrap();

        assert_eq!(new_balance, 700, "WR-04: 余额应为 700");
        assert_eq!(repo.get_balance(uid), Some(700));
    }

    // ── WR-05: 余额不足 → InsufficientBalance，余额不变，流水无新增 ──────────
    #[tokio::test]
    async fn wr05_insufficient_balance_returns_error_no_changes() {
        let repo = FakeWalletRepository::default();
        let uid = Uuid::new_v4();
        repo.seed_user(uid, 500);

        let err = repo
            .adjust_balance_atomic(uid, -1000, Uuid::new_v4(), "扣减")
            .await
            .unwrap_err();

        assert!(
            matches!(err, AppError::InsufficientBalance),
            "WR-05: 应返回 InsufficientBalance，实际: {err:?}"
        );
        assert_eq!(repo.get_balance(uid), Some(500), "WR-05: 余额应保持 500");
        assert_eq!(repo.get_transactions().len(), 0, "WR-05: 无流水记录");
    }

    // ── WR-06: 用户不存在 → UserNotFound ────────────────────────────────────
    #[tokio::test]
    async fn wr06_unknown_user_returns_user_not_found() {
        let repo = FakeWalletRepository::default();
        let err = repo
            .adjust_balance_atomic(Uuid::new_v4(), 100, Uuid::new_v4(), "test")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::UserNotFound(_)),
            "WR-06: 应返回 UserNotFound"
        );
    }

    // ── WR-07: inject_admin_log_error → DatabaseError，余额不变（WA08）───────
    #[tokio::test]
    async fn wr07_inject_admin_log_error_rolls_back_balance() {
        let repo = FakeWalletRepository::default();
        let uid = Uuid::new_v4();
        repo.seed_user(uid, 500);
        repo.set_inject_admin_log_error(true);

        let err = repo
            .adjust_balance_atomic(uid, 100, Uuid::new_v4(), "test")
            .await
            .unwrap_err();

        assert!(
            matches!(err, AppError::DatabaseError(_)),
            "WR-07: 应返回 DatabaseError"
        );
        assert_eq!(
            repo.get_balance(uid),
            Some(500),
            "WR-07: 余额应保持 500（原子回滚）"
        );
        assert_eq!(repo.get_transactions().len(), 0, "WR-07: 无流水记录");
        assert_eq!(
            repo.get_admin_logs_written().len(),
            0,
            "WR-07: 无 admin_log 记录"
        );
    }
}
