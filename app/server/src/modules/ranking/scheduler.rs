//! Ranking Scheduler — 日榜/周榜定时归档 + 补偿执行
//!
//! ## 归档策略（缺陷 #3 修复后基于 Asia/Riyadh 时区）
//! - 每日 Riyadh 00:00（=UTC 21:00 前一日）归档前一日日榜
//! - 每周一 Riyadh 00:00 归档上周周榜
//! - 归档目标：`ranking_archive:{type}:{period}:{date}` ZSet（TTL 7 天）
//! - 原 key 保留 48h TTL（客户端过渡期可读）
//!
//! ## 补偿执行（幂等）
//! - 启动时读取 `ranking:last_archive:{type}:{period}` → 上次已归档的日期
//! - 若上次归档日期 < 昨天（Riyadh 日期），循环补偿所有未归档的日期
//! - 每归档成功一天，立即更新 `ranking:last_archive:{type}:{period}`（防重复）
//!
//! ## 使用方式
//! ```rust,no_run
//! // 在 main.rs 启动后调用（非阻塞 tokio 任务）
//! # use tokio::sync::watch;
//! # let redis_url = "redis://127.0.0.1:6379".to_string();
//! # let (tx, shutdown_rx) = watch::channel(false);
//! # let _ =
//! voice_room_server::modules::ranking::scheduler::start_ranking_scheduler(redis_url, shutdown_rx);
//! ```

use std::time::Duration;

use redis::AsyncCommands;
use tokio::sync::watch;

use super::RankingType;
use crate::common::time::riyadh;

/// 归档 key 的 TTL（7 天）
const ARCHIVE_TTL_SECS: u64 = 604_800;

/// 上次归档记录 key 的前缀
const LAST_ARCHIVE_KEY_PREFIX: &str = "ranking:last_archive";

// ─── 核心归档函数（供测试直接调用）────────────────────────────────────────────

/// 将指定日期的日榜归档到 `ranking_archive:{type}:day:{date}`。
///
/// 使用 ZUNIONSTORE 将源 key 数据复制到 archive key。
/// 幂等：若 archive key 已存在则跳过（使用 ZUNIONSTORE 会覆盖，实际上是幂等的）。
///
/// # 参数
/// - `conn`: Redis 多路复用连接
/// - `ty`: 榜单类型（charm/wealth）
/// - `date_str`: 日期字符串，格式 YYYY-MM-DD（通常是昨天的日期）
pub async fn do_archive_day(
    conn: &mut redis::aio::MultiplexedConnection,
    ty: RankingType,
    date_str: &str,
) -> redis::RedisResult<()> {
    let src_key = format!("ranking:{}:day:{}", ty.as_key_segment(), date_str);
    let archive_key = format!("ranking_archive:{}:day:{}", ty.as_key_segment(), date_str);

    // ZUNIONSTORE dest numkeys src
    let count: i64 = redis::cmd("ZUNIONSTORE")
        .arg(&archive_key)
        .arg(1i32)
        .arg(&src_key)
        .query_async(conn)
        .await?;

    tracing::info!(
        ty = %ty.as_key_segment(),
        date = %date_str,
        archive_key = %archive_key,
        entries = count,
        "ranked list archived"
    );

    // 设置 archive key 7 天 TTL
    let _: redis::RedisResult<()> = conn.expire(&archive_key, ARCHIVE_TTL_SECS as i64).await;

    Ok(())
}

/// 将指定周的周榜归档到 `ranking_archive:{type}:week:{week_str}`。
pub async fn do_archive_week(
    conn: &mut redis::aio::MultiplexedConnection,
    ty: RankingType,
    week_str: &str,
) -> redis::RedisResult<()> {
    let src_key = format!("ranking:{}:week:{}", ty.as_key_segment(), week_str);
    let archive_key = format!("ranking_archive:{}:week:{}", ty.as_key_segment(), week_str);

    let count: i64 = redis::cmd("ZUNIONSTORE")
        .arg(&archive_key)
        .arg(1i32)
        .arg(&src_key)
        .query_async(conn)
        .await?;

    tracing::info!(
        ty = %ty.as_key_segment(),
        week = %week_str,
        archive_key = %archive_key,
        entries = count,
        "ranked list (week) archived"
    );

    let _: redis::RedisResult<()> = conn.expire(&archive_key, ARCHIVE_TTL_SECS as i64).await;

    Ok(())
}

// ─── 补偿执行（幂等，启动时调用）─────────────────────────────────────────────

/// 检查并补偿所有未归档的日榜（charm + wealth，幂等）。
///
/// 读取 `ranking:last_archive:{type}:day`，若落后于昨天（Riyadh 日期）则逐日补偿。
pub async fn compensate_day_archives(
    conn: &mut redis::aio::MultiplexedConnection,
) -> anyhow::Result<()> {
    // 缺陷 #3：使用 Riyadh 时区的"昨天"作为补偿终点
    let yesterday_riyadh = (riyadh::now_riyadh() - chrono::Duration::days(1)).date_naive();

    for ty in [RankingType::Charm, RankingType::Wealth] {
        let last_archive_key = format!("{}_{}:day", LAST_ARCHIVE_KEY_PREFIX, ty.as_key_segment());

        // 读取上次归档日期
        let last_str: Option<String> = conn.get(&last_archive_key).await.unwrap_or(None);

        // 计算应从哪一天开始补偿
        let start_date = if let Some(s) = last_str {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                d + chrono::Duration::days(1)
            } else {
                yesterday_riyadh
            }
        } else {
            yesterday_riyadh
        };

        // 逐日归档，直到 yesterday (Riyadh)
        let mut cur = start_date;
        while cur <= yesterday_riyadh {
            let date_str = cur.format("%Y-%m-%d").to_string();
            if let Err(e) = do_archive_day(conn, ty, &date_str).await {
                tracing::warn!(
                    ty = %ty.as_key_segment(),
                    date = %date_str,
                    error = %e,
                    "compensate: failed to archive day, will retry on next startup"
                );
                break; // 一旦失败，停止此类型的补偿（下次启动重试）
            }
            // 更新上次归档记录
            let _: redis::RedisResult<()> = conn.set(&last_archive_key, &date_str).await;
            cur += chrono::Duration::days(1);
        }
    }

    Ok(())
}

// ─── 定时归档 Task ─────────────────────────────────────────────────────────────

/// 启动定时归档任务（非阻塞，内部 tokio::spawn）。
///
/// - 每隔 1 小时检查一次是否需要归档（而非精确 cron，实现简单可靠）
/// - 每天第一次检查到 UTC 日期发生切换时，触发昨天的归档
/// - 接受 `watch::Receiver<bool>` 实现优雅停机
///
/// 生产环境中在 `main.rs` 调用此函数。
pub fn start_ranking_scheduler(
    redis_url: String,
    mut shutdown: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let client = match redis::Client::open(redis_url.clone()) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("ranking scheduler: failed to open redis: {e}");
                return;
            }
        };

        // 启动时执行补偿
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            if let Err(e) = compensate_day_archives(&mut conn).await {
                tracing::warn!("ranking scheduler: compensate failed: {e}");
            }
        }

        // 记录上次已归档的日期（防止同一天重复归档）
        let mut last_archived_day: Option<String> = None;
        let mut last_archived_week: Option<String> = None;

        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(3600)) => {}
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("ranking scheduler: shutdown signal received");
                        return;
                    }
                }
            }

            let mut conn = match client.get_multiplexed_async_connection().await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("ranking scheduler: redis reconnect failed: {e}");
                    continue;
                }
            };

            // 检查是否需要归档昨天的日榜（缺陷 #3：基于 Riyadh 时区计算"昨天"）
            let yesterday_str = riyadh::yesterday_riyadh_str();

            if last_archived_day.as_deref() != Some(&yesterday_str) {
                for ty in [RankingType::Charm, RankingType::Wealth] {
                    if let Err(e) = do_archive_day(&mut conn, ty, &yesterday_str).await {
                        tracing::warn!(
                            "ranking scheduler: archive day {} {} failed: {e}",
                            ty.as_key_segment(),
                            yesterday_str
                        );
                    }
                }
                last_archived_day = Some(yesterday_str);
            }

            // 检查是否需要归档上周的周榜（基于 Riyadh 时区）
            let last_week_str = riyadh::last_week_riyadh_str();

            if last_archived_week.as_deref() != Some(&last_week_str) {
                for ty in [RankingType::Charm, RankingType::Wealth] {
                    if let Err(e) = do_archive_week(&mut conn, ty, &last_week_str).await {
                        tracing::warn!(
                            "ranking scheduler: archive week {} {} failed: {e}",
                            ty.as_key_segment(),
                            last_week_str
                        );
                    }
                }
                last_archived_week = Some(last_week_str);
            }
        }
    })
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use chrono_tz::Asia::Riyadh;

    // SCH-01: archive key 命名格式正确（charm day）
    #[test]
    fn archive_key_format_charm_day() {
        let ty = RankingType::Charm;
        let date = "2026-04-21";
        let archive_key = format!("ranking_archive:{}:day:{}", ty.as_key_segment(), date);
        assert_eq!(archive_key, "ranking_archive:charm:day:2026-04-21");
    }

    // SCH-02: archive key 命名格式正确（wealth week）
    #[test]
    fn archive_key_format_wealth_week() {
        let ty = RankingType::Wealth;
        let week = "2026-17";
        let archive_key = format!("ranking_archive:{}:week:{}", ty.as_key_segment(), week);
        assert_eq!(archive_key, "ranking_archive:wealth:week:2026-17");
    }

    // SCH-03: last_archive key 格式
    #[test]
    fn last_archive_key_format() {
        let key = format!(
            "{}_{}:day",
            LAST_ARCHIVE_KEY_PREFIX,
            RankingType::Charm.as_key_segment()
        );
        assert_eq!(key, "ranking:last_archive_charm:day");
    }

    // SCH-04: 缺陷 #3 — yesterday_riyadh_str 必须基于 Riyadh 时区
    // 当 UTC 当前时刻位于 Riyadh 23:59 与 Riyadh 02:59 (= UTC 21:00 - 24:00) 时，
    // Riyadh 的"昨天"应为 Riyadh 当地日历昨天，不应该等同于 UTC 算出的"昨天"。
    #[test]
    fn sch04_yesterday_uses_riyadh_timezone() {
        // 当前 Riyadh 日 - 1 = 我们的 yesterday_str
        let expected = (riyadh::now_riyadh() - chrono::Duration::days(1))
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(riyadh::yesterday_riyadh_str(), expected);
    }

    // SCH-05: TimeZone trait 已引入即可使用 Riyadh::with_ymd_and_hms（编译期校验）
    #[test]
    fn sch05_riyadh_tz_construction_compiles() {
        let dt = Riyadh.with_ymd_and_hms(2026, 4, 26, 0, 0, 0).single();
        assert!(dt.is_some());
    }
}
