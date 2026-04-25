use std::sync::Arc;
use std::time::Duration;

use chrono::{NaiveDate, Utc};
use redis::AsyncCommands;
use tokio::sync::OnceCell;

use crate::common::error::AppError;

use super::{
    dto::{DateRange, StatsOverviewQuery, StatsOverviewResponse},
    repository::AdminStatsRepository,
};

// ─── 日期解析辅助 ─────────────────────────────────────────────────────────────

fn parse_date(s: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| AppError::ValidationError("日期格式须为 YYYY-MM-DD".into()))
}

// ─── Redis Key 常量（必须与 voice-room-server 端 stats::service 保持一致）────
const ONLINE_USERS_KEY: &str = "stats:online_users";
const ACTIVE_ROOMS_KEY: &str = "stats:active_rooms";
const REDIS_OP_TIMEOUT: Duration = Duration::from_millis(500);

// ─── AdminStatsService ────────────────────────────────────────────────────────

pub struct AdminStatsService {
    repo: Arc<dyn AdminStatsRepository>,
    /// 可选的 Redis 连接（OnceCell：测试默认为空，main.rs 启动时注入）。
    /// 未注入或读取失败时 `online_users / active_rooms` 优雅降级为 0。
    redis_conn: OnceCell<redis::aio::MultiplexedConnection>,
}

impl AdminStatsService {
    pub fn new(repo: Arc<dyn AdminStatsRepository>) -> Self {
        Self {
            repo,
            redis_conn: OnceCell::new(),
        }
    }

    /// 使用 Redis URL 初始化在线统计源。失败仅 log warn 并保持降级模式（不返回错误，
    /// 以便启动流程不被弱依赖中断）。
    pub async fn try_init_redis(&self, url: &str) {
        match redis::Client::open(url) {
            Ok(client) => match client.get_multiplexed_async_connection().await {
                Ok(conn) => {
                    if self.redis_conn.set(conn).is_err() {
                        tracing::warn!("AdminStatsService: redis already initialised");
                    } else {
                        tracing::info!("AdminStatsService: redis connection initialised");
                    }
                }
                Err(e) => tracing::warn!(error = %e, "AdminStatsService: redis connect failed (fallback to 0)"),
            },
            Err(e) => tracing::warn!(error = %e, "AdminStatsService: invalid REDIS_URL (fallback to 0)"),
        }
    }

    /// 读取 Redis 在线/活跃房间计数。失败/超时一律 0（绝不阻断概览接口）。
    async fn read_realtime_counts(&self) -> (i64, i64) {
        let Some(conn) = self.redis_conn.get() else {
            return (0, 0);
        };
        let mut conn = conn.clone();

        let online_fut = async {
            let v: redis::RedisResult<i64> = conn.pfcount(ONLINE_USERS_KEY).await;
            v
        };
        let online_users: i64 = match tokio::time::timeout(REDIS_OP_TIMEOUT, online_fut).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => {
                tracing::warn!(error = %e, key = ONLINE_USERS_KEY, "redis pfcount failed");
                0
            }
            Err(_) => {
                tracing::warn!(key = ONLINE_USERS_KEY, "redis pfcount timeout");
                0
            }
        };

        let mut conn2 = self.redis_conn.get().unwrap().clone();
        let rooms_fut = async {
            let v: redis::RedisResult<i64> = conn2.scard(ACTIVE_ROOMS_KEY).await;
            v
        };
        let active_rooms: i64 = match tokio::time::timeout(REDIS_OP_TIMEOUT, rooms_fut).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => {
                tracing::warn!(error = %e, key = ACTIVE_ROOMS_KEY, "redis scard failed");
                0
            }
            Err(_) => {
                tracing::warn!(key = ACTIVE_ROOMS_KEY, "redis scard timeout");
                0
            }
        };

        (online_users, active_rooms)
    }

    /// 获取统计概览。
    ///
    /// - 解析并校验日期参数（缺省今天，start > end 返回 AppError::ValidationError）
    /// - 并发查询 new_users 和 dau（tokio::try_join!）
    /// - active_rooms / online_users：若已注入 Redis，则从 PFCOUNT/SCARD 读取并 500ms 超时降级为 0
    pub async fn get_overview(
        &self,
        query: StatsOverviewQuery,
    ) -> Result<StatsOverviewResponse, AppError> {
        let today = Utc::now().date_naive();

        let start = match &query.start_date {
            Some(s) => parse_date(s)?,
            None => today,
        };

        let end = match &query.end_date {
            Some(s) => parse_date(s)?,
            None => today,
        };

        if start > end {
            return Err(AppError::ValidationError(
                "start_date 不能大于 end_date".into(),
            ));
        }

        let (new_users, dau) = tokio::try_join!(
            self.repo.count_new_users(start, end),
            self.repo.count_dau(start, end)
        )?;

        let (online_users, active_rooms) = self.read_realtime_counts().await;

        Ok(StatsOverviewResponse {
            dau,
            new_users,
            active_rooms,
            online_users,
            date_range: DateRange {
                start: start.format("%Y-%m-%d").to_string(),
                end: end.format("%Y-%m-%d").to_string(),
            },
        })
    }
}

// ─── 单元测试（TDD T-10010 Service 验收用例）─────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::stats::{dto::StatsOverviewQuery, repository::FakeAdminStatsRepository};

    fn make_service() -> AdminStatsService {
        AdminStatsService::new(Arc::new(FakeAdminStatsRepository::default()))
    }

    // ST-01: 正常查询（start_date="2024-01-01", end_date="2024-01-31"）
    // 返回结构正确，active_rooms=0, online_users=0，日期回显与入参一致
    #[tokio::test]
    async fn st01_valid_date_range_returns_correct_structure() {
        let service = make_service();
        let query = StatsOverviewQuery {
            start_date: Some("2024-01-01".to_string()),
            end_date: Some("2024-01-31".to_string()),
        };
        let result = service.get_overview(query).await.unwrap();

        assert_eq!(result.active_rooms, 0, "ST-01: active_rooms MVP 值应为 0");
        assert_eq!(result.online_users, 0, "ST-01: online_users MVP 值应为 0");
        assert_eq!(
            result.date_range.start, "2024-01-01",
            "ST-01: date_range.start 应回显"
        );
        assert_eq!(
            result.date_range.end, "2024-01-31",
            "ST-01: date_range.end 应回显"
        );
    }

    // ST-02: 缺省日期（start_date=None, end_date=None）
    // date_range.start == date_range.end == today（UTC）
    #[tokio::test]
    async fn st02_default_dates_use_today() {
        let service = make_service();
        let query = StatsOverviewQuery {
            start_date: None,
            end_date: None,
        };
        let result = service.get_overview(query).await.unwrap();

        let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
        assert_eq!(result.date_range.start, today, "ST-02: start 默认应为今天");
        assert_eq!(result.date_range.end, today, "ST-02: end 默认应为今天");
    }

    // ST-03: 仅传 start_date（end_date=None）：date_range.end 补全为 today
    #[tokio::test]
    async fn st03_only_start_date_end_defaults_to_today() {
        let service = make_service();
        let today = Utc::now().date_naive();
        let query = StatsOverviewQuery {
            start_date: Some(today.format("%Y-%m-%d").to_string()),
            end_date: None,
        };
        let result = service.get_overview(query).await.unwrap();

        assert_eq!(
            result.date_range.end,
            today.format("%Y-%m-%d").to_string(),
            "ST-03: end 默认应补全为今天"
        );
    }

    // ST-04: 仅传 end_date（start_date=None）：date_range.start 补全为 today
    #[tokio::test]
    async fn st04_only_end_date_start_defaults_to_today() {
        let service = make_service();
        let today = Utc::now().date_naive();
        let query = StatsOverviewQuery {
            start_date: None,
            end_date: Some(today.format("%Y-%m-%d").to_string()),
        };
        let result = service.get_overview(query).await.unwrap();

        assert_eq!(
            result.date_range.start,
            today.format("%Y-%m-%d").to_string(),
            "ST-04: start 默认应补全为今天"
        );
    }

    // ST-05: 日期格式非法（start_date="2024/01/01"）→ AppError::ValidationError
    #[tokio::test]
    async fn st05_invalid_date_format_returns_validation_error() {
        let service = make_service();
        let query = StatsOverviewQuery {
            start_date: Some("2024/01/01".to_string()),
            end_date: None,
        };
        let result = service.get_overview(query).await;

        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "ST-05: 非法日期格式应返回 ValidationError"
        );
    }

    // ST-06: start_date > end_date → AppError::ValidationError
    #[tokio::test]
    async fn st06_start_date_after_end_date_returns_validation_error() {
        let service = make_service();
        let query = StatsOverviewQuery {
            start_date: Some("2024-01-31".to_string()),
            end_date: Some("2024-01-01".to_string()),
        };
        let result = service.get_overview(query).await;

        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "ST-06: start_date > end_date 应返回 ValidationError"
        );
    }

    // ST-07 (P0-3): Redis 未注入时 active_rooms / online_users 优雅降级为 0
    #[tokio::test]
    async fn st07_no_redis_falls_back_to_zero() {
        let service = make_service();
        let query = StatsOverviewQuery {
            start_date: None,
            end_date: None,
        };
        let result = service.get_overview(query).await.unwrap();
        assert_eq!(result.online_users, 0);
        assert_eq!(result.active_rooms, 0);
    }

    // ST-08 (P0-3): 注入非法 REDIS_URL 不 panic，仍返回 0/0
    #[tokio::test]
    async fn st08_invalid_redis_url_does_not_panic() {
        let service = make_service();
        // 非法 URL 应被 try_init_redis 内部 catch 住
        service.try_init_redis("redis://invalid-host-9999:65500").await;
        let result = service
            .get_overview(StatsOverviewQuery {
                start_date: None,
                end_date: None,
            })
            .await
            .unwrap();
        assert_eq!(result.online_users, 0, "无法连接 Redis 时仍应返回 0");
        assert_eq!(result.active_rooms, 0);
    }
}
