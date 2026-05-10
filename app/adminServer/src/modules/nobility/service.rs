// PROTO-BINDING: doc/protocol/nobility_api.md §10.5 Admin REST
//! NobilityService — 贵族 tier CRUD、手动赠送/撤销、用户查询业务层。
//!
//! 包含：
//! - `list_tiers` / `get_tier` / `create_tier` / `update_tier` / `delete_tier` (T-10030)
//! - `grant_noble` / `revoke_noble` (T-10031)
//! - `list_noble_users` / `get_noble_history` (T-10032)

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    common::error::AppError,
    modules::{
        audit::service::AuditLogger,
        event::publisher::{EventPublisher, RawEvent},
    },
};

use super::{
    dto::{
        CreateTierRequest, GrantRequest, ListTiersResponse, ListUsersQuery, ListUsersResponse,
        NobleHistoryItem, NobleStatusFilter, RevokeRequest, TierResponse, UpdateTierRequest,
        UserNobleResponse,
    },
    repository::{CreateTierData, GrantData, NobilityRepo, UpdateTierData, UserNobleFilter},
};

// ─── Validation helpers ───────────────────────────────────────────────────────

/// 校验 privileges JSONB 中的关键百分比字段（0..100）
/// 以及必要结构存在性。
/// 返回 Err(PrivilegesSchemaInvalid) 时携带字段路径。
fn validate_privileges(p: &serde_json::Value) -> Result<(), AppError> {
    // monthly_stipend.percent ∈ [0,100]
    if let Some(pct) = p
        .get("monthly_stipend")
        .and_then(|s| s.get("percent"))
        .and_then(|v| v.as_i64())
    {
        if !(0..=100).contains(&pct) {
            return Err(AppError::PrivilegesSchemaInvalid(
                "$.monthly_stipend.percent must be in [0, 100]".to_string(),
            ));
        }
    }
    // gift_discount.percent ∈ [0,100]
    if let Some(pct) = p
        .get("gift_discount")
        .and_then(|s| s.get("percent"))
        .and_then(|v| v.as_i64())
    {
        if !(0..=100).contains(&pct) {
            return Err(AppError::PrivilegesSchemaInvalid(
                "$.gift_discount.percent must be in [0, 100]".to_string(),
            ));
        }
    }
    Ok(())
}

/// 校验 CreateTierRequest 中的字段约束
fn validate_create_tier(req: &CreateTierRequest) -> Result<(), AppError> {
    if req.monthly_diamonds <= 0 {
        return Err(AppError::ValidationError(
            "monthly_diamonds must be > 0".to_string(),
        ));
    }
    if req.level < 1 || req.level > 6 {
        return Err(AppError::ValidationError(
            "level must be between 1 and 6".to_string(),
        ));
    }
    if req.tier_id.trim().is_empty() {
        return Err(AppError::ValidationError(
            "tier_id must not be empty".to_string(),
        ));
    }
    validate_privileges(&req.privileges)?;
    Ok(())
}

/// 校验 UpdateTierRequest 中的字段约束
fn validate_update_tier(req: &UpdateTierRequest) -> Result<(), AppError> {
    if let Some(d) = req.monthly_diamonds {
        if d <= 0 {
            return Err(AppError::ValidationError(
                "monthly_diamonds must be > 0".to_string(),
            ));
        }
    }
    if let Some(ref p) = req.privileges {
        validate_privileges(p)?;
    }
    Ok(())
}

// ─── NobilityService ──────────────────────────────────────────────────────────

pub struct NobilityService {
    repo: Arc<dyn NobilityRepo>,
    audit_logger: Arc<AuditLogger>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl NobilityService {
    pub fn new(
        repo: Arc<dyn NobilityRepo>,
        audit_logger: Arc<AuditLogger>,
        event_publisher: Arc<dyn EventPublisher>,
    ) -> Self {
        Self {
            repo,
            audit_logger,
            event_publisher,
        }
    }

    // ─── T-10030: Tier CRUD ──────────────────────────────────────────────────

    /// GET /api/v1/admin/nobles/tiers?page=&size=
    pub async fn list_tiers(&self, page: i64, size: i64) -> Result<ListTiersResponse, AppError> {
        let page = page.max(1);
        let size = size.clamp(1, 100);
        let (total, rows) = self.repo.list_tiers(page, size).await?;
        Ok(ListTiersResponse {
            items: rows.into_iter().map(Into::into).collect(),
            total,
            page,
            size,
        })
    }

    /// GET /api/v1/admin/nobles/tiers/:id
    pub async fn get_tier(&self, tier_id: &str) -> Result<TierResponse, AppError> {
        self.repo
            .get_tier(tier_id)
            .await?
            .map(Into::into)
            .ok_or_else(|| AppError::NotFound(format!("tier '{tier_id}'")))
    }

    /// POST /api/v1/admin/nobles/tiers
    pub async fn create_tier(
        &self,
        req: CreateTierRequest,
        admin_id: Uuid,
    ) -> Result<TierResponse, AppError> {
        // Validation
        validate_create_tier(&req)?;

        // Level uniqueness check
        if self.repo.level_exists(req.level, None).await? {
            return Err(AppError::TierLevelConflict(req.level));
        }

        let data = CreateTierData {
            tier_id: req.tier_id,
            name_en: req.name_en,
            name_ar: req.name_ar,
            level: req.level,
            monthly_diamonds: req.monthly_diamonds,
            monthly_usd: req.monthly_usd,
            usd_sku_id: req.usd_sku_id,
            privileges: req.privileges,
            icon_url: req.icon_url,
            frame_url: req.frame_url,
            entrance_animation_url: req.entrance_animation_url,
            bgm_url: req.bgm_url,
            badge_color: req.badge_color,
            bubble_style_id: req.bubble_style_id,
        };

        let row = self.repo.create_tier(data).await?;
        let tier: TierResponse = row.into();

        // Audit log
        self.audit_logger
            .log_action(
                admin_id,
                "tier_create",
                Some("noble_tier"),
                None,
                None,
                Some(serde_json::json!({
                    "after": serde_json::to_value(&tier).unwrap_or_default()
                })),
            )
            .await;

        // Redis cache invalidation (fire-and-forget)
        self.publish_cache_invalidate(&tier.tier_id).await;

        Ok(tier)
    }

    /// PUT /api/v1/admin/nobles/tiers/:id
    pub async fn update_tier(
        &self,
        tier_id: &str,
        req: UpdateTierRequest,
        admin_id: Uuid,
    ) -> Result<TierResponse, AppError> {
        validate_update_tier(&req)?;

        // Must exist first (for before-snapshot)
        let before = self
            .repo
            .get_tier(tier_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("tier '{tier_id}'")))?;

        // Must be active
        if !before.is_active {
            return Err(AppError::TierInactive(tier_id.to_string()));
        }

        let data = UpdateTierData {
            name_en: req.name_en,
            name_ar: req.name_ar,
            monthly_diamonds: req.monthly_diamonds,
            monthly_usd: req.monthly_usd,
            usd_sku_id: req.usd_sku_id,
            privileges: req.privileges,
            icon_url: req.icon_url,
            frame_url: req.frame_url,
            entrance_animation_url: req.entrance_animation_url,
            bgm_url: req.bgm_url,
            badge_color: req.badge_color,
            bubble_style_id: req.bubble_style_id,
        };

        let after = self.repo.update_tier(tier_id, data).await?;
        let after_resp: TierResponse = after.into();

        // Audit log
        self.audit_logger
            .log_action(
                admin_id,
                "tier_update",
                Some("noble_tier"),
                None,
                None,
                Some(serde_json::json!({
                    "before": serde_json::to_value(TierResponse::from(before)).unwrap_or_default(),
                    "after": serde_json::to_value(&after_resp).unwrap_or_default()
                })),
            )
            .await;

        // Redis cache invalidation (fire-and-forget)
        self.publish_cache_invalidate(tier_id).await;

        Ok(after_resp)
    }

    /// DELETE /api/v1/admin/nobles/tiers/:id (软删)
    pub async fn delete_tier(&self, tier_id: &str, admin_id: Uuid) -> Result<(), AppError> {
        let before = self
            .repo
            .get_tier(tier_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("tier '{tier_id}'")))?;

        self.repo.soft_delete_tier(tier_id).await?;

        // Audit log
        self.audit_logger
            .log_action(
                admin_id,
                "tier_delete",
                Some("noble_tier"),
                None,
                None,
                Some(serde_json::json!({
                    "name_en": before.name_en,
                    "level": before.level,
                    "is_active_before": true
                })),
            )
            .await;

        // Redis cache invalidation (fire-and-forget)
        self.publish_cache_invalidate(tier_id).await;

        Ok(())
    }

    /// Redis Pub/Sub: 发布缓存失效事件（admin:events channel）
    async fn publish_cache_invalidate(&self, tier_id: &str) {
        let event = RawEvent {
            event_type: "noble_tiers_invalidate".to_string(),
            payload: serde_json::json!({ "tier_id": tier_id }),
            admin_id: "system".to_string(),
            ts: Utc::now().timestamp_millis(),
        };
        if let Err(e) = self
            .event_publisher
            .publish_raw("admin:events", event)
            .await
        {
            tracing::warn!(error = %e, "noble tier cache invalidation publish failed");
        }
    }

    // ─── T-10031: Grant/Revoke ───────────────────────────────────────────────

    /// POST /api/v1/admin/users/:id/noble/grant
    pub async fn grant_noble(
        &self,
        user_id: Uuid,
        req: GrantRequest,
        admin_id: Uuid,
    ) -> Result<UserNobleResponse, AppError> {
        // Validation
        if req.reason.trim().is_empty() {
            return Err(AppError::ValidationError(
                "reason must not be empty".to_string(),
            ));
        }
        if req.duration_days < 1 || req.duration_days > 365 {
            return Err(AppError::ValidationError(
                "duration_days must be between 1 and 365".to_string(),
            ));
        }

        // Verify tier exists and is active
        let tier = self
            .repo
            .get_tier(&req.tier_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("tier '{}'", req.tier_id)))?;
        if !tier.is_active {
            return Err(AppError::TierInactive(req.tier_id.clone()));
        }

        // Get current noble (for from_tier)
        let existing = self.repo.get_user_noble(user_id).await?;
        let from_tier = existing.as_ref().map(|un| un.tier_id.as_str());

        // Upsert with merged expire_at
        let grant_data = GrantData {
            user_id,
            tier_id: req.tier_id.clone(),
            duration_days: req.duration_days,
        };
        let updated = self.repo.upsert_user_noble_grant(grant_data).await?;

        // noble_history INSERT
        self.repo
            .insert_noble_history(
                user_id,
                "admin_grant",
                from_tier,
                Some(&req.tier_id),
                &format!("admin:{admin_id}"),
                serde_json::json!({
                    "duration_days": req.duration_days,
                    "reason": req.reason
                }),
            )
            .await?;

        // Audit log
        self.audit_logger
            .log_action(
                admin_id,
                "noble_grant",
                Some("user"),
                Some(user_id),
                None,
                Some(serde_json::json!({
                    "tier_id": req.tier_id,
                    "duration_days": req.duration_days,
                    "reason": req.reason,
                    "expire_at": updated.expire_at
                })),
            )
            .await;

        // Redis Pub/Sub (fire-and-forget)
        let event = RawEvent {
            event_type: "noble_grant".to_string(),
            payload: serde_json::json!({
                "user_id": user_id,
                "from_tier_id": from_tier,
                "to_tier_id": req.tier_id,
                "expire_at": updated.expire_at,
                "reason": "admin_grant"
            }),
            admin_id: admin_id.to_string(),
            ts: Utc::now().timestamp_millis(),
        };
        if let Err(e) = self
            .event_publisher
            .publish_raw("admin:events", event)
            .await
        {
            tracing::warn!(error = %e, "noble_grant publish failed");
        }

        Ok(updated.into())
    }

    /// POST /api/v1/admin/users/:id/noble/revoke
    pub async fn revoke_noble(
        &self,
        user_id: Uuid,
        req: RevokeRequest,
        admin_id: Uuid,
    ) -> Result<(), AppError> {
        // Validation
        if req.reason.trim().is_empty() {
            return Err(AppError::ValidationError(
                "reason must not be empty".to_string(),
            ));
        }

        // Revoke (DELETE from user_nobles)
        let revoked = self.repo.revoke_user_noble(user_id).await?;

        // noble_history INSERT
        self.repo
            .insert_noble_history(
                user_id,
                "admin_revoke",
                Some(&revoked.tier_id),
                None,
                &format!("admin:{admin_id}"),
                serde_json::json!({ "reason": req.reason }),
            )
            .await?;

        // Audit log
        self.audit_logger
            .log_action(
                admin_id,
                "noble_revoke",
                Some("user"),
                Some(user_id),
                None,
                Some(serde_json::json!({
                    "revoked_tier": revoked.tier_id,
                    "reason": req.reason
                })),
            )
            .await;

        // Redis Pub/Sub (fire-and-forget)
        let event = RawEvent {
            event_type: "noble_revoke".to_string(),
            payload: serde_json::json!({
                "user_id": user_id,
                "from_tier_id": revoked.tier_id,
                "to_tier_id": null,
                "expire_at": null,
                "reason": "admin_revoke"
            }),
            admin_id: admin_id.to_string(),
            ts: Utc::now().timestamp_millis(),
        };
        if let Err(e) = self
            .event_publisher
            .publish_raw("admin:events", event)
            .await
        {
            tracing::warn!(error = %e, "noble_revoke publish failed");
        }

        Ok(())
    }

    // ─── T-10032: User query ─────────────────────────────────────────────────

    /// GET /api/v1/admin/nobles/users
    pub async fn list_noble_users(
        &self,
        query: ListUsersQuery,
    ) -> Result<ListUsersResponse, AppError> {
        // Validate pagination
        if query.page < 1 {
            return Err(AppError::ValidationError(
                "page must be >= 1".to_string(),
            ));
        }
        let size = query.size;
        if size > 100 {
            return Err(AppError::ValidationError(
                "size must be <= 100".to_string(),
            ));
        }
        let size = size.clamp(1, 100);

        // Parse expire_before
        let expire_before = if let Some(ref s) = query.expire_before {
            let dt = super::dto::parse_expire_before(s)
                .map_err(AppError::ValidationError)?;
            Some(dt)
        } else {
            None
        };

        let active_only = query.status.map(|s| matches!(s, NobleStatusFilter::Active));

        let filter = UserNobleFilter {
            tier_id: query.tier_id,
            active_only,
            expire_before,
        };

        let offset = (query.page - 1) * size;
        let (items, total) = tokio::try_join!(
            self.repo.list_noble_users(&filter, size, offset),
            self.repo.count_noble_users(&filter)
        )?;

        Ok(ListUsersResponse {
            items,
            total,
            page: query.page,
            size,
        })
    }

    /// GET /api/v1/admin/nobles/users/:user_id/history
    pub async fn get_noble_history(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<NobleHistoryItem>, AppError> {
        self.repo.get_noble_history(user_id).await
    }
}

// ─── Unit Tests (TDD RED → GREEN) ────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::{
        audit::{repository::FakeAuditRepository, service::AuditLogger},
        event::publisher::NoopEventPublisher,
        nobility::repository::FakeNobilityRepo,
    };
    use chrono::Duration;
    use std::sync::Arc;
    use uuid::Uuid;

    fn make_service() -> (NobilityService, Arc<FakeAuditRepository>, Arc<NoopEventPublisher>, Arc<FakeNobilityRepo>) {
        let repo = Arc::new(FakeNobilityRepo::default());
        let audit_repo = Arc::new(FakeAuditRepository::default());
        let publisher = Arc::new(NoopEventPublisher::default());
        let svc = NobilityService::new(
            repo.clone(),
            Arc::new(AuditLogger::new(audit_repo.clone())),
            publisher.clone(),
        );
        (svc, audit_repo, publisher, repo)
    }

    fn base_create_req() -> CreateTierRequest {
        CreateTierRequest {
            tier_id: "duke".to_string(),
            name_en: "Duke".to_string(),
            name_ar: "دوق".to_string(),
            level: 5,
            monthly_diamonds: 300_000,
            monthly_usd: "999.99".to_string(),
            usd_sku_id: None,
            privileges: serde_json::json!({
                "monthly_stipend": { "percent": 15 },
                "gift_discount": { "percent": 10 }
            }),
            icon_url: "https://cdn.example.com/duke_icon.svg".to_string(),
            frame_url: "https://cdn.example.com/duke_frame.png".to_string(),
            entrance_animation_url: Some("https://cdn.example.com/duke_entry.json".to_string()),
            bgm_url: Some("https://cdn.example.com/duke_bgm.mp3".to_string()),
            badge_color: "#06B6D4".to_string(),
            bubble_style_id: "duke".to_string(),
        }
    }

    // ── TC01: POST 创建成功 → admin_logs +1 ─────────────────────────────────────
    #[tokio::test]
    async fn tc01_create_tier_success_admin_logs() {
        let (svc, audit_repo, publisher, _repo) = make_service();
        let admin_id = Uuid::new_v4();

        let resp = svc.create_tier(base_create_req(), admin_id).await;
        assert!(resp.is_ok(), "TC01: create_tier should succeed");
        let tier = resp.unwrap();
        assert_eq!(tier.tier_id, "duke");
        assert!(tier.is_active);

        // admin_logs +1 with action=tier_create
        let logs = audit_repo.get_logs();
        assert_eq!(logs.len(), 1, "TC01: admin_logs should have 1 record");
        assert_eq!(logs[0].action, "tier_create");
        assert_eq!(logs[0].admin_id, admin_id);

        // Redis publish called once (cache invalidation)
        let raw_calls = publisher.raw_calls.lock().unwrap();
        assert_eq!(raw_calls.len(), 1, "TC01: Redis publish should be called once");
        assert_eq!(raw_calls[0].1.event_type, "noble_tiers_invalidate");
    }

    // ── TC02: POST privileges JSON Schema 不通过 → PrivilegesSchemaInvalid ──────
    #[tokio::test]
    async fn tc02_create_tier_privileges_schema_invalid() {
        let (svc, _, _, _) = make_service();
        let mut req = base_create_req();
        req.privileges = serde_json::json!({
            "monthly_stipend": { "percent": 150 }  // Invalid: > 100
        });

        let err = svc.create_tier(req, Uuid::new_v4()).await.unwrap_err();
        assert!(
            matches!(err, AppError::PrivilegesSchemaInvalid(_)),
            "TC02: should return PrivilegesSchemaInvalid, got {err:?}"
        );
    }

    // ── TC03: POST monthly_diamonds=0 → ValidationError ─────────────────────────
    #[tokio::test]
    async fn tc03_create_tier_monthly_diamonds_zero() {
        let (svc, _, _, _) = make_service();
        let mut req = base_create_req();
        req.monthly_diamonds = 0;

        let err = svc.create_tier(req, Uuid::new_v4()).await.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "TC03: monthly_diamonds=0 should be ValidationError"
        );
    }

    // ── TC04: POST monthly_diamonds=-1 → ValidationError ─────────────────────────
    #[tokio::test]
    async fn tc04_create_tier_monthly_diamonds_negative() {
        let (svc, _, _, _) = make_service();
        let mut req = base_create_req();
        req.monthly_diamonds = -1;

        let err = svc.create_tier(req, Uuid::new_v4()).await.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(_)),
            "TC04: monthly_diamonds=-1 should be ValidationError"
        );
    }

    // ── TC05: POST privileges.monthly_stipend.percent=101 → PrivilegesSchemaInvalid
    #[tokio::test]
    async fn tc05_create_tier_stipend_percent_over_100() {
        let (svc, _, _, _) = make_service();
        let mut req = base_create_req();
        req.privileges = serde_json::json!({
            "monthly_stipend": { "percent": 101 }
        });

        let err = svc.create_tier(req, Uuid::new_v4()).await.unwrap_err();
        assert!(
            matches!(err, AppError::PrivilegesSchemaInvalid(_)),
            "TC05: percent=101 should be PrivilegesSchemaInvalid"
        );
    }

    // ── TC06: POST level 重复 → TierLevelConflict ─────────────────────────────────
    #[tokio::test]
    async fn tc06_create_tier_level_conflict() {
        let (svc, _, _, _) = make_service();
        // Create first
        svc.create_tier(base_create_req(), Uuid::new_v4())
            .await
            .unwrap();

        // Create second with same level
        let mut req2 = base_create_req();
        req2.tier_id = "duke2".to_string();
        // Same level=5

        let err = svc.create_tier(req2, Uuid::new_v4()).await.unwrap_err();
        assert!(
            matches!(err, AppError::TierLevelConflict(5)),
            "TC06: level conflict should return TierLevelConflict(5)"
        );
    }

    // ── TC07: PUT 修改 monthly_diamonds → admin_logs has tier_update ──────────────
    #[tokio::test]
    async fn tc07_update_tier_admin_logs_recorded() {
        let (svc, audit_repo, publisher, _) = make_service();
        let admin_id = Uuid::new_v4();

        svc.create_tier(base_create_req(), admin_id).await.unwrap();
        audit_repo.get_logs(); // drain

        let update = UpdateTierRequest {
            name_en: None,
            name_ar: None,
            monthly_diamonds: Some(400_000),
            monthly_usd: None,
            usd_sku_id: None,
            privileges: None,
            icon_url: None,
            frame_url: None,
            entrance_animation_url: None,
            bgm_url: None,
            badge_color: None,
            bubble_style_id: None,
        };
        let resp = svc.update_tier("duke", update, admin_id).await;
        assert!(resp.is_ok(), "TC07: update_tier should succeed");
        assert_eq!(resp.unwrap().monthly_diamonds, 400_000);

        let logs = audit_repo.get_logs();
        // 2 logs: create + update
        assert!(
            logs.iter().any(|l| l.action == "tier_update"),
            "TC07: admin_logs must have tier_update record"
        );

        // Redis publish called again
        let raw_calls = publisher.raw_calls.lock().unwrap();
        assert!(
            raw_calls.len() >= 2,
            "TC07: Redis publish called for create + update"
        );
    }

    // ── TC08: DELETE 软删 → is_active=false，user_nobles 不影响 ─────────────────
    #[tokio::test]
    async fn tc08_soft_delete_tier() {
        let (svc, audit_repo, _, _repo) = make_service();
        let admin_id = Uuid::new_v4();

        svc.create_tier(base_create_req(), admin_id).await.unwrap();
        svc.delete_tier("duke", admin_id).await.unwrap();

        let logs = audit_repo.get_logs();
        assert!(
            logs.iter().any(|l| l.action == "tier_delete"),
            "TC08: admin_logs must have tier_delete"
        );

        // Soft-deleted tier should not appear in list
        let list = svc.list_tiers(1, 20).await.unwrap();
        assert_eq!(list.total, 0, "TC08: soft-deleted tier must not appear in list");
    }

    // ── TC09: GET list 分页正确 ────────────────────────────────────────────────
    #[tokio::test]
    async fn tc09_list_tiers_pagination() {
        let (svc, _, _, repo) = make_service();

        // Seed 5 tiers
        for i in 1i16..=5 {
            let mut req = base_create_req();
            req.tier_id = format!("tier_{i}");
            req.level = i;
            svc.create_tier(req, Uuid::new_v4()).await.unwrap();
        }

        let page1 = svc.list_tiers(1, 2).await.unwrap();
        assert_eq!(page1.total, 5, "TC09: total should be 5");
        assert_eq!(page1.items.len(), 2, "TC09: page 1 should have 2 items");

        let page3 = svc.list_tiers(3, 2).await.unwrap();
        assert_eq!(page3.items.len(), 1, "TC09: page 3 should have 1 item");

        let _ = repo; // ensure repo is still alive
    }

    // ── TC10: GET list 仅返回 is_active=true ──────────────────────────────────
    #[tokio::test]
    async fn tc10_list_tiers_only_active() {
        let (svc, _, _, _) = make_service();
        let admin_id = Uuid::new_v4();

        svc.create_tier(base_create_req(), admin_id).await.unwrap();
        svc.delete_tier("duke", admin_id).await.unwrap();

        let list = svc.list_tiers(1, 20).await.unwrap();
        assert_eq!(list.total, 0, "TC10: soft-deleted tier must not appear");
    }

    // ── TC11: 非 NobleTierWrite 权限 → 403 (handled at controller level) ────────
    // This is tested at controller level; service validates business rules only.
    // Covered by permission matrix tests in context.rs.

    // ── TC12: Redis publish noble_tiers_invalidate called ────────────────────
    #[tokio::test]
    async fn tc12_redis_publish_noble_tiers_invalidate() {
        let (svc, _, publisher, _) = make_service();

        svc.create_tier(base_create_req(), Uuid::new_v4())
            .await
            .unwrap();

        let raw_calls = publisher.raw_calls.lock().unwrap();
        assert_eq!(raw_calls.len(), 1);
        assert_eq!(raw_calls[0].1.event_type, "noble_tiers_invalidate");
        let payload = &raw_calls[0].1.payload;
        assert_eq!(payload["tier_id"].as_str(), Some("duke"));
    }

    // ── TC13/14: level bounds validation ────────────────────────────────────────
    #[tokio::test]
    async fn tc13_create_tier_level_zero() {
        let (svc, _, _, _) = make_service();
        let mut req = base_create_req();
        req.level = 0;
        let err = svc.create_tier(req, Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    #[tokio::test]
    async fn tc14_create_tier_level_seven() {
        let (svc, _, _, _) = make_service();
        let mut req = base_create_req();
        req.level = 7;
        let err = svc.create_tier(req, Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    // ── GR01: super_admin 赠送成功 → user_nobles +1, admin_logs +1, Redis publish ─
    #[tokio::test]
    async fn gr01_grant_noble_success() {
        let (svc, audit_repo, publisher, repo) = make_service();
        let admin_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // Seed tier
        svc.create_tier(base_create_req(), admin_id).await.unwrap();

        let req = GrantRequest {
            tier_id: "duke".to_string(),
            duration_days: 30,
            reason: "VIP 补偿".to_string(),
        };
        let resp = svc.grant_noble(user_id, req, admin_id).await;
        assert!(resp.is_ok(), "GR01: grant_noble should succeed");
        let un = resp.unwrap();
        assert_eq!(un.tier_id, "duke");
        assert_eq!(un.renew_channel, "admin_grant");

        // noble_history +1
        let hist = repo.get_history();
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].event, "admin_grant");

        // admin_logs: noble_grant
        let logs = audit_repo.get_logs();
        assert!(logs.iter().any(|l| l.action == "noble_grant"), "GR01: admin_logs must have noble_grant");

        // Redis publish
        let raw_calls = publisher.raw_calls.lock().unwrap();
        let grant_call = raw_calls.iter().find(|(_, e)| e.event_type == "noble_grant");
        assert!(grant_call.is_some(), "GR01: Redis publish noble_grant should be called");
        let payload = &grant_call.unwrap().1.payload;
        assert_eq!(payload["reason"].as_str(), Some("admin_grant"));
    }

    // ── GR03: reason 为空字符串 → 400 ───────────────────────────────────────────
    #[tokio::test]
    async fn gr03_grant_empty_reason() {
        let (svc, _, _, _) = make_service();
        let req = GrantRequest {
            tier_id: "duke".to_string(),
            duration_days: 30,
            reason: "".to_string(),
        };
        let err = svc.grant_noble(Uuid::new_v4(), req, Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "GR03: empty reason → ValidationError");
    }

    // ── GR04: reason 缺省（body 无 reason key）→ 400 ─────────────────────────────
    #[tokio::test]
    async fn gr04_grant_whitespace_reason() {
        let (svc, _, _, _) = make_service();
        let req = GrantRequest {
            tier_id: "duke".to_string(),
            duration_days: 30,
            reason: "   ".to_string(),
        };
        let err = svc.grant_noble(Uuid::new_v4(), req, Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "GR04: whitespace reason → ValidationError");
    }

    // ── GR05: tier_id 不存在 → 404 ──────────────────────────────────────────────
    #[tokio::test]
    async fn gr05_grant_tier_not_found() {
        let (svc, _, _, _) = make_service();
        let req = GrantRequest {
            tier_id: "nonexistent".to_string(),
            duration_days: 30,
            reason: "test".to_string(),
        };
        let err = svc.grant_noble(Uuid::new_v4(), req, Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "GR05: tier not found → NotFound");
    }

    // ── GR06: tier_id 已下架 → TierInactive ─────────────────────────────────────
    #[tokio::test]
    async fn gr06_grant_tier_inactive() {
        let (svc, _, _, _) = make_service();
        let admin_id = Uuid::new_v4();
        svc.create_tier(base_create_req(), admin_id).await.unwrap();
        svc.delete_tier("duke", admin_id).await.unwrap();

        let req = GrantRequest {
            tier_id: "duke".to_string(),
            duration_days: 30,
            reason: "test".to_string(),
        };
        let err = svc.grant_noble(Uuid::new_v4(), req, admin_id).await.unwrap_err();
        assert!(matches!(err, AppError::TierInactive(_)), "GR06: inactive tier → TierInactive");
    }

    // ── GR07: 重复赠送合并有效期 ────────────────────────────────────────────────────
    #[tokio::test]
    async fn gr07_grant_merge_expire_at() {
        let (svc, _, _, _) = make_service();
        let admin_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        svc.create_tier(base_create_req(), admin_id).await.unwrap();

        // First grant: 30 days
        let req1 = GrantRequest {
            tier_id: "duke".to_string(),
            duration_days: 30,
            reason: "first".to_string(),
        };
        let first = svc.grant_noble(user_id, req1, admin_id).await.unwrap();
        let first_expire = first.expire_at;

        // Second grant: 30 days → expire_at should be first_expire + 30d
        let req2 = GrantRequest {
            tier_id: "duke".to_string(),
            duration_days: 30,
            reason: "second".to_string(),
        };
        let second = svc.grant_noble(user_id, req2, admin_id).await.unwrap();
        let expected_min = first_expire + Duration::days(29); // allow 1 second slack
        assert!(
            second.expire_at >= expected_min,
            "GR07: second grant must extend expire_at by 30 days from first"
        );
    }

    // ── GR09: renew_channel = admin_grant ────────────────────────────────────────
    #[tokio::test]
    async fn gr09_grant_renew_channel_is_admin_grant() {
        let (svc, _, _, _) = make_service();
        let admin_id = Uuid::new_v4();
        svc.create_tier(base_create_req(), admin_id).await.unwrap();

        let req = GrantRequest {
            tier_id: "duke".to_string(),
            duration_days: 30,
            reason: "test".to_string(),
        };
        let resp = svc.grant_noble(Uuid::new_v4(), req, admin_id).await.unwrap();
        assert_eq!(resp.renew_channel, "admin_grant", "GR09: renew_channel must be 'admin_grant'");
    }

    // ── GR10: revoke 成功 → user_nobles 删除, noble_history +1, Redis publish ─────
    #[tokio::test]
    async fn gr10_revoke_noble_success() {
        let (svc, audit_repo, publisher, repo) = make_service();
        let admin_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        svc.create_tier(base_create_req(), admin_id).await.unwrap();
        svc.grant_noble(
            user_id,
            GrantRequest {
                tier_id: "duke".to_string(),
                duration_days: 30,
                reason: "grant first".to_string(),
            },
            admin_id,
        )
        .await
        .unwrap();

        let revoke_req = RevokeRequest {
            reason: "违规处罚".to_string(),
        };
        let result = svc.revoke_noble(user_id, revoke_req, admin_id).await;
        assert!(result.is_ok(), "GR10: revoke_noble should succeed");

        // noble_history: admin_revoke
        let hist = repo.get_history();
        assert!(
            hist.iter().any(|h| h.event == "admin_revoke"),
            "GR10: noble_history should have admin_revoke"
        );

        // admin_logs: noble_revoke
        let logs = audit_repo.get_logs();
        assert!(
            logs.iter().any(|l| l.action == "noble_revoke"),
            "GR10: admin_logs must have noble_revoke"
        );

        // Redis publish noble_revoke with to_tier_id=null
        let raw_calls = publisher.raw_calls.lock().unwrap();
        let revoke_call = raw_calls.iter().find(|(_, e)| e.event_type == "noble_revoke");
        assert!(revoke_call.is_some(), "GR10: Redis publish noble_revoke should be called");
        let payload = &revoke_call.unwrap().1.payload;
        assert!(payload["to_tier_id"].is_null(), "GR10: to_tier_id must be null");
        assert_eq!(payload["reason"].as_str(), Some("admin_revoke"));
    }

    // ── GR11: revoke 对无贵族的用户 → 404 ─────────────────────────────────────────
    #[tokio::test]
    async fn gr11_revoke_no_noble_user() {
        let (svc, _, _, _) = make_service();
        let revoke_req = RevokeRequest {
            reason: "test".to_string(),
        };
        let err = svc
            .revoke_noble(Uuid::new_v4(), revoke_req, Uuid::new_v4())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)), "GR11: no noble → NotFound");
    }

    // ── NQ01: status=active 仅返回 expire_at > now() ─────────────────────────────
    #[tokio::test]
    async fn nq01_list_users_status_active() {
        use super::super::dto::NobleStatusFilter;
        use crate::modules::nobility::repository::UserNobleRow;

        let (svc, _, _, repo) = make_service();
        let now = Utc::now();

        // Active user
        let active_uid = Uuid::new_v4();
        repo.push_user_noble(
            UserNobleRow {
                user_id: active_uid,
                tier_id: "duke".to_string(),
                start_at: now - Duration::days(10),
                current_period_start: now - Duration::days(10),
                expire_at: now + Duration::days(20), // active
                auto_renew: true,
                renew_channel: "diamonds".to_string(),
                total_paid_diamonds: 300_000,
                total_paid_usd_micros: 0,
            },
            "Alice",
            None,
            "Duke",
            "دوق",
            5,
            "#06B6D4",
        );

        // Expired user
        let expired_uid = Uuid::new_v4();
        repo.push_user_noble(
            UserNobleRow {
                user_id: expired_uid,
                tier_id: "knight".to_string(),
                start_at: now - Duration::days(40),
                current_period_start: now - Duration::days(40),
                expire_at: now - Duration::days(5), // expired
                auto_renew: false,
                renew_channel: "diamonds".to_string(),
                total_paid_diamonds: 3_000,
                total_paid_usd_micros: 0,
            },
            "Bob",
            None,
            "Knight",
            "فارس",
            1,
            "#6B7280",
        );

        let query = ListUsersQuery {
            status: Some(NobleStatusFilter::Active),
            page: 1,
            size: 20,
            ..Default::default()
        };
        let resp = svc.list_noble_users(query).await.unwrap();
        assert_eq!(resp.total, 1, "NQ01: only 1 active user");
        assert_eq!(resp.items[0].user_id, active_uid);
    }

    // ── NQ02: status=expired 仅返回 expire_at <= now() ────────────────────────────
    #[tokio::test]
    async fn nq02_list_users_status_expired() {
        use super::super::dto::NobleStatusFilter;
        use crate::modules::nobility::repository::UserNobleRow;

        let (svc, _, _, repo) = make_service();
        let now = Utc::now();

        // Active
        repo.push_user_noble(
            UserNobleRow {
                user_id: Uuid::new_v4(),
                tier_id: "duke".to_string(),
                start_at: now,
                current_period_start: now,
                expire_at: now + Duration::days(10),
                auto_renew: true,
                renew_channel: "diamonds".to_string(),
                total_paid_diamonds: 0,
                total_paid_usd_micros: 0,
            },
            "Active",
            None,
            "Duke",
            "دوق",
            5,
            "#06B6D4",
        );

        // Expired
        let exp_uid = Uuid::new_v4();
        repo.push_user_noble(
            UserNobleRow {
                user_id: exp_uid,
                tier_id: "knight".to_string(),
                start_at: now - Duration::days(40),
                current_period_start: now - Duration::days(40),
                expire_at: now - Duration::days(1),
                auto_renew: false,
                renew_channel: "diamonds".to_string(),
                total_paid_diamonds: 0,
                total_paid_usd_micros: 0,
            },
            "Expired",
            None,
            "Knight",
            "فارس",
            1,
            "#6B7280",
        );

        let query = ListUsersQuery {
            status: Some(NobleStatusFilter::Expired),
            page: 1,
            size: 20,
            ..Default::default()
        };
        let resp = svc.list_noble_users(query).await.unwrap();
        assert_eq!(resp.total, 1, "NQ02: only 1 expired user");
        assert_eq!(resp.items[0].user_id, exp_uid);
    }

    // ── NQ04: tier_id filter ────────────────────────────────────────────────────
    #[tokio::test]
    async fn nq04_list_users_filter_by_tier_id() {
        use crate::modules::nobility::repository::UserNobleRow;

        let (svc, _, _, repo) = make_service();
        let now = Utc::now();

        let duke_uid = Uuid::new_v4();
        repo.push_user_noble(
            UserNobleRow {
                user_id: duke_uid,
                tier_id: "duke".to_string(),
                start_at: now,
                current_period_start: now,
                expire_at: now + Duration::days(30),
                auto_renew: true,
                renew_channel: "diamonds".to_string(),
                total_paid_diamonds: 0,
                total_paid_usd_micros: 0,
            },
            "DukeUser",
            None,
            "Duke",
            "دوق",
            5,
            "#06B6D4",
        );
        repo.push_user_noble(
            UserNobleRow {
                user_id: Uuid::new_v4(),
                tier_id: "knight".to_string(),
                start_at: now,
                current_period_start: now,
                expire_at: now + Duration::days(30),
                auto_renew: true,
                renew_channel: "diamonds".to_string(),
                total_paid_diamonds: 0,
                total_paid_usd_micros: 0,
            },
            "KnightUser",
            None,
            "Knight",
            "فارس",
            1,
            "#6B7280",
        );

        let query = ListUsersQuery {
            tier_id: Some("duke".to_string()),
            page: 1,
            size: 20,
            ..Default::default()
        };
        let resp = svc.list_noble_users(query).await.unwrap();
        assert_eq!(resp.total, 1, "NQ04: only duke users");
        assert_eq!(resp.items[0].user_id, duke_uid);
    }

    // ── NQ05: expire_before filter ──────────────────────────────────────────────
    #[tokio::test]
    async fn nq05_list_users_expire_before() {
        use crate::modules::nobility::repository::UserNobleRow;

        let (svc, _, _, repo) = make_service();
        let now = Utc::now();

        // User expiring in 5 days
        let soon_uid = Uuid::new_v4();
        repo.push_user_noble(
            UserNobleRow {
                user_id: soon_uid,
                tier_id: "duke".to_string(),
                start_at: now,
                current_period_start: now,
                expire_at: now + Duration::days(5),
                auto_renew: true,
                renew_channel: "diamonds".to_string(),
                total_paid_diamonds: 0,
                total_paid_usd_micros: 0,
            },
            "SoonExpire",
            None,
            "Duke",
            "دوق",
            5,
            "#06B6D4",
        );

        // User expiring in 60 days
        repo.push_user_noble(
            UserNobleRow {
                user_id: Uuid::new_v4(),
                tier_id: "duke".to_string(),
                start_at: now,
                current_period_start: now,
                expire_at: now + Duration::days(60),
                auto_renew: true,
                renew_channel: "diamonds".to_string(),
                total_paid_diamonds: 0,
                total_paid_usd_micros: 0,
            },
            "LaterExpire",
            None,
            "Duke",
            "دوق",
            5,
            "#06B6D4",
        );

        // Expire before 30 days from now
        let expire_before_date = (now + Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        let query = ListUsersQuery {
            expire_before: Some(expire_before_date),
            page: 1,
            size: 20,
            ..Default::default()
        };
        let resp = svc.list_noble_users(query).await.unwrap();
        assert_eq!(resp.total, 1, "NQ05: only user with expire_at < +30d");
        assert_eq!(resp.items[0].user_id, soon_uid);
    }

    // ── NQ06: 分页 25 条数据 ───────────────────────────────────────────────────────
    #[tokio::test]
    async fn nq06_list_users_pagination() {
        use crate::modules::nobility::repository::UserNobleRow;

        let (svc, _, _, repo) = make_service();
        let now = Utc::now();

        for _ in 0..25 {
            repo.push_user_noble(
                UserNobleRow {
                    user_id: Uuid::new_v4(),
                    tier_id: "duke".to_string(),
                    start_at: now,
                    current_period_start: now,
                    expire_at: now + Duration::days(30),
                    auto_renew: true,
                    renew_channel: "diamonds".to_string(),
                    total_paid_diamonds: 0,
                    total_paid_usd_micros: 0,
                },
                "User",
                None,
                "Duke",
                "دوق",
                5,
                "#06B6D4",
            );
        }

        let p1 = svc.list_noble_users(ListUsersQuery { page: 1, size: 10, ..Default::default() }).await.unwrap();
        assert_eq!(p1.total, 25, "NQ06: total=25");
        assert_eq!(p1.items.len(), 10, "NQ06: page1 has 10 items");

        let p3 = svc.list_noble_users(ListUsersQuery { page: 3, size: 10, ..Default::default() }).await.unwrap();
        assert_eq!(p3.items.len(), 5, "NQ06: page3 has 5 items");
    }

    // ── NQ09: history 按 created_at DESC ───────────────────────────────────────────
    #[tokio::test]
    async fn nq09_noble_history_desc_order() {
        let (svc, _, _, repo) = make_service();
        let user_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();

        svc.create_tier(base_create_req(), admin_id).await.unwrap();

        // Grant twice
        for i in 1..=3u32 {
            svc.grant_noble(
                user_id,
                GrantRequest {
                    tier_id: "duke".to_string(),
                    duration_days: i as i32,
                    reason: format!("reason {i}"),
                },
                admin_id,
            )
            .await
            .unwrap();
        }

        let hist = svc.get_noble_history(user_id).await.unwrap();
        assert_eq!(hist.len(), 3, "NQ09: should have 3 history records");
        // Verify DESC order
        for window in hist.windows(2) {
            assert!(
                window[0].created_at >= window[1].created_at,
                "NQ09: history must be in DESC order"
            );
        }
        let _ = repo;
    }

    // ── NQ10: 无历史时返回空数组 ──────────────────────────────────────────────────
    #[tokio::test]
    async fn nq10_noble_history_empty() {
        let (svc, _, _, _) = make_service();
        let hist = svc.get_noble_history(Uuid::new_v4()).await.unwrap();
        assert!(hist.is_empty(), "NQ10: no history → empty vec");
    }

    // ── NQ13: size=101 → ValidationError ─────────────────────────────────────────
    #[tokio::test]
    async fn nq13_list_users_size_over_limit() {
        let (svc, _, _, _) = make_service();
        let query = ListUsersQuery {
            page: 1,
            size: 101,
            ..Default::default()
        };
        let err = svc.list_noble_users(query).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "NQ13: size=101 → ValidationError");
    }

    // ── NQ14: page=0 → ValidationError ────────────────────────────────────────────
    #[tokio::test]
    async fn nq14_list_users_page_zero() {
        let (svc, _, _, _) = make_service();
        let query = ListUsersQuery {
            page: 0,
            size: 20,
            ..Default::default()
        };
        let err = svc.list_noble_users(query).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "NQ14: page=0 → ValidationError");
    }

    // ── NQ15: expire_before 格式非法 → ValidationError ──────────────────────────
    #[tokio::test]
    async fn nq15_list_users_invalid_expire_before() {
        let (svc, _, _, _) = make_service();
        let query = ListUsersQuery {
            expire_before: Some("abc".to_string()),
            page: 1,
            size: 20,
            ..Default::default()
        };
        let err = svc.list_noble_users(query).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "NQ15: invalid date → ValidationError");
    }

    // ── NQ16: list_noble_users 与 count_noble_users 均被调用 ─────────────────────
    #[tokio::test]
    async fn nq16_both_query_and_count_called() {
        let (svc, _, _, repo) = make_service();
        let _resp = svc.list_noble_users(ListUsersQuery { page: 1, size: 10, ..Default::default() }).await.unwrap();
        let calls = repo.calls.lock().unwrap();
        assert!(calls.contains(&"list_noble_users".to_string()), "NQ16: list_noble_users called");
        assert!(calls.contains(&"count_noble_users".to_string()), "NQ16: count_noble_users called");
    }

    // ── Permission tests (from context.rs) ──────────────────────────────────────
    #[test]
    fn noble_tier_read_permissions() {
        use crate::common::auth::{AdminAuthContext, Permission};
        for role in ["super_admin", "operator", "cs"] {
            let ctx = AdminAuthContext::new(Uuid::new_v4(), role);
            assert!(ctx.has_permission(Permission::NobleTierRead), "{role} should have NobleTierRead");
        }
        let finance = AdminAuthContext::new(Uuid::new_v4(), "finance");
        assert!(!finance.has_permission(Permission::NobleTierRead), "finance must not have NobleTierRead");
    }

    #[test]
    fn noble_tier_write_permissions() {
        use crate::common::auth::{AdminAuthContext, Permission};
        for role in ["super_admin", "operator"] {
            let ctx = AdminAuthContext::new(Uuid::new_v4(), role);
            assert!(ctx.has_permission(Permission::NobleTierWrite), "{role} should have NobleTierWrite");
        }
        for role in ["cs", "finance"] {
            let ctx = AdminAuthContext::new(Uuid::new_v4(), role);
            assert!(!ctx.has_permission(Permission::NobleTierWrite), "{role} must not have NobleTierWrite");
        }
    }

    // ── error code mapping tests ─────────────────────────────────────────────────
    #[test]
    fn tier_inactive_maps_to_40914_404() {
        use axum::http::StatusCode;
        let err = AppError::TierInactive("duke".to_string());
        assert_eq!(err.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_code() as i32, 40914);
    }

    #[test]
    fn privileges_schema_invalid_maps_to_40917_422() {
        use axum::http::StatusCode;
        let err = AppError::PrivilegesSchemaInvalid("bad percent".to_string());
        assert_eq!(err.http_status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.error_code() as i32, 40917);
    }

    #[test]
    fn tier_level_conflict_maps_to_409() {
        use axum::http::StatusCode;
        let err = AppError::TierLevelConflict(5);
        assert_eq!(err.http_status(), StatusCode::CONFLICT);
    }
}
