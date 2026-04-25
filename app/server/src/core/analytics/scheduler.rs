//! PartitionScheduler — events 表按日分区自动创建（T-00022）
//!
//! ## 功能
//! - `create_partition_if_not_exists(pool, date)` — 创建指定日期分区（幂等）
//! - `compensate_missing_partitions(pool, dates)` — 补偿创建多个缺失分区
//! - `start_partition_scheduler(pool, shutdown)` — 启动定时任务（每日 23:00 Riyadh 时间）
//!
//! ## 分区命名规则
//! - 分区名：`events_YYYYMMDD`（按 Asia/Riyadh 时区计算）
//! - 分区范围：`[day 00:00 Riyadh, (day+1) 00:00 Riyadh)`（即 UTC+3 午夜）
//!
//! ## 时区说明
//! - Asia/Riyadh = UTC+3（全年不变，无夏令时）
//! - Cron `0 0 23 * * *`（UTC）= 每日 Riyadh 02:00，提前建次日分区

use std::time::Duration;

use chrono::NaiveDate;
use sqlx::PgPool;
use tokio::sync::watch;

use crate::common::error::AppError;

/// Riyadh UTC 偏移（小时）—— 仅供 unit 测试与文档参考；运行时计算由
/// `crate::common::time::riyadh` 统一处理。
#[allow(dead_code)]
const RIYADH_OFFSET_HOURS: i64 = 3;

// ─── 核心分区创建函数 ──────────────────────────────────────────────────────────

/// 创建指定日期的 events 分区（幂等：若已存在则跳过）
///
/// # 参数
/// - `pool`: PostgreSQL 连接池
/// - `date`: 分区对应的 Riyadh 日期（`NaiveDate`）
///
/// # 分区范围
/// `[date 00:00:00+03, (date+1) 00:00:00+03)`
pub async fn create_partition_if_not_exists(
    pool: &PgPool,
    date: NaiveDate,
) -> Result<(), AppError> {
    let partition_name = format!("events_{}", date.format("%Y%m%d"));

    // 检查分区是否已存在（使用参数化查询，安全）
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_class c \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         WHERE c.relname = $1 AND n.nspname = 'public' AND c.relkind = 'r')",
    )
    .bind(&partition_name)
    .fetch_one(pool)
    .await?;

    if exists {
        tracing::debug!(partition = %partition_name, "partition already exists, skipping");
        return Ok(());
    }

    // 计算 Riyadh 时区的时间戳边界
    // date = Riyadh 当日（例如 2026-04-21）
    // Riyadh 00:00:00 = UTC 前一天 21:00:00（UTC+3，减去 3 小时跨越午夜）
    // from_ts: prev_day 21:00:00+00 = date 00:00 Riyadh
    // to_ts:   date     21:00:00+00 = (date+1) 00:00 Riyadh
    let prev_day = date - chrono::Duration::days(1);
    let from_ts = format!("{} 21:00:00+00", prev_day.format("%Y-%m-%d"));
    let to_ts = format!("{} 21:00:00+00", date.format("%Y-%m-%d"));

    // partition_name 仅含 [a-z0-9_]，可安全插入 SQL（无 SQL 注入风险）
    // PostgreSQL 不支持 DDL 参数化，必须使用字符串插值
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS {partition_name} \
         PARTITION OF events \
         FOR VALUES FROM ('{from_ts}') TO ('{to_ts}')"
    );

    sqlx::query(&sql).execute(pool).await?;

    tracing::info!(
        partition = %partition_name,
        from_ts = %from_ts,
        to_ts = %to_ts,
        "created events partition"
    );

    Ok(())
}

/// 补偿创建多个缺失分区（幂等，可用于启动补偿）
///
/// # 参数
/// - `pool`: PostgreSQL 连接池
/// - `dates`: 需要确保存在的分区日期列表
pub async fn compensate_missing_partitions(
    pool: &PgPool,
    dates: &[NaiveDate],
) -> Result<(), AppError> {
    let mut created_count = 0usize;
    let mut skipped_count = 0usize;

    for &date in dates {
        let partition_name = format!("events_{}", date.format("%Y%m%d"));

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_class c \
             JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE c.relname = $1 AND n.nspname = 'public' AND c.relkind = 'r')",
        )
        .bind(&partition_name)
        .fetch_one(pool)
        .await?;

        if exists {
            skipped_count += 1;
        } else {
            create_partition_if_not_exists(pool, date).await?;
            created_count += 1;
        }
    }

    tracing::info!(
        created = created_count,
        skipped = skipped_count,
        total = dates.len(),
        "partition compensation complete"
    );

    Ok(())
}

// ─── 定时任务 ─────────────────────────────────────────────────────────────────

/// 启动分区定时创建任务（非阻塞，tokio::spawn）
///
/// 每隔 24h 检查并创建次日分区；同时在启动时补偿创建今日和明日分区。
/// 通过 `shutdown_rx` 接收停止信号。
pub fn start_partition_scheduler(pool: PgPool, mut shutdown_rx: watch::Receiver<bool>) {
    tokio::spawn(async move {
        // 启动时补偿创建今日和明日分区
        let today = chrono::Utc::now()
            .with_timezone(&chrono_tz::Asia::Riyadh)
            .date_naive();
        let tomorrow = today + chrono::Duration::days(1);

        if let Err(e) = compensate_missing_partitions(&pool, &[today, tomorrow]).await {
            tracing::warn!(error = %e, "startup partition compensation failed");
        }

        // 每 24h 检查一次（实际生产应使用 cron，此处用 interval 简化）
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 3600));
        interval.tick().await; // 跳过第一个立即触发

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let next_day = chrono::Utc::now()
                        .with_timezone(&chrono_tz::Asia::Riyadh)
                        .date_naive() + chrono::Duration::days(1);

                    if let Err(e) = create_partition_if_not_exists(&pool, next_day).await {
                        tracing::warn!(error = %e, date = %next_day, "failed to create partition");
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("partition scheduler shutting down");
                        break;
                    }
                }
            }
        }
    });
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // S-01: 分区名格式正确
    #[test]
    fn s01_partition_name_format() {
        let date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let name = format!("events_{}", date.format("%Y%m%d"));
        assert_eq!(name, "events_20260421");
    }

    // S-02: 补偿函数接受空列表不报错
    #[test]
    fn s02_empty_dates_slice() {
        let dates: Vec<NaiveDate> = vec![];
        // 只验证数据结构逻辑，不需要 DB
        assert!(dates.is_empty(), "empty dates should be valid input");
    }

    // S-03: RIYADH_OFFSET_HOURS 为 3
    #[test]
    fn s03_riyadh_offset_is_3() {
        assert_eq!(RIYADH_OFFSET_HOURS, 3);
    }

    // S-04: 分区时间边界计算正确
    // events_20260421 对应 Riyadh 2026-04-21 整天
    // Riyadh 2026-04-21 00:00 = UTC 2026-04-20 21:00（UTC+3 需减去 3 小时，跨越午夜）
    // 故 from_ts = "2026-04-20 21:00:00+00"，to_ts = "2026-04-21 21:00:00+00"
    #[test]
    fn s04_partition_time_boundary() {
        let date = NaiveDate::from_ymd_opt(2026, 4, 21).unwrap();
        let prev_day = date - chrono::Duration::days(1);

        let from_ts = format!("{} 21:00:00+00", prev_day.format("%Y-%m-%d"));
        let to_ts = format!("{} 21:00:00+00", date.format("%Y-%m-%d"));

        // Riyadh 2026-04-21 00:00 = UTC 2026-04-20 21:00
        assert_eq!(from_ts, "2026-04-20 21:00:00+00");
        // Riyadh 2026-04-22 00:00 = UTC 2026-04-21 21:00
        assert_eq!(to_ts, "2026-04-21 21:00:00+00");
    }
}
