use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::common::error::AppError;

use super::query_dto::{EventFilter, EventItem, EventQueryParams, EventQueryResponse};
use super::query_repo::EventQueryRepository;

// ─── EventQueryService ────────────────────────────────────────────────────────

/// 用户事件查询业务层。
///
/// 职责：
/// 1. 参数解析与验证（时间窗 ≤ 30 天，limit ≤ 100）
/// 2. event_name 逗号分隔 → Vec<String>
/// 3. 角色权限衍生（非 super_admin → 过滤 admin_* 事件）
/// 4. 调用 EventQueryRepository 分页查询
/// 5. 组装 EventQueryResponse
pub struct EventQueryService {
    repo: Arc<dyn EventQueryRepository>,
}

impl EventQueryService {
    pub fn new(repo: Arc<dyn EventQueryRepository>) -> Self {
        Self { repo }
    }

    /// 查询指定用户的事件列表。
    ///
    /// # 参数
    /// - `user_id`：目标用户 ID（已由 handler 层验证存在性）
    /// - `params`：HTTP query string 解析出的参数
    /// - `filter_admin_events`：是否过滤 `admin_` 前缀事件
    ///   - `true` if role != "super_admin"（non-super cannot see admin audits）
    ///   - `false` if role == "super_admin"
    ///
    /// # 错误
    /// - `ValidationError(40003)`: 时间窗 > 30 天 / limit > 100 / 非法时间格式
    pub async fn query_events(
        &self,
        user_id: Uuid,
        params: EventQueryParams,
        filter_admin_events: bool,
    ) -> Result<EventQueryResponse, AppError> {
        let now = Utc::now();

        // ── 解析 to（默认当前时间）──────────────────────────────────────────
        let to = if let Some(to_str) = &params.to {
            to_str
                .parse::<chrono::DateTime<Utc>>()
                .map_err(|_| {
                    AppError::ValidationError(format!("invalid 'to': '{}'", to_str))
                })?
        } else {
            now
        };

        // ── 解析 from（默认 24h 前）────────────────────────────────────────
        let from = if let Some(from_str) = &params.from {
            from_str
                .parse::<chrono::DateTime<Utc>>()
                .map_err(|_| {
                    AppError::ValidationError(format!("invalid 'from': '{}'", from_str))
                })?
        } else {
            now - Duration::hours(24)
        };

        // ── 时间窗校验：>30 天 → 400/40003 ───────────────────────────────
        if to - from > Duration::days(30) {
            return Err(AppError::ValidationError(
                "time window exceeds 30 days".to_string(),
            ));
        }

        // ── limit 校验：>100 → 400/40003 ──────────────────────────────────
        let limit_raw = params.limit.unwrap_or(20);
        if limit_raw > 100 {
            return Err(AppError::ValidationError(
                "limit must be <= 100".to_string(),
            ));
        }
        let limit = limit_raw.max(1);
        let page = params.page.unwrap_or(1).max(1);
        let offset = ((page - 1) as i64) * (limit as i64);

        // ── event_name 逗号分隔 → Option<Vec<String>> ─────────────────────
        let event_names: Option<Vec<String>> = params.event_name.as_ref().map(|s| {
            s.split(',')
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty())
                .collect()
        });

        // ── 角色过滤 admin_* 事件名 ────────────────────────────────────────
        // 若非 super_admin，从 event_names 过滤掉 admin_* 前缀条目
        let event_names = if filter_admin_events {
            event_names.map(|names| {
                names
                    .into_iter()
                    .filter(|n| !n.starts_with("admin_"))
                    .collect::<Vec<_>>()
            })
        } else {
            event_names
        };

        let filter = EventFilter {
            from,
            to,
            event_names,
            exclude_admin_prefix: filter_admin_events,
        };

        // ── 查询仓库 ───────────────────────────────────────────────────────
        let total = self.repo.count_events(user_id, &filter).await?;
        let rows = self
            .repo
            .find_events(user_id, &filter, limit as i64, offset)
            .await?;

        Ok(EventQueryResponse {
            total,
            page,
            limit,
            items: rows.into_iter().map(EventItem::from).collect(),
        })
    }
}

// ─── 单元测试（EQ01~EQ05, EQ07, EQ08）──────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::event::query_repo::FakeEventQueryRepository;
    use chrono::Duration;

    // ── 辅助函数 ──────────────────────────────────────────────────────────

    fn make_row(name: &str, secs_ago: i64) -> crate::modules::event::query_dto::EventRow {
        crate::modules::event::query_dto::EventRow {
            id: Uuid::new_v4(),
            event_name: name.to_string(),
            server_ts: Utc::now() - Duration::seconds(secs_ago),
            client_ts: None,
            session_id: None,
            device_id: "test-device".to_string(),
            properties: serde_json::json!({}),
            app_version: Some("1.0.0".to_string()),
            os_version: None,
            locale: None,
            network_type: None,
        }
    }

    fn default_params() -> EventQueryParams {
        let now = Utc::now();
        let from = now - Duration::hours(1);
        EventQueryParams {
            event_name: None,
            from: Some(from.to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        }
    }

    // ── EQ01: 正常查询返回按 server_ts DESC 排序 ──────────────────────────

    /// EQ01: 正常查询返回结果，server_ts 倒序（最新在前）
    #[tokio::test]
    async fn eq01_normal_query_returns_sorted_by_server_ts_desc() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        // 插入 3 条事件，时间错落
        repo.push(make_row("event_old", 3600));      // 1h 前
        repo.push(make_row("event_mid", 1800));      // 30m 前
        repo.push(make_row("event_new", 60));        // 1m 前

        let service = EventQueryService::new(repo);
        let now = Utc::now();
        let params = EventQueryParams {
            event_name: None,
            from: Some((now - Duration::hours(2)).to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        };

        let resp = service
            .query_events(Uuid::new_v4(), params, false)
            .await
            .unwrap();

        assert_eq!(resp.total, 3, "EQ01: total 应为 3");
        assert_eq!(resp.items.len(), 3, "EQ01: items 长度应为 3");
        // 验证按 server_ts 倒序排列
        let ts0 = resp.items[0].server_ts.parse::<chrono::DateTime<Utc>>().unwrap();
        let ts1 = resp.items[1].server_ts.parse::<chrono::DateTime<Utc>>().unwrap();
        let ts2 = resp.items[2].server_ts.parse::<chrono::DateTime<Utc>>().unwrap();
        assert!(
            ts0 >= ts1 && ts1 >= ts2,
            "EQ01: items 应按 server_ts DESC 排序"
        );
    }

    // ── EQ02: 时间窗 31 天返回 ValidationError ────────────────────────────

    /// EQ02: from=now-31d, to=now → ValidationError（时间窗超 30 天）
    #[tokio::test]
    async fn eq02_time_window_31_days_returns_validation_error() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let service = EventQueryService::new(repo);

        let now = Utc::now();
        let params = EventQueryParams {
            event_name: None,
            from: Some((now - Duration::days(31)).to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        };

        let result = service
            .query_events(Uuid::new_v4(), params, false)
            .await;

        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "EQ02: 时间窗 31 天应返回 ValidationError，got: {:?}",
            result
        );
    }

    /// EQ02b: 刚好 30 天不报错
    #[tokio::test]
    async fn eq02b_time_window_exactly_30_days_is_ok() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let service = EventQueryService::new(repo);

        let now = Utc::now();
        let params = EventQueryParams {
            event_name: None,
            from: Some((now - Duration::days(30)).to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        };

        let result = service
            .query_events(Uuid::new_v4(), params, false)
            .await;

        assert!(result.is_ok(), "EQ02b: 刚好 30 天不应报错");
    }

    // ── EQ03: event_name 多值过滤生效 ────────────────────────────────────

    /// EQ03: event_name=gift_send,room_join 只返回这两种事件
    #[tokio::test]
    async fn eq03_event_name_multi_value_filter() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let now = Utc::now();
        let from = now - Duration::hours(1);

        // 插入 3 种事件
        repo.push(make_row("gift_send", 60));
        repo.push(make_row("room_join", 120));
        repo.push(make_row("user_login", 180));

        let service = EventQueryService::new(repo);
        let params = EventQueryParams {
            event_name: Some("gift_send,room_join".to_string()),
            from: Some(from.to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        };

        let resp = service
            .query_events(Uuid::new_v4(), params, false)
            .await
            .unwrap();

        assert_eq!(resp.total, 2, "EQ03: total 应为 2（过滤 user_login）");
        assert_eq!(resp.items.len(), 2, "EQ03: items 长度应为 2");
        for item in &resp.items {
            assert!(
                item.event_name == "gift_send" || item.event_name == "room_join",
                "EQ03: 只应返回 gift_send 或 room_join，但得到: {}",
                item.event_name
            );
        }
    }

    /// EQ03b: event_name 带空格的逗号分隔（" gift_send , room_join "）
    #[tokio::test]
    async fn eq03b_event_name_with_spaces_trimmed() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let now = Utc::now();
        let from = now - Duration::hours(1);
        repo.push(make_row("gift_send", 60));

        let service = EventQueryService::new(repo);
        let params = EventQueryParams {
            event_name: Some(" gift_send , user_login ".to_string()),
            from: Some(from.to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        };

        let resp = service
            .query_events(Uuid::new_v4(), params, false)
            .await
            .unwrap();

        assert_eq!(resp.total, 1, "EQ03b: 空格被 trim 后应匹配 gift_send");
    }

    // ── EQ04: limit=101 返回 ValidationError ─────────────────────────────

    /// EQ04: limit=101 → ValidationError（max limit=100）
    #[tokio::test]
    async fn eq04_limit_over_100_returns_validation_error() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let service = EventQueryService::new(repo);

        let now = Utc::now();
        let params = EventQueryParams {
            event_name: None,
            from: Some((now - Duration::hours(1)).to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(101),
        };

        let result = service
            .query_events(Uuid::new_v4(), params, false)
            .await;

        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "EQ04: limit=101 应返回 ValidationError，got: {:?}",
            result
        );
    }

    /// EQ04b: limit=100 不报错（边界值）
    #[tokio::test]
    async fn eq04b_limit_exactly_100_is_ok() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let service = EventQueryService::new(repo);
        let now = Utc::now();
        let params = EventQueryParams {
            event_name: None,
            from: Some((now - Duration::hours(1)).to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(100),
        };

        let result = service.query_events(Uuid::new_v4(), params, false).await;
        assert!(result.is_ok(), "EQ04b: limit=100 不应报错");
    }

    // ── EQ05: cs 角色查 admin_* 被过滤 ────────────────────────────────────

    /// EQ05: filter_admin_events=true → admin_* 前缀事件被过滤（cs 角色场景）
    #[tokio::test]
    async fn eq05_cs_role_admin_events_filtered() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let now = Utc::now();
        let from = now - Duration::hours(1);

        // 预设 3 条事件（含 2 条 admin_* 和 1 条普通事件）
        repo.push(make_row("admin_login", 60));
        repo.push(make_row("admin_ban_user", 120));
        repo.push(make_row("gift_send", 180));

        let service = EventQueryService::new(repo);
        let params = EventQueryParams {
            event_name: None,
            from: Some(from.to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        };

        // filter_admin_events=true 模拟 cs 角色
        let resp = service
            .query_events(Uuid::new_v4(), params, true)
            .await
            .unwrap();

        assert_eq!(resp.total, 1, "EQ05: total 应为 1（admin_* 事件被过滤）");
        assert_eq!(resp.items.len(), 1, "EQ05: items 长度应为 1");
        assert_eq!(
            resp.items[0].event_name, "gift_send",
            "EQ05: 只应返回 gift_send"
        );
        for item in &resp.items {
            assert!(
                !item.event_name.starts_with("admin_"),
                "EQ05: admin_ 前缀事件不应出现在结果中"
            );
        }
    }

    /// EQ05b: cs 角色请求 event_name=admin_login → 结果为空
    #[tokio::test]
    async fn eq05b_cs_role_requests_admin_event_name_returns_empty() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let now = Utc::now();
        let from = now - Duration::hours(1);

        repo.push(make_row("admin_login", 60));
        repo.push(make_row("gift_send", 120));

        let service = EventQueryService::new(repo);
        let params = EventQueryParams {
            event_name: Some("admin_login".to_string()),
            from: Some(from.to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        };

        let resp = service
            .query_events(Uuid::new_v4(), params, true)
            .await
            .unwrap();

        assert_eq!(
            resp.total, 0,
            "EQ05b: cs 查 admin_login 应返回空（被过滤）"
        );
        assert!(resp.items.is_empty());
    }

    // ── EQ07: 分页功能验证 ────────────────────────────────────────────────

    /// EQ07-PAGINATION: 分页参数生效（page=2, limit=2）
    #[tokio::test]
    async fn eq07_pagination_works_correctly() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let now = Utc::now();
        let from = now - Duration::hours(1);

        // 插入 5 条事件
        for i in 0..5 {
            repo.push(make_row("evt", (i + 1) * 30));
        }

        let service = EventQueryService::new(repo);

        // page=1, limit=2
        let p1 = service
            .query_events(
                Uuid::new_v4(),
                EventQueryParams {
                    event_name: None,
                    from: Some(from.to_rfc3339()),
                    to: Some(now.to_rfc3339()),
                    page: Some(1),
                    limit: Some(2),
                },
                false,
            )
            .await
            .unwrap();

        // page=2, limit=2
        let p2 = service
            .query_events(
                Uuid::new_v4(),
                EventQueryParams {
                    event_name: None,
                    from: Some(from.to_rfc3339()),
                    to: Some(now.to_rfc3339()),
                    page: Some(2),
                    limit: Some(2),
                },
                false,
            )
            .await
            .unwrap();

        assert_eq!(p1.total, 5, "EQ07: total 应为 5");
        assert_eq!(p1.items.len(), 2, "EQ07: page1 应有 2 条");
        assert_eq!(p2.items.len(), 2, "EQ07: page2 应有 2 条");
        assert_ne!(
            p1.items[0].id, p2.items[0].id,
            "EQ07: 两页的第一条不应相同"
        );
    }

    // ── EQ08: 响应时间 <300ms（本地填充 10K events 测试）────────────────

    /// EQ08: 填充 10K events，service 处理时间 <300ms
    #[tokio::test]
    async fn eq08_performance_10k_events_under_300ms() {
        let repo = Arc::new(FakeEventQueryRepository::default());

        // 使用固定基准时间创建事件，避免时间漂移导致边界问题
        let base_now = Utc::now();

        // 填充 10,000 条事件（均在过去 1~86400s 内，均严格 < base_now）
        let events: Vec<_> = (1..=10_000)
            .map(|i| {
                let secs_ago = (i % 86399) + 1; // 1 ~ 86399 秒前，严格在窗口内
                let ts = base_now - Duration::seconds(secs_ago as i64);
                crate::modules::event::query_dto::EventRow {
                    id: Uuid::new_v4(),
                    event_name: "gift_send".to_string(),
                    server_ts: ts,
                    client_ts: None,
                    session_id: None,
                    device_id: "device-test".to_string(),
                    properties: serde_json::json!({}),
                    app_version: None,
                    os_version: None,
                    locale: None,
                    network_type: None,
                }
            })
            .collect();
        repo.push_many(events);

        let service = EventQueryService::new(repo);
        let start = std::time::Instant::now();

        let params = EventQueryParams {
            event_name: None,
            from: Some((base_now - Duration::hours(24)).to_rfc3339()),
            to: Some(base_now.to_rfc3339()),
            page: Some(1),
            limit: Some(100),
        };

        let resp = service
            .query_events(Uuid::new_v4(), params, false)
            .await
            .unwrap();

        let elapsed = start.elapsed();
        assert_eq!(resp.limit, 100, "EQ08: limit 应为 100");
        assert_eq!(resp.total, 10_000, "EQ08: total 应为 10000");
        assert!(
            elapsed.as_millis() < 300,
            "EQ08: 响应时间应 <300ms，实际 {}ms",
            elapsed.as_millis()
        );
    }

    // ── 边界用例 ──────────────────────────────────────────────────────────

    /// 无结果时返回 total=0, items=[]
    #[tokio::test]
    async fn empty_result_when_no_matching_events() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let service = EventQueryService::new(repo);

        let params = default_params();
        let resp = service
            .query_events(Uuid::new_v4(), params, false)
            .await
            .unwrap();

        assert_eq!(resp.total, 0);
        assert!(resp.items.is_empty());
    }

    /// 默认参数（无 from/to）使用 24h 默认窗口，不报错
    #[tokio::test]
    async fn default_time_range_24h_does_not_error() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let service = EventQueryService::new(repo);

        let params = EventQueryParams {
            event_name: None,
            from: None,
            to: None,
            page: None,
            limit: None,
        };

        let result = service.query_events(Uuid::new_v4(), params, false).await;
        assert!(result.is_ok(), "默认参数不应报错，got: {:?}", result);
    }

    /// from 格式非法 → ValidationError
    #[tokio::test]
    async fn invalid_from_format_returns_validation_error() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let service = EventQueryService::new(repo);

        let params = EventQueryParams {
            event_name: None,
            from: Some("not-a-date".to_string()),
            to: None,
            page: None,
            limit: None,
        };

        let result = service.query_events(Uuid::new_v4(), params, false).await;
        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "非法 from 应返回 ValidationError"
        );
    }

    /// super_admin（filter_admin_events=false）可以看到 admin_* 事件
    #[tokio::test]
    async fn super_admin_can_see_admin_events() {
        let repo = Arc::new(FakeEventQueryRepository::default());
        let now = Utc::now();
        let from = now - Duration::hours(1);

        repo.push(make_row("admin_login", 60));
        repo.push(make_row("admin_ban_user", 120));
        repo.push(make_row("gift_send", 180));

        let service = EventQueryService::new(repo);
        let params = EventQueryParams {
            event_name: None,
            from: Some(from.to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
        };

        // filter_admin_events=false 模拟 super_admin 角色
        let resp = service
            .query_events(Uuid::new_v4(), params, false)
            .await
            .unwrap();

        assert_eq!(resp.total, 3, "super_admin 应能看到全部 3 条事件");
    }
}
