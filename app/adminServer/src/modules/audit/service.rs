use std::sync::Arc;

use uuid::Uuid;

use crate::common::error::AppError;

use super::{
    dto::{AdminLogItem, ListLogsQuery, ListLogsResponse},
    repository::{AdminLogFilter, AuditRepository, CreateAuditLog},
};

// ─── AuditLogger（写入，fire-and-forget）──────────────────────────────────────

/// 审计日志写入器。
///
/// 通过 `log_action` 写入操作记录，失败时仅打印 warn 日志，不影响主业务流程。
pub struct AuditLogger {
    pub repo: Arc<dyn AuditRepository>,
}

impl AuditLogger {
    pub fn new(repo: Arc<dyn AuditRepository>) -> Self {
        Self { repo }
    }

    /// 记录一条审计日志（fire-and-forget：失败仅 warn，不返回 Error）。
    pub async fn log_action(
        &self,
        admin_id: Uuid,
        action: &str,
        target_type: Option<&str>,
        target_id: Option<Uuid>,
        ip: Option<String>,
        detail: Option<serde_json::Value>,
    ) {
        let entry = CreateAuditLog {
            admin_id,
            action: action.to_string(),
            target_type: target_type.map(|s| s.to_string()),
            target_id,
            ip,
            detail,
        };
        if let Err(e) = self.repo.insert(entry).await {
            tracing::warn!(
                error = %e,
                admin_id = %admin_id,
                action = action,
                "audit log write failed"
            );
        }
    }
}

// ─── AuditService（查询）──────────────────────────────────────────────────────

/// 审计日志查询服务。
pub struct AuditService {
    pub repo: Arc<dyn AuditRepository>,
}

impl AuditService {
    pub fn new(repo: Arc<dyn AuditRepository>) -> Self {
        Self { repo }
    }

    /// 分页查询审计日志。
    ///
    /// # 参数校验
    /// - `page`：默认 1，最小 1
    /// - `size`：默认 20，最大 100（超出返回 ValidationError）
    pub async fn list_logs(&self, query: ListLogsQuery) -> Result<ListLogsResponse, AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let size_raw = query.size.unwrap_or(20);

        if size_raw > 100 {
            return Err(AppError::ValidationError(
                "size must be <= 100".to_string(),
            ));
        }
        let size = size_raw.max(1);

        // 解析时间范围（可选）
        let start_date = query
            .start_date
            .as_deref()
            .map(|s| {
                s.parse::<chrono::DateTime<chrono::Utc>>()
                    .map_err(|_| AppError::ValidationError(format!("invalid start_date: '{s}'")))
            })
            .transpose()?;

        let end_date = query
            .end_date
            .as_deref()
            .map(|s| {
                s.parse::<chrono::DateTime<chrono::Utc>>()
                    .map_err(|_| AppError::ValidationError(format!("invalid end_date: '{s}'")))
            })
            .transpose()?;

        let filter = AdminLogFilter {
            admin_id: query.admin_id,
            action: query.action,
            start_date,
            end_date,
            page: page - 1, // 1-based → 0-based
            size,
        };

        let (total, rows) = self.repo.list(filter).await?;

        let items = rows
            .into_iter()
            .map(|r| AdminLogItem {
                id: r.id,
                admin_id: r.admin_id,
                action: r.action,
                target_type: r.target_type,
                target_id: r.target_id,
                ip_address: r.ip_address,
                detail: r.detail,
                created_at: r.created_at,
            })
            .collect();

        Ok(ListLogsResponse {
            total,
            page,
            size,
            items,
        })
    }
}

// ─── 单元测试（SL-01~03）──────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::audit::repository::{CreateAuditLog, FakeAuditRepository};
    use std::sync::Arc;
    use uuid::Uuid;

    /// SL-01: log_action 成功，FakeRepo.logs 长度 +1，字段全部匹配
    #[tokio::test]
    async fn sl01_log_action_inserts_entry_with_correct_fields() {
        let repo = Arc::new(FakeAuditRepository::default());
        let logger = AuditLogger::new(repo.clone());

        let admin_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        logger
            .log_action(
                admin_id,
                "ban_user",
                Some("user"),
                Some(user_id),
                Some("1.2.3.4".to_string()),
                Some(serde_json::json!({"reason": "test"})),
            )
            .await;

        let logs = repo.get_logs();
        assert_eq!(logs.len(), 1, "SL-01: logs 长度应为 1");
        assert_eq!(logs[0].action, "ban_user", "SL-01: action 应为 ban_user");
        assert_eq!(logs[0].admin_id, admin_id, "SL-01: admin_id 应匹配");
        assert_eq!(logs[0].target_id, Some(user_id), "SL-01: target_id 应匹配");
        assert_eq!(
            logs[0].ip_address,
            Some("1.2.3.4".to_string()),
            "SL-01: ip 应匹配"
        );
        assert_eq!(
            logs[0].target_type,
            Some("user".to_string()),
            "SL-01: target_type 应为 user"
        );
    }

    /// SL-02: repo 注入 error 后 log_action 不 panic（fire-and-forget 验证）
    #[tokio::test]
    async fn sl02_log_action_does_not_panic_when_repo_errors() {
        let repo = Arc::new(FakeAuditRepository::default());
        repo.set_inject_error(true);
        let logger = AuditLogger::new(repo.clone());

        let admin_id = Uuid::new_v4();

        // 不应 panic，只打 warn
        logger
            .log_action(admin_id, "ban_user", Some("user"), None, None, None)
            .await;

        // 因为 insert 报错，logs 应为空
        let logs = repo.get_logs();
        assert_eq!(logs.len(), 0, "SL-02: error 注入后 logs 应为空");
    }

    /// SL-03: AuditService::list_logs 正确构建 ListLogsResponse
    #[tokio::test]
    async fn sl03_audit_service_list_logs_returns_correct_response() {
        let repo = Arc::new(FakeAuditRepository::default());
        let admin_id = Uuid::new_v4();

        for i in 0..3 {
            repo.insert(CreateAuditLog {
                admin_id,
                action: format!("action_{i}"),
                target_type: Some("user".to_string()),
                target_id: None,
                ip: None,
                detail: None,
            })
            .await
            .unwrap();
        }

        let service = AuditService::new(repo);
        let query = ListLogsQuery {
            admin_id: None,
            action: None,
            start_date: None,
            end_date: None,
            page: Some(1),
            size: Some(10),
        };

        let resp = service.list_logs(query).await.unwrap();
        assert_eq!(resp.total, 3, "SL-03: total 应为 3");
        assert_eq!(resp.page, 1, "SL-03: page 应为 1");
        assert_eq!(resp.size, 10, "SL-03: size 应为 10");
        assert_eq!(resp.items.len(), 3, "SL-03: items 应有 3 条");
    }

    /// SL-04: size > 100 → ValidationError
    #[tokio::test]
    async fn sl04_list_logs_size_over_100_returns_validation_error() {
        let repo = Arc::new(FakeAuditRepository::default());
        let service = AuditService::new(repo);
        let query = ListLogsQuery {
            admin_id: None,
            action: None,
            start_date: None,
            end_date: None,
            page: Some(1),
            size: Some(101),
        };

        let result = service.list_logs(query).await;
        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "SL-04: size=101 应返回 ValidationError"
        );
    }
}
