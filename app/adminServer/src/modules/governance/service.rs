use std::sync::Arc;

use chrono::{Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    common::error::AppError,
    modules::audit::service::AuditLogger,
};

use super::repo::{GovernanceFilter, GovernanceRepo, KickLogItem, MuteLogItem};

// ─── DTO ─────────────────────────────────────────────────────────────────────

/// HTTP Query 参数，适用于 kicks 和 mutes 两个接口。
#[derive(Debug, Clone, Default)]
pub struct GovernanceQueryParams {
    pub room_id: Option<Uuid>,
    pub target_user_id: Option<Uuid>,
    pub operator_user_id: Option<Uuid>,
    /// RFC3339 字符串；默认 7 天前
    pub from: Option<String>,
    /// RFC3339 字符串；默认当前时间
    pub to: Option<String>,
    /// 仅 mutes 有效：`mic` | `chat`
    pub mute_type: Option<String>,
    /// 页码（1-based），默认 1；=0 → ValidationError
    pub page: Option<i64>,
    /// 每页条数，默认 20；>100 → 截断为 100
    pub limit: Option<i64>,
}

/// kicks 接口响应体 data 字段
#[derive(Debug, Serialize)]
pub struct KicksResponse {
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub items: Vec<KickLogItem>,
}

/// mutes 接口响应体 data 字段
#[derive(Debug, Serialize)]
pub struct MutesResponse {
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub items: Vec<MuteLogItem>,
}

// ─── GovernanceService ────────────────────────────────────────────────────────

/// 治理日志业务层。
///
/// 职责：
/// 1. 解析并验证时间范围（最大 90 天，page ≥ 1）
/// 2. 截断 limit（> 100 → 100）
/// 3. 调用 GovernanceRepo 查询
/// 4. 通过 AuditLogger 记录查询操作
pub struct GovernanceService {
    repo: Arc<dyn GovernanceRepo>,
    audit_logger: Arc<AuditLogger>,
}

impl GovernanceService {
    pub fn new(repo: Arc<dyn GovernanceRepo>, audit_logger: Arc<AuditLogger>) -> Self {
        Self { repo, audit_logger }
    }

    // ── 内部：解析并验证公共参数 ────────────────────────────────────────────

    fn resolve_params(
        &self,
        params: &GovernanceQueryParams,
    ) -> Result<(GovernanceFilter, i64, i64, i64, i64), AppError> {
        let now = Utc::now();

        // 解析 to（默认当前时间）
        let to = if let Some(s) = &params.to {
            s.parse::<chrono::DateTime<Utc>>()
                .map_err(|_| AppError::ValidationError(format!("invalid 'to': '{s}'")))?
        } else {
            now
        };

        // 解析 from（默认 7 天前）
        let from = if let Some(s) = &params.from {
            s.parse::<chrono::DateTime<Utc>>()
                .map_err(|_| AppError::ValidationError(format!("invalid 'from': '{s}'")))?
        } else {
            now - Duration::days(7)
        };

        // 时间窗校验：> 90 天 → 40003
        if to - from > Duration::days(90) {
            return Err(AppError::ValidationError(
                "time window exceeds 90 days".to_string(),
            ));
        }

        // page 校验：= 0 → 40003；默认 1
        let page = params.page.unwrap_or(1);
        if page == 0 {
            return Err(AppError::ValidationError(
                "page must be >= 1".to_string(),
            ));
        }

        // limit 校验：> 100 → 截断为 100；默认 20
        let limit = params.limit.unwrap_or(20).clamp(1, 100);

        let offset = (page - 1) * limit;

        let filter = GovernanceFilter {
            room_id: params.room_id,
            target_user_id: params.target_user_id,
            operator_user_id: params.operator_user_id,
            from: Some(from),
            to: Some(to),
            mute_type: params.mute_type.clone(),
        };

        Ok((filter, page, limit, offset, limit))
    }

    // ── 公共接口 ─────────────────────────────────────────────────────────────

    /// 查询踢人记录。
    pub async fn query_kicks(
        &self,
        params: GovernanceQueryParams,
        admin_id: Uuid,
        ip: Option<String>,
    ) -> Result<KicksResponse, AppError> {
        let (filter, page, limit, offset, _) = self.resolve_params(&params)?;

        let (total, items) = self.repo.find_kicks(&filter, limit, offset).await?;

        // 审计日志（fire-and-forget）
        self.audit_logger
            .log_action(
                admin_id,
                "query_kick_records",
                Some("governance"),
                None,
                ip,
                Some(serde_json::json!({
                    "filters": {
                        "room_id": params.room_id,
                        "target_user_id": params.target_user_id,
                        "operator_user_id": params.operator_user_id,
                        "from": params.from,
                        "to": params.to,
                        "page": page,
                        "limit": limit,
                    }
                })),
            )
            .await;

        Ok(KicksResponse { total, page, limit, items })
    }

    /// 查询禁言记录。
    pub async fn query_mutes(
        &self,
        params: GovernanceQueryParams,
        admin_id: Uuid,
        ip: Option<String>,
    ) -> Result<MutesResponse, AppError> {
        let (filter, page, limit, offset, _) = self.resolve_params(&params)?;

        let (total, items) = self.repo.find_mutes(&filter, limit, offset).await?;

        // 审计日志（fire-and-forget）
        self.audit_logger
            .log_action(
                admin_id,
                "query_mute_records",
                Some("governance"),
                None,
                ip,
                Some(serde_json::json!({
                    "filters": {
                        "room_id": params.room_id,
                        "target_user_id": params.target_user_id,
                        "operator_user_id": params.operator_user_id,
                        "from": params.from,
                        "to": params.to,
                        "mute_type": params.mute_type,
                        "page": page,
                        "limit": limit,
                    }
                })),
            )
            .await;

        Ok(MutesResponse { total, page, limit, items })
    }
}

// ─── 单元测试（G16-02/G16-08 服务层验证）─────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::{
        audit::repository::FakeAuditRepository,
        governance::repo::FakeGovernanceRepo,
    };
    use chrono::Duration;
    use std::sync::Arc;
    use uuid::Uuid;

    fn make_service() -> GovernanceService {
        let repo = Arc::new(FakeGovernanceRepo::default());
        let audit_repo = Arc::new(FakeAuditRepository::default());
        GovernanceService::new(repo, Arc::new(AuditLogger::new(audit_repo)))
    }

    fn default_params() -> GovernanceQueryParams {
        let now = Utc::now();
        GovernanceQueryParams {
            from: Some((now - Duration::days(7)).to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
            ..Default::default()
        }
    }

    /// SV-01: 时间窗 > 90 天 → ValidationError
    #[tokio::test]
    async fn sv01_time_window_over_90_days_validation_error() {
        let service = make_service();
        let now = Utc::now();
        let params = GovernanceQueryParams {
            from: Some((now - Duration::days(91)).to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
            ..Default::default()
        };
        let result = service.query_kicks(params, Uuid::new_v4(), None).await;
        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "SV-01: 超 90 天应 ValidationError"
        );
    }

    /// SV-02: 时间窗刚好 90 天 → 正常
    #[tokio::test]
    async fn sv02_time_window_exactly_90_days_ok() {
        let service = make_service();
        let now = Utc::now();
        let params = GovernanceQueryParams {
            from: Some((now - Duration::days(90)).to_rfc3339()),
            to: Some(now.to_rfc3339()),
            page: Some(1),
            limit: Some(20),
            ..Default::default()
        };
        assert!(service.query_kicks(params, Uuid::new_v4(), None).await.is_ok());
    }

    /// SV-03: page=0 → ValidationError
    #[tokio::test]
    async fn sv03_page_zero_validation_error() {
        let service = make_service();
        let mut params = default_params();
        params.page = Some(0);
        let result = service.query_kicks(params, Uuid::new_v4(), None).await;
        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "SV-03: page=0 应 ValidationError"
        );
    }

    /// SV-04: limit > 100 → 截断为 100，不报错
    #[tokio::test]
    async fn sv04_limit_over_100_is_clamped() {
        let service = make_service();
        let mut params = default_params();
        params.limit = Some(999);
        let resp = service.query_kicks(params, Uuid::new_v4(), None).await.unwrap();
        assert_eq!(resp.limit, 100, "SV-04: limit 应截断为 100");
    }

    /// SV-05: 默认 from/to/page/limit → 正常
    #[tokio::test]
    async fn sv05_default_params_ok() {
        let service = make_service();
        let params = GovernanceQueryParams::default();
        let resp = service.query_kicks(params, Uuid::new_v4(), None).await.unwrap();
        assert_eq!(resp.page, 1);
        assert_eq!(resp.limit, 20);
    }

    /// SV-06: 审计日志写入 action=query_kick_records
    #[tokio::test]
    async fn sv06_audit_log_written_for_kicks() {
        let repo = Arc::new(FakeGovernanceRepo::default());
        let audit_repo = Arc::new(FakeAuditRepository::default());
        let admin_id = Uuid::new_v4();
        let service = GovernanceService::new(
            repo,
            Arc::new(AuditLogger::new(audit_repo.clone())),
        );
        service.query_kicks(default_params(), admin_id, None).await.unwrap();
        let logs = audit_repo.get_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action, "query_kick_records");
        assert_eq!(logs[0].admin_id, admin_id);
    }

    /// SV-07: 审计日志写入 action=query_mute_records
    #[tokio::test]
    async fn sv07_audit_log_written_for_mutes() {
        let repo = Arc::new(FakeGovernanceRepo::default());
        let audit_repo = Arc::new(FakeAuditRepository::default());
        let service = GovernanceService::new(
            repo,
            Arc::new(AuditLogger::new(audit_repo.clone())),
        );
        service.query_mutes(default_params(), Uuid::new_v4(), None).await.unwrap();
        let logs = audit_repo.get_logs();
        assert_eq!(logs[0].action, "query_mute_records");
    }

    /// SV-08: invalid from format → ValidationError
    #[tokio::test]
    async fn sv08_invalid_from_format_validation_error() {
        let service = make_service();
        let params = GovernanceQueryParams {
            from: Some("not-a-date".to_string()),
            ..Default::default()
        };
        let result = service.query_kicks(params, Uuid::new_v4(), None).await;
        assert!(matches!(result, Err(AppError::ValidationError(_))));
    }
}
