use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ─── DTO / Filter ─────────────────────────────────────────────────────────────

/// 创建审计日志的参数结构（传入 AuditRepository::insert）
#[derive(Debug, Clone)]
pub struct CreateAuditLog {
    pub admin_id: Uuid,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    /// 操作者 IP 地址（字符串格式）
    pub ip: Option<String>,
    pub detail: Option<serde_json::Value>,
}

/// 从数据库读取的审计日志行
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdminLogRow {
    pub id: Uuid,
    pub admin_id: Uuid,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    /// 由 INET::text 转换而来
    pub ip_address: Option<String>,
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// 列表查询过滤器（service 层填充后传入 repository.list）
#[derive(Debug, Default)]
pub struct AdminLogFilter {
    pub admin_id: Option<Uuid>,
    pub action: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    /// 0-based 页码（service 层负责从 1-based 转换）
    pub page: i64,
    /// 每页条数（上限 100）
    pub size: i64,
}

// ─── Trait ────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait AuditRepository: Send + Sync {
    async fn insert(&self, log: CreateAuditLog) -> Result<(), sqlx::Error>;
    async fn list(&self, filter: AdminLogFilter) -> Result<(i64, Vec<AdminLogRow>), sqlx::Error>;
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

pub struct PgAuditRepository {
    pool: PgPool,
}

impl PgAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditRepository for PgAuditRepository {
    async fn insert(&self, log: CreateAuditLog) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO admin_logs (admin_id, action, target_type, target_id, detail, ip_address) \
             VALUES ($1, $2, $3, $4, $5, $6::inet)",
        )
        .bind(log.admin_id)
        .bind(log.action)
        .bind(log.target_type)
        .bind(log.target_id)
        .bind(log.detail)
        .bind(log.ip)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list(&self, filter: AdminLogFilter) -> Result<(i64, Vec<AdminLogRow>), sqlx::Error> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM admin_logs \
             WHERE ($1::uuid IS NULL OR admin_id = $1) \
               AND ($2::text IS NULL OR action = $2) \
               AND ($3::timestamptz IS NULL OR created_at >= $3) \
               AND ($4::timestamptz IS NULL OR created_at <= $4)",
        )
        .bind(filter.admin_id)
        .bind(filter.action.as_deref())
        .bind(filter.start_date)
        .bind(filter.end_date)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, AdminLogRow>(
            "SELECT id, admin_id, action, target_type, target_id, \
                    ip_address::text AS ip_address, detail, created_at \
             FROM admin_logs \
             WHERE ($1::uuid IS NULL OR admin_id = $1) \
               AND ($2::text IS NULL OR action = $2) \
               AND ($3::timestamptz IS NULL OR created_at >= $3) \
               AND ($4::timestamptz IS NULL OR created_at <= $4) \
             ORDER BY created_at DESC \
             LIMIT $5 OFFSET $6",
        )
        .bind(filter.admin_id)
        .bind(filter.action.as_deref())
        .bind(filter.start_date)
        .bind(filter.end_date)
        .bind(filter.size)
        .bind(filter.page * filter.size)
        .fetch_all(&self.pool)
        .await?;

        Ok((count.0, rows))
    }
}

// ─── Fake 实现（内存，用于单元/集成测试）──────────────────────────────────────

/// 测试专用：内存 AuditRepository，支持错误注入。
pub struct FakeAuditRepository {
    logs: Arc<Mutex<Vec<AdminLogRow>>>,
    inject_error: Arc<AtomicBool>,
}

impl Default for FakeAuditRepository {
    fn default() -> Self {
        Self {
            logs: Arc::new(Mutex::new(vec![])),
            inject_error: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl FakeAuditRepository {
    /// 设置是否注入错误（true = 下一次操作返回 sqlx::Error::RowNotFound）
    pub fn set_inject_error(&self, v: bool) {
        self.inject_error.store(v, Ordering::SeqCst);
    }

    /// 读取所有已插入的日志（供测试断言使用）
    pub fn get_logs(&self) -> Vec<AdminLogRow> {
        self.logs.lock().unwrap().clone()
    }
}

#[async_trait]
impl AuditRepository for FakeAuditRepository {
    async fn insert(&self, log: CreateAuditLog) -> Result<(), sqlx::Error> {
        if self.inject_error.load(Ordering::SeqCst) {
            return Err(sqlx::Error::RowNotFound);
        }
        let mut logs = self.logs.lock().unwrap();
        logs.push(AdminLogRow {
            id: Uuid::new_v4(),
            admin_id: log.admin_id,
            action: log.action,
            target_type: log.target_type,
            target_id: log.target_id,
            ip_address: log.ip,
            detail: log.detail,
            created_at: Utc::now(),
        });
        Ok(())
    }

    async fn list(&self, filter: AdminLogFilter) -> Result<(i64, Vec<AdminLogRow>), sqlx::Error> {
        if self.inject_error.load(Ordering::SeqCst) {
            return Err(sqlx::Error::RowNotFound);
        }
        let logs = self.logs.lock().unwrap();

        let filtered: Vec<AdminLogRow> = logs
            .iter()
            .filter(|log| {
                if let Some(admin_id) = filter.admin_id {
                    if log.admin_id != admin_id {
                        return false;
                    }
                }
                if let Some(ref action) = filter.action {
                    if &log.action != action {
                        return false;
                    }
                }
                if let Some(start) = filter.start_date {
                    if log.created_at < start {
                        return false;
                    }
                }
                if let Some(end) = filter.end_date {
                    if log.created_at > end {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total = filtered.len() as i64;
        let offset = (filter.page * filter.size) as usize;
        let items: Vec<AdminLogRow> = filtered
            .into_iter()
            .skip(offset)
            .take(filter.size as usize)
            .collect();

        Ok((total, items))
    }
}

// ─── 单元测试（AL-01~05）──────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(action: &str, admin_id: Uuid, target_id: Option<Uuid>) -> CreateAuditLog {
        CreateAuditLog {
            admin_id,
            action: action.to_string(),
            target_type: Some("user".to_string()),
            target_id,
            ip: Some("1.2.3.4".to_string()),
            detail: Some(serde_json::json!({"reason": "test"})),
        }
    }

    /// AL-01: insert 成功，FakeRepo 的 logs 长度 +1，字段与传入一致
    #[tokio::test]
    async fn al01_insert_increments_logs_and_matches_fields() {
        let repo = FakeAuditRepository::default();
        let admin_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let entry = make_entry("ban_user", admin_id, Some(target_id));

        let result = repo.insert(entry).await;
        assert!(result.is_ok(), "AL-01: insert 应返回 Ok(())");

        let logs = repo.get_logs();
        assert_eq!(logs.len(), 1, "AL-01: logs 长度应为 1");
        assert_eq!(logs[0].action, "ban_user", "AL-01: action 应匹配");
        assert_eq!(logs[0].admin_id, admin_id, "AL-01: admin_id 应匹配");
        assert_eq!(logs[0].target_id, Some(target_id), "AL-01: target_id 应匹配");
        assert_eq!(
            logs[0].ip_address,
            Some("1.2.3.4".to_string()),
            "AL-01: ip_address 应匹配"
        );
        assert_eq!(
            logs[0].target_type,
            Some("user".to_string()),
            "AL-01: target_type 应匹配"
        );
    }

    /// AL-02: list 按 admin_id 过滤，仅返回匹配记录，total 正确
    #[tokio::test]
    async fn al02_list_filters_by_admin_id() {
        let repo = FakeAuditRepository::default();
        let admin1 = Uuid::new_v4();
        let admin2 = Uuid::new_v4();

        repo.insert(make_entry("ban_user", admin1, None)).await.unwrap();
        repo.insert(make_entry("close_room", admin2, None))
            .await
            .unwrap();
        repo.insert(make_entry("unban_user", admin1, None))
            .await
            .unwrap();

        let filter = AdminLogFilter {
            admin_id: Some(admin1),
            action: None,
            start_date: None,
            end_date: None,
            page: 0,
            size: 20,
        };

        let (total, items) = repo.list(filter).await.unwrap();
        assert_eq!(total, 2, "AL-02: admin1 应有 2 条日志");
        assert_eq!(items.len(), 2, "AL-02: items 应有 2 条");
        for item in &items {
            assert_eq!(item.admin_id, admin1, "AL-02: 每条日志的 admin_id 应匹配");
        }
    }

    /// AL-03: list 按 action 过滤，仅返回 action 匹配的记录
    #[tokio::test]
    async fn al03_list_filters_by_action() {
        let repo = FakeAuditRepository::default();
        let admin_id = Uuid::new_v4();

        repo.insert(make_entry("ban_user", admin_id, None))
            .await
            .unwrap();
        repo.insert(make_entry("close_room", admin_id, None))
            .await
            .unwrap();
        repo.insert(make_entry("ban_user", admin_id, None))
            .await
            .unwrap();

        let filter = AdminLogFilter {
            admin_id: None,
            action: Some("ban_user".to_string()),
            start_date: None,
            end_date: None,
            page: 0,
            size: 20,
        };

        let (total, items) = repo.list(filter).await.unwrap();
        assert_eq!(total, 2, "AL-03: action=ban_user 应有 2 条");
        assert_eq!(items.len(), 2, "AL-03: items 应有 2 条");
        for item in &items {
            assert_eq!(item.action, "ban_user", "AL-03: action 应为 ban_user");
        }
    }

    /// AL-04: list 分页，第 2 页（page=1, size=2）返回正确偏移的数据
    #[tokio::test]
    async fn al04_list_pagination_second_page() {
        let repo = FakeAuditRepository::default();
        let admin_id = Uuid::new_v4();

        for i in 0..5 {
            repo.insert(CreateAuditLog {
                admin_id,
                action: format!("action_{i}"),
                target_type: None,
                target_id: None,
                ip: None,
                detail: None,
            })
            .await
            .unwrap();
        }

        let filter = AdminLogFilter {
            admin_id: None,
            action: None,
            start_date: None,
            end_date: None,
            page: 1, // 0-based，第 2 页
            size: 2,
        };

        let (total, items) = repo.list(filter).await.unwrap();
        assert_eq!(total, 5, "AL-04: total 应为 5");
        assert_eq!(items.len(), 2, "AL-04: 第 2 页应有 2 条");
    }

    /// AL-05: list 按时间范围过滤，start_date 在未来 → 无结果
    #[tokio::test]
    async fn al05_list_filters_by_date_range() {
        let repo = FakeAuditRepository::default();
        let admin_id = Uuid::new_v4();

        repo.insert(make_entry("ban_user", admin_id, None))
            .await
            .unwrap();

        // start_date 在未来 → 无结果
        let future = Utc::now() + chrono::Duration::hours(1);
        let filter = AdminLogFilter {
            admin_id: None,
            action: None,
            start_date: Some(future),
            end_date: None,
            page: 0,
            size: 20,
        };

        let (total, items) = repo.list(filter).await.unwrap();
        assert_eq!(total, 0, "AL-05: 未来时间范围内应无日志");
        assert!(items.is_empty(), "AL-05: items 应为空");

        // end_date 在过去 → 无结果
        let past = Utc::now() - chrono::Duration::hours(1);
        let filter2 = AdminLogFilter {
            admin_id: None,
            action: None,
            start_date: None,
            end_date: Some(past),
            page: 0,
            size: 20,
        };

        let (total2, items2) = repo.list(filter2).await.unwrap();
        assert_eq!(total2, 0, "AL-05: 过去 end_date 应无日志");
        assert!(items2.is_empty(), "AL-05: items2 应为空");
    }
}
