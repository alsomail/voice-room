//! T-10028: 财务报告数据库查询层

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::PgPool;

use crate::common::error::AppError;

// ─── DB 行 ────────────────────────────────────────────────────────────────────

/// 从数据库查出的原始聚合行。
#[derive(Debug, Clone)]
pub struct ReportDbRow {
    /// DATE_TRUNC 结果，格式化为 YYYY-MM-DD 或 YYYY-MM
    pub date: String,
    /// 货币代码（ISO 4217）
    pub currency: String,
    /// 该桶内已结算收入金额总和（USD 直出，或原始金额需 FX）
    pub revenue_sum: f64,
    pub order_count: i64,
    pub refund_count: i64,
    pub refund_sum: f64,
}

// ─── Trait ────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait ReportQueryRepo: Send + Sync {
    /// 按粒度 + 时间范围查询聚合数据。
    ///
    /// 返回按 `(date, currency)` 分组的原始行，
    /// provider != 'mock' 过滤掉 dev 数据。
    async fn aggregate(
        &self,
        granularity: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ReportDbRow>, AppError>;
}

// ─── Fake 实现 ────────────────────────────────────────────────────────────────

#[cfg(any(test, feature = "test-utils"))]
#[derive(Default)]
pub struct FakeReportQuery {
    rows: Arc<Mutex<Vec<ReportDbRow>>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeReportQuery {
    /// 预置数据行（用于单元测试）。
    pub fn seed(&self, row: ReportDbRow) {
        self.rows.lock().unwrap().push(row);
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl ReportQueryRepo for FakeReportQuery {
    async fn aggregate(
        &self,
        _granularity: &str,
        _from: NaiveDate,
        _to: NaiveDate,
    ) -> Result<Vec<ReportDbRow>, AppError> {
        Ok(self.rows.lock().unwrap().clone())
    }
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

pub struct PgReportQuery {
    pool: PgPool,
}

impl PgReportQuery {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// DB 内部行（sqlx FromRow）
#[derive(Debug, sqlx::FromRow)]
struct RawRow {
    pub date: String,
    pub currency: String,
    pub revenue_sum: f64,
    pub order_count: i64,
    pub refund_count: i64,
    pub refund_sum: f64,
}

#[async_trait]
impl ReportQueryRepo for PgReportQuery {
    async fn aggregate(
        &self,
        granularity: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<ReportDbRow>, AppError> {
        // 格式化占位符
        let date_fmt = if granularity == "month" {
            "YYYY-MM"
        } else {
            "YYYY-MM-DD"
        };

        let sql = format!(
            r#"
            SELECT
                to_char(
                    DATE_TRUNC('{granularity}',
                        (created_at AT TIME ZONE 'UTC') AT TIME ZONE 'Asia/Riyadh'
                    ),
                    '{date_fmt}'
                )                            AS date,
                COALESCE(currency, 'USD')    AS currency,
                COALESCE(SUM(CASE WHEN state NOT IN ('REFUNDED','CANCELLED','FAILED')
                                  THEN amount_usd ELSE 0 END), 0)::float8  AS revenue_sum,
                COUNT(*) FILTER (WHERE state NOT IN ('REFUNDED','CANCELLED','FAILED'))
                                             AS order_count,
                COUNT(*) FILTER (WHERE state = 'REFUNDED')
                                             AS refund_count,
                COALESCE(SUM(CASE WHEN state = 'REFUNDED'
                                  THEN amount_usd ELSE 0 END), 0)::float8  AS refund_sum
            FROM payment_orders
            WHERE provider != 'mock'
              AND created_at >= $1::date
              AND created_at <  $2::date + INTERVAL '1 day'
            GROUP BY date, currency
            ORDER BY date, currency
            "#,
            granularity = granularity,
            date_fmt = date_fmt,
        );

        let rows = sqlx::query_as::<_, RawRow>(&sql)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| ReportDbRow {
                date: r.date,
                currency: r.currency,
                revenue_sum: r.revenue_sum,
                order_count: r.order_count,
                refund_count: r.refund_count,
                refund_sum: r.refund_sum,
            })
            .collect())
    }
}
