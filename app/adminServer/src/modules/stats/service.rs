use std::sync::Arc;

use chrono::{NaiveDate, Utc};

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

// ─── AdminStatsService ────────────────────────────────────────────────────────

pub struct AdminStatsService {
    repo: Arc<dyn AdminStatsRepository>,
}

impl AdminStatsService {
    pub fn new(repo: Arc<dyn AdminStatsRepository>) -> Self {
        Self { repo }
    }

    /// 获取统计概览。
    ///
    /// - 解析并校验日期参数（缺省今天，start > end 返回 AppError::ValidationError）
    /// - 并发查询 new_users 和 dau（tokio::try_join!）
    /// - active_rooms / online_users MVP 固定为 0
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

        // TODO(T-10011): 接入 Redis 后替换，从 App Server 维护的在线集合读取 SCARD online:users
        let online_users: i64 = 0;

        // TODO(T-10011): 接入 Redis 后替换，从 App Server 维护的活跃房间集合读取 SCARD rooms:active
        let active_rooms: i64 = 0;

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
    use crate::modules::stats::{
        dto::StatsOverviewQuery, repository::FakeAdminStatsRepository,
    };

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
        assert_eq!(result.date_range.start, "2024-01-01", "ST-01: date_range.start 应回显");
        assert_eq!(result.date_range.end, "2024-01-31", "ST-01: date_range.end 应回显");
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
        assert_eq!(
            result.date_range.start, today,
            "ST-02: start 默认应为今天"
        );
        assert_eq!(
            result.date_range.end, today,
            "ST-02: end 默认应为今天"
        );
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
}
