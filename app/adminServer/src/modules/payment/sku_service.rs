//! T-10027: SkuService — SKU 业务逻辑

use std::sync::Arc;


use crate::common::error::AppError;

use super::{
    sku_dto::{CreateSkuRequest, CreateSkuResponse, SkuResponse, UpdateSkuRequest, UpdateSkuQuery},
    sku_repo::{CreateSkuParams, SkuRepository, UpdateSkuParams},
};

#[cfg(any(test, feature = "test-utils"))]
use super::sku_repo::FakeSkuRepository;

// ─── 价格变更确认响应 ─────────────────────────────────────────────────────────

/// 当价格/钻石变更但未携带 confirm=true 时返回此 diff。
#[derive(Debug, serde::Serialize)]
pub struct PriceChangeDiff {
    pub sku_id: String,
    pub diamonds_before: i64,
    pub diamonds_after: i64,
    pub price_before: String,
    pub price_after: String,
}

// ─── Service ─────────────────────────────────────────────────────────────────

pub struct SkuService {
    repo: Arc<dyn SkuRepository>,
}

impl SkuService {
    pub fn new(repo: Arc<dyn SkuRepository>) -> Self {
        Self { repo }
    }

    /// 列出所有 SKU（含下架）。
    pub async fn list_skus(&self) -> Result<Vec<SkuResponse>, AppError> {
        let rows = self.repo.list_all().await?;
        Ok(rows.iter().map(|r| r.to_response()).collect())
    }

    /// 创建新 SKU。
    /// - 校验 diamonds / price
    /// - sku_id 格式警告（non-blocking）
    /// - sku_id 重复 → SkuConflict
    pub async fn create_sku(
        &self,
        req: CreateSkuRequest,
    ) -> Result<CreateSkuResponse, AppError> {
        req.validate().map_err(|e| AppError::ValidationError(e))?;

        let warning = req.sku_id_warning();

        let params = CreateSkuParams {
            sku_id: req.sku_id.clone(),
            provider: req.provider.clone(),
            diamonds: req.diamonds,
            display_price_usd: req.display_price_usd.clone(),
            display_price_local: req.display_price_local.clone(),
            display_currency: req.display_currency.clone(),
            sort_order: req.sort_order.unwrap_or(0),
            tag: req.tag.clone(),
            is_active: req.is_active.unwrap_or(true),
        };

        let row = self.repo.insert(params).await?;

        Ok(CreateSkuResponse {
            sku: row.to_response(),
            warning,
        })
    }

    /// 更新 SKU。
    ///
    /// 若修改了 diamonds / display_price_usd 且 `confirm` 为 false，
    /// 返回 PriceChangeRequiresConfirm 错误（含 diff payload）。
    pub async fn update_sku(
        &self,
        sku_id: &str,
        req: UpdateSkuRequest,
        query: UpdateSkuQuery,
    ) -> Result<SkuResponse, AppError> {
        req.validate().map_err(|e| AppError::ValidationError(e))?;

        // 先查出当前 SKU（用于价格 diff 校验和 NotFound）
        let current = self
            .repo
            .find_by_id(sku_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("SKU {sku_id} not found")))?;

        // 价格变更需要确认
        if req.has_price_change(current.diamonds, &current.display_price_usd) {
            if !query.confirm.unwrap_or(false) {
                return Err(AppError::PriceChangeRequiresConfirm);
            }
        }

        let params = UpdateSkuParams {
            diamonds: req.diamonds,
            display_price_usd: req.display_price_usd,
            display_price_local: req.display_price_local,
            display_currency: req.display_currency,
            is_active: req.is_active,
            sort_order: req.sort_order,
            tag: req.tag,
        };

        let row = self.repo.update(sku_id, params).await?;
        Ok(row.to_response())
    }

    /// 软删 SKU（is_active=false）。
    pub async fn delete_sku(&self, sku_id: &str) -> Result<SkuResponse, AppError> {
        let row = self.repo.soft_delete(sku_id).await?;
        Ok(row.to_response())
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::payment::sku_repo::SkuRow;
    use chrono::Utc;

    fn make_service() -> SkuService {
        SkuService::new(Arc::new(FakeSkuRepository::default()))
    }

    fn seed_sku(service: &SkuService, sku_id: &str) {
        let repo = service.repo.clone();
        // downcast trick: we know it's FakeSkuRepository during test
        // Instead, rebuild service with pre-seeded data via seed() helper
        // (This works because FakeSkuRepository exposes seed())
    }

    fn make_seeded_service(sku_id: &str, is_active: bool) -> SkuService {
        let fake = Arc::new(FakeSkuRepository::default());
        fake.seed(SkuRow {
            sku_id: sku_id.to_string(),
            provider: "google_play".to_string(),
            diamonds: 600,
            display_price_usd: "9.99".to_string(),
            display_price_local: None,
            display_currency: None,
            is_active,
            sort_order: 30,
            tag: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
        SkuService::new(fake)
    }

    fn update_req_only_sort(sort: i32) -> UpdateSkuRequest {
        UpdateSkuRequest {
            diamonds: None,
            display_price_usd: None,
            display_price_local: None,
            display_currency: None,
            is_active: None,
            sort_order: Some(sort),
            tag: None,
        }
    }

    // ── SK-02: create 正常流程 ────────────────────────────────────────────

    #[tokio::test]
    async fn sk02_create_sku_happy_path() {
        let svc = make_service();
        let req = CreateSkuRequest {
            sku_id: "diamond_600".to_string(),
            provider: "google_play".to_string(),
            diamonds: 600,
            display_price_usd: "9.99".to_string(),
            display_price_local: None,
            display_currency: None,
            sort_order: None,
            tag: None,
            is_active: None,
        };
        let resp = svc.create_sku(req).await.unwrap();
        assert_eq!(resp.sku.sku_id, "diamond_600");
        assert_eq!(resp.sku.diamonds, 600);
        assert!(resp.warning.is_none());
    }

    // ── SK-09: diamonds=0 → ValidationError ──────────────────────────────

    #[tokio::test]
    async fn sk09_create_diamonds_zero_returns_validation_error() {
        let svc = make_service();
        let req = CreateSkuRequest {
            sku_id: "diamond_0".to_string(),
            provider: "google_play".to_string(),
            diamonds: 0,
            display_price_usd: "9.99".to_string(),
            display_price_local: None,
            display_currency: None,
            sort_order: None,
            tag: None,
            is_active: None,
        };
        let err = svc.create_sku(req).await.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)), "{err:?}");
    }

    // ── SK-12: 重复 sku_id → SkuConflict ─────────────────────────────────

    #[tokio::test]
    async fn sk12_duplicate_sku_id_returns_conflict() {
        let svc = make_seeded_service("diamond_600", true);
        let req = CreateSkuRequest {
            sku_id: "diamond_600".to_string(),
            provider: "google_play".to_string(),
            diamonds: 600,
            display_price_usd: "9.99".to_string(),
            display_price_local: None,
            display_currency: None,
            sort_order: None,
            tag: None,
            is_active: None,
        };
        let err = svc.create_sku(req).await.unwrap_err();
        assert!(matches!(err, AppError::SkuConflict(_)), "{err:?}");
    }

    // ── SK-03: sku_id 格式不符 → warning 返回 ────────────────────────────

    #[tokio::test]
    async fn sk03_sku_id_bad_format_returns_warning() {
        let svc = make_service();
        let req = CreateSkuRequest {
            sku_id: "Diamond_600".to_string(), // uppercase → warning
            provider: "google_play".to_string(),
            diamonds: 600,
            display_price_usd: "9.99".to_string(),
            display_price_local: None,
            display_currency: None,
            sort_order: None,
            tag: None,
            is_active: None,
        };
        let resp = svc.create_sku(req).await.unwrap();
        assert!(resp.warning.is_some(), "SK-03: should return warning");
    }

    // ── SK-05: update sort_order only (no price change) → ok ─────────────

    #[tokio::test]
    async fn sk05_update_non_price_field_no_confirm_needed() {
        let svc = make_seeded_service("diamond_600", true);
        let result = svc
            .update_sku("diamond_600", update_req_only_sort(99), UpdateSkuQuery::default())
            .await
            .unwrap();
        assert_eq!(result.sort_order, 99);
    }

    // ── SK-08: price change without confirm → PriceChangeRequiresConfirm ─

    #[tokio::test]
    async fn sk08_price_change_without_confirm_returns_422() {
        let svc = make_seeded_service("diamond_600", true);
        let req = UpdateSkuRequest {
            diamonds: Some(1200), // price change
            display_price_usd: None,
            display_price_local: None,
            display_currency: None,
            is_active: None,
            sort_order: None,
            tag: None,
        };
        let err = svc
            .update_sku("diamond_600", req, UpdateSkuQuery { confirm: None })
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::PriceChangeRequiresConfirm),
            "SK-08: {err:?}"
        );
    }

    // ── SK-07: price change with confirm=true → ok ────────────────────────

    #[tokio::test]
    async fn sk07_price_change_with_confirm_true_succeeds() {
        let svc = make_seeded_service("diamond_600", true);
        let req = UpdateSkuRequest {
            diamonds: Some(1200),
            display_price_usd: None,
            display_price_local: None,
            display_currency: None,
            is_active: None,
            sort_order: None,
            tag: None,
        };
        let result = svc
            .update_sku("diamond_600", req, UpdateSkuQuery { confirm: Some(true) })
            .await
            .unwrap();
        assert_eq!(result.diamonds, 1200);
    }

    // ── SK-11: update 不存在 → NotFound ──────────────────────────────────

    #[tokio::test]
    async fn sk11_update_nonexistent_returns_not_found() {
        let svc = make_service();
        let req = update_req_only_sort(10);
        let err = svc
            .update_sku("nonexistent", req, UpdateSkuQuery::default())
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    // ── SK-06: delete → is_active=false ──────────────────────────────────

    #[tokio::test]
    async fn sk06_delete_sku_sets_inactive() {
        let svc = make_seeded_service("diamond_600", true);
        let result = svc.delete_sku("diamond_600").await.unwrap();
        assert!(!result.is_active, "SK-06: deleted SKU must be inactive");
    }

    // ── SK-14: delete 不存在 → NotFound ──────────────────────────────────

    #[tokio::test]
    async fn sk14_delete_nonexistent_returns_not_found() {
        let svc = make_service();
        let err = svc.delete_sku("nonexistent").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
