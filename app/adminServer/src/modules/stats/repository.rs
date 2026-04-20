use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::PgPool;

use crate::common::error::AppError;

// ─── Trait ───────────────────────────────────────────────────────────────────

/// stats 模块数据层抽象，隔离 DB 与测试 Fake。
#[async_trait]
pub trait AdminStatsRepository: Send + Sync {
    /// 统计日期范围内 created_at 在区间内的用户数（新增用户）。
    async fn count_new_users(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<i64, AppError>;

    /// 统计日期范围内 updated_at 在区间内的用户数（近似 DAU）。
    async fn count_dau(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<i64, AppError>;
}

// ─── Postgres 实现 ───────────────────────────────────────────────────────────

/// 基于 SQLx + PostgreSQL 的 AdminStatsRepository 生产实现。
pub struct PgAdminStatsRepository {
    pool: PgPool,
}

impl PgAdminStatsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminStatsRepository for PgAdminStatsRepository {
    async fn count_new_users(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<i64, AppError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users \
             WHERE created_at::date >= $1 AND created_at::date <= $2 AND deleted_at IS NULL",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(count.0)
    }

    async fn count_dau(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<i64, AppError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users \
             WHERE updated_at::date >= $1 AND updated_at::date <= $2 AND deleted_at IS NULL",
        )
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(count.0)
    }
}

// ─── Fake 实现（内存，用于单元 / 集成测试）────────────────────────────────────

/// 内存版 AdminStatsRepository，按 (start, end) 键预置返回值。
#[derive(Default)]
pub struct FakeAdminStatsRepository {
    /// 按 (start, end) 键返回 new_users 值
    pub new_users: HashMap<(NaiveDate, NaiveDate), i64>,
    /// 按 (start, end) 键返回 dau 值
    pub dau: HashMap<(NaiveDate, NaiveDate), i64>,
}

#[async_trait]
impl AdminStatsRepository for FakeAdminStatsRepository {
    async fn count_new_users(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<i64, AppError> {
        Ok(*self.new_users.get(&(start, end)).unwrap_or(&0))
    }

    async fn count_dau(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<i64, AppError> {
        Ok(*self.dau.get(&(start, end)).unwrap_or(&0))
    }
}

// ─── 单元测试（TDD T-10010 Repository 验收用例）──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    // RT-01: 预置 new_users[(2024-01-01, 2024-01-31)] = 56
    // → count_new_users(2024-01-01, 2024-01-31) 返回 Ok(56)
    #[tokio::test]
    async fn rt01_count_new_users_returns_preset_value() {
        let mut repo = FakeAdminStatsRepository::default();
        repo.new_users
            .insert((date("2024-01-01"), date("2024-01-31")), 56);

        let result = repo
            .count_new_users(date("2024-01-01"), date("2024-01-31"))
            .await;
        assert_eq!(result.unwrap(), 56, "RT-01: 预置值 56 应被正确返回");
    }

    // RT-02: 预置 dau[(2024-01-01, 2024-01-31)] = 1234
    // → count_dau(2024-01-01, 2024-01-31) 返回 Ok(1234)
    #[tokio::test]
    async fn rt02_count_dau_returns_preset_value() {
        let mut repo = FakeAdminStatsRepository::default();
        repo.dau
            .insert((date("2024-01-01"), date("2024-01-31")), 1234);

        let result = repo
            .count_dau(date("2024-01-01"), date("2024-01-31"))
            .await;
        assert_eq!(result.unwrap(), 1234, "RT-02: 预置值 1234 应被正确返回");
    }

    // RT-03: 未预置 key 时，count_new_users 与 count_dau 默认返回 Ok(0)
    #[tokio::test]
    async fn rt03_missing_key_returns_zero() {
        let repo = FakeAdminStatsRepository::default();

        let new_users = repo
            .count_new_users(date("2024-01-01"), date("2024-01-31"))
            .await;
        let dau = repo
            .count_dau(date("2024-01-01"), date("2024-01-31"))
            .await;

        assert_eq!(new_users.unwrap(), 0, "RT-03: 未预置时 count_new_users 应返回 0");
        assert_eq!(dau.unwrap(), 0, "RT-03: 未预置时 count_dau 应返回 0");
    }
}
