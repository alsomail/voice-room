use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::error::AppError;

use super::query_dto::{EventFilter, EventRow};

// ─── Trait ────────────────────────────────────────────────────────────────────

/// events 表查询抽象（隔离真实 DB 与测试 Fake）。
#[async_trait]
pub trait EventQueryRepository: Send + Sync {
    /// 按过滤条件统计事件总数（用于分页 total）。
    async fn count_events(&self, user_id: Uuid, filter: &EventFilter) -> Result<i64, AppError>;

    /// 按过滤条件分页查询事件列表，结果按 server_ts DESC 排序。
    async fn find_events(
        &self,
        user_id: Uuid,
        filter: &EventFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EventRow>, AppError>;
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

/// 基于 SQLx + PostgreSQL 的 EventQueryRepository 生产实现。
///
/// SQL 利用 `server_ts BETWEEN from AND to` 命中分区键，实现分区剪枝。
pub struct PgEventQueryRepository {
    pool: PgPool,
}

impl PgEventQueryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventQueryRepository for PgEventQueryRepository {
    async fn count_events(&self, user_id: Uuid, filter: &EventFilter) -> Result<i64, AppError> {
        let names = filter.event_names.as_deref();
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM events \
             WHERE user_id = $1 \
               AND server_ts BETWEEN $2 AND $3 \
               AND ($4::text[] IS NULL OR event_name = ANY($4)) \
               AND (NOT $5 OR event_name NOT LIKE 'admin_%')",
        )
        .bind(user_id)
        .bind(filter.from)
        .bind(filter.to)
        .bind(names)
        .bind(filter.exclude_admin_prefix)
        .fetch_one(&self.pool)
        .await?;
        Ok(count.0)
    }

    async fn find_events(
        &self,
        user_id: Uuid,
        filter: &EventFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EventRow>, AppError> {
        let names = filter.event_names.as_deref();
        let rows = sqlx::query_as::<_, EventRow>(
            "SELECT id, event_name, server_ts, client_ts, session_id, device_id, \
                    properties, app_version, os_version, locale, network_type \
             FROM events \
             WHERE user_id = $1 \
               AND server_ts BETWEEN $2 AND $3 \
               AND ($4::text[] IS NULL OR event_name = ANY($4)) \
               AND (NOT $5 OR event_name NOT LIKE 'admin_%') \
             ORDER BY server_ts DESC \
             LIMIT $6 OFFSET $7",
        )
        .bind(user_id)
        .bind(filter.from)
        .bind(filter.to)
        .bind(names)
        .bind(filter.exclude_admin_prefix)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

// ─── Fake 实现（内存，用于单元/集成测试）──────────────────────────────────────

/// 测试专用：内存 EventQueryRepository。
pub struct FakeEventQueryRepository {
    events: Arc<Mutex<Vec<EventRow>>>,
}

impl Default for FakeEventQueryRepository {
    fn default() -> Self {
        Self {
            events: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl FakeEventQueryRepository {
    /// 插入一条事件（供测试预填充数据）。
    pub fn push(&self, event: EventRow) {
        self.events.lock().unwrap().push(event);
    }

    /// 批量插入事件（供 EQ08 性能测试填充大量数据）。
    pub fn push_many(&self, events: Vec<EventRow>) {
        self.events.lock().unwrap().extend(events);
    }

    /// 读取所有事件（供测试断言使用）。
    pub fn all(&self) -> Vec<EventRow> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl EventQueryRepository for FakeEventQueryRepository {
    async fn count_events(&self, user_id: Uuid, filter: &EventFilter) -> Result<i64, AppError> {
        let events = self.events.lock().unwrap();
        let count = events
            .iter()
            .filter(|e| {
                let user_match = true; // Fake 不校验 user_id，由 service 层处理
                let _ = user_id; // suppress warning
                let in_time = e.server_ts >= filter.from && e.server_ts <= filter.to;
                let name_match = match &filter.event_names {
                    None => true,
                    Some(names) => {
                        if names.is_empty() {
                            false
                        } else {
                            names.contains(&e.event_name)
                        }
                    }
                };
                let admin_ok = !filter.exclude_admin_prefix
                    || !e.event_name.starts_with("admin_");
                user_match && in_time && name_match && admin_ok
            })
            .count() as i64;
        Ok(count)
    }

    async fn find_events(
        &self,
        user_id: Uuid,
        filter: &EventFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<EventRow>, AppError> {
        let events = self.events.lock().unwrap();
        let _ = user_id;

        let mut filtered: Vec<EventRow> = events
            .iter()
            .filter(|e| {
                let in_time = e.server_ts >= filter.from && e.server_ts <= filter.to;
                let name_match = match &filter.event_names {
                    None => true,
                    Some(names) => {
                        if names.is_empty() {
                            false
                        } else {
                            names.contains(&e.event_name)
                        }
                    }
                };
                let admin_ok = !filter.exclude_admin_prefix
                    || !e.event_name.starts_with("admin_");
                in_time && name_match && admin_ok
            })
            .cloned()
            .collect();

        // 按 server_ts DESC 排序（模拟 DB 行为）
        filtered.sort_by(|a, b| b.server_ts.cmp(&a.server_ts));

        // 分页
        let start = offset as usize;
        if start >= filtered.len() {
            return Ok(vec![]);
        }
        let end = (offset + limit).min(filtered.len() as i64) as usize;
        Ok(filtered[start..end].to_vec())
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn make_event(name: &str, offset_secs: i64) -> EventRow {
        let ts = Utc::now() - Duration::seconds(offset_secs);
        EventRow {
            id: Uuid::new_v4(),
            event_name: name.to_string(),
            server_ts: ts,
            client_ts: None,
            session_id: None,
            device_id: "test-device".to_string(),
            properties: serde_json::json!({}),
            app_version: None,
            os_version: None,
            locale: None,
            network_type: None,
        }
    }

    fn make_filter(hours_back: i64) -> EventFilter {
        EventFilter {
            from: Utc::now() - Duration::hours(hours_back),
            to: Utc::now(),
            event_names: None,
            exclude_admin_prefix: false,
        }
    }

    /// Fake::count_events 返回在时间窗内的事件数
    #[tokio::test]
    async fn fake_count_events_within_time_window() {
        let repo = FakeEventQueryRepository::default();
        repo.push(make_event("gift_send", 10));
        repo.push(make_event("room_join", 20));

        let filter = make_filter(1); // 1 hour window
        let uid = Uuid::new_v4();
        let count = repo.count_events(uid, &filter).await.unwrap();
        assert_eq!(count, 2);
    }

    /// Fake::count_events 对 admin_ 前缀过滤生效
    #[tokio::test]
    async fn fake_count_events_excludes_admin_prefix() {
        let repo = FakeEventQueryRepository::default();
        repo.push(make_event("admin_login", 5));
        repo.push(make_event("gift_send", 10));

        let filter = EventFilter {
            from: Utc::now() - Duration::hours(1),
            to: Utc::now(),
            event_names: None,
            exclude_admin_prefix: true,
        };
        let uid = Uuid::new_v4();
        let count = repo.count_events(uid, &filter).await.unwrap();
        assert_eq!(count, 1, "admin_login 应被过滤，只剩 gift_send");
    }

    /// Fake::find_events 返回按 server_ts DESC 排序
    #[tokio::test]
    async fn fake_find_events_sorted_desc() {
        let repo = FakeEventQueryRepository::default();
        // 第一条更旧（offset_secs 更大）
        repo.push(make_event("event_old", 30));
        repo.push(make_event("event_new", 5));

        let filter = make_filter(1);
        let uid = Uuid::new_v4();
        let items = repo.find_events(uid, &filter, 10, 0).await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].event_name, "event_new",
            "最新事件应排在第一位"
        );
        assert_eq!(items[1].event_name, "event_old");
    }

    /// Fake::find_events 分页偏移生效
    #[tokio::test]
    async fn fake_find_events_pagination() {
        let repo = FakeEventQueryRepository::default();
        for i in 0..5 {
            repo.push(make_event("evt", i * 5));
        }
        let filter = make_filter(1);
        let uid = Uuid::new_v4();

        let page1 = repo.find_events(uid, &filter, 2, 0).await.unwrap();
        let page2 = repo.find_events(uid, &filter, 2, 2).await.unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 2);
        // page1 和 page2 的事件不相同（按时间排序后分页）
        assert_ne!(page1[0].id, page2[0].id);
    }
}
