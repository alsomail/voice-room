//! T-10027: SkuRepository — SKU 数据访问层

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::error::AppError;

use super::sku_dto::SkuResponse;

// ─── 数据行 ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SkuRow {
    pub sku_id: String,
    pub provider: String,
    pub diamonds: i64,
    pub display_price_usd: String,
    pub display_price_local: Option<String>,
    pub display_currency: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub tag: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SkuRow {
    pub fn to_response(&self) -> SkuResponse {
        SkuResponse {
            sku_id: self.sku_id.clone(),
            provider: self.provider.clone(),
            diamonds: self.diamonds,
            display_price_usd: self.display_price_usd.clone(),
            display_price_local: self.display_price_local.clone(),
            display_currency: self.display_currency.clone(),
            is_active: self.is_active,
            sort_order: self.sort_order,
            tag: self.tag.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

// ─── 创建参数 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateSkuParams {
    pub sku_id: String,
    pub provider: String,
    pub diamonds: i64,
    pub display_price_usd: String,
    pub display_price_local: Option<String>,
    pub display_currency: Option<String>,
    pub sort_order: i32,
    pub tag: Option<String>,
    pub is_active: bool,
}

// ─── 更新参数 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct UpdateSkuParams {
    pub diamonds: Option<i64>,
    pub display_price_usd: Option<String>,
    pub display_price_local: Option<String>,
    pub display_currency: Option<String>,
    pub is_active: Option<bool>,
    pub sort_order: Option<i32>,
    pub tag: Option<String>,
}

// ─── Trait ────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait SkuRepository: Send + Sync {
    /// 列出所有 SKU（含 is_active=false）
    async fn list_all(&self) -> Result<Vec<SkuRow>, AppError>;

    /// 按 sku_id 查找
    async fn find_by_id(&self, sku_id: &str) -> Result<Option<SkuRow>, AppError>;

    /// 插入新 SKU（sku_id 重复 → SkuConflict）
    async fn insert(&self, params: CreateSkuParams) -> Result<SkuRow, AppError>;

    /// 更新 SKU（sku_id 不存在 → NotFound）
    async fn update(&self, sku_id: &str, params: UpdateSkuParams) -> Result<SkuRow, AppError>;

    /// 软删（is_active=false，行保留）
    async fn soft_delete(&self, sku_id: &str) -> Result<SkuRow, AppError>;
}

// ─── Fake 实现 ────────────────────────────────────────────────────────────────

#[cfg(any(test, feature = "test-utils"))]
#[derive(Default)]
pub struct FakeSkuRepository {
    skus: Arc<Mutex<Vec<SkuRow>>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeSkuRepository {
    pub fn seed(&self, row: SkuRow) {
        self.skus.lock().unwrap().push(row);
    }

    pub fn get_all(&self) -> Vec<SkuRow> {
        self.skus.lock().unwrap().clone()
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl SkuRepository for FakeSkuRepository {
    async fn list_all(&self) -> Result<Vec<SkuRow>, AppError> {
        Ok(self.skus.lock().unwrap().clone())
    }

    async fn find_by_id(&self, sku_id: &str) -> Result<Option<SkuRow>, AppError> {
        Ok(self
            .skus
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.sku_id == sku_id)
            .cloned())
    }

    async fn insert(&self, params: CreateSkuParams) -> Result<SkuRow, AppError> {
        let mut guard = self.skus.lock().unwrap();
        if guard.iter().any(|s| s.sku_id == params.sku_id) {
            return Err(AppError::SkuConflict(params.sku_id));
        }
        let now = Utc::now();
        let row = SkuRow {
            sku_id: params.sku_id,
            provider: params.provider,
            diamonds: params.diamonds,
            display_price_usd: params.display_price_usd,
            display_price_local: params.display_price_local,
            display_currency: params.display_currency,
            is_active: params.is_active,
            sort_order: params.sort_order,
            tag: params.tag,
            created_at: now,
            updated_at: now,
        };
        guard.push(row.clone());
        Ok(row)
    }

    async fn update(&self, sku_id: &str, params: UpdateSkuParams) -> Result<SkuRow, AppError> {
        let mut guard = self.skus.lock().unwrap();
        let row = guard
            .iter_mut()
            .find(|s| s.sku_id == sku_id)
            .ok_or_else(|| AppError::NotFound(format!("SKU {sku_id} not found")))?;

        if let Some(d) = params.diamonds {
            row.diamonds = d;
        }
        if let Some(p) = params.display_price_usd {
            row.display_price_usd = p;
        }
        if let Some(l) = params.display_price_local {
            row.display_price_local = Some(l);
        }
        if let Some(c) = params.display_currency {
            row.display_currency = Some(c);
        }
        if let Some(a) = params.is_active {
            row.is_active = a;
        }
        if let Some(o) = params.sort_order {
            row.sort_order = o;
        }
        if let Some(t) = params.tag {
            row.tag = Some(t);
        }
        row.updated_at = Utc::now();

        Ok(row.clone())
    }

    async fn soft_delete(&self, sku_id: &str) -> Result<SkuRow, AppError> {
        let mut guard = self.skus.lock().unwrap();
        let row = guard
            .iter_mut()
            .find(|s| s.sku_id == sku_id)
            .ok_or_else(|| AppError::NotFound(format!("SKU {sku_id} not found")))?;
        row.is_active = false;
        row.updated_at = Utc::now();
        Ok(row.clone())
    }
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct SkuDbRow {
    pub sku_id: String,
    pub provider: String,
    pub diamonds: i64,
    pub display_price_usd: String,
    pub display_price_local: Option<String>,
    pub display_currency: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub tag: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SkuDbRow> for SkuRow {
    fn from(r: SkuDbRow) -> Self {
        SkuRow {
            sku_id: r.sku_id,
            provider: r.provider,
            diamonds: r.diamonds,
            display_price_usd: r.display_price_usd,
            display_price_local: r.display_price_local,
            display_currency: r.display_currency,
            is_active: r.is_active,
            sort_order: r.sort_order,
            tag: r.tag,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

pub struct PgSkuRepository {
    pool: PgPool,
}

impl PgSkuRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SkuRepository for PgSkuRepository {
    async fn list_all(&self) -> Result<Vec<SkuRow>, AppError> {
        let rows = sqlx::query_as::<_, SkuDbRow>(
            "SELECT sku_id, provider::text, diamonds, display_price_usd, \
                    display_price_local, display_currency, is_active, \
                    sort_order, tag, created_at, updated_at \
             FROM payment_skus \
             ORDER BY sort_order ASC, created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_by_id(&self, sku_id: &str) -> Result<Option<SkuRow>, AppError> {
        let row = sqlx::query_as::<_, SkuDbRow>(
            "SELECT sku_id, provider::text, diamonds, display_price_usd, \
                    display_price_local, display_currency, is_active, \
                    sort_order, tag, created_at, updated_at \
             FROM payment_skus WHERE sku_id = $1",
        )
        .bind(sku_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn insert(&self, params: CreateSkuParams) -> Result<SkuRow, AppError> {
        let row = sqlx::query_as::<_, SkuDbRow>(
            "INSERT INTO payment_skus \
               (sku_id, provider, diamonds, display_price_usd, display_price_local, \
                display_currency, is_active, sort_order, tag) \
             VALUES ($1, $2::payment_provider, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING sku_id, provider::text, diamonds, display_price_usd, \
                       display_price_local, display_currency, is_active, \
                       sort_order, tag, created_at, updated_at",
        )
        .bind(&params.sku_id)
        .bind(&params.provider)
        .bind(params.diamonds)
        .bind(&params.display_price_usd)
        .bind(&params.display_price_local)
        .bind(&params.display_currency)
        .bind(params.is_active)
        .bind(params.sort_order)
        .bind(&params.tag)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            // unique violation → SkuConflict
            if let sqlx::Error::Database(ref dbe) = e {
                if dbe.code().as_deref() == Some("23505") {
                    return AppError::SkuConflict(params.sku_id.clone());
                }
            }
            AppError::DatabaseError(e.to_string())
        })?;
        Ok(row.into())
    }

    async fn update(&self, sku_id: &str, params: UpdateSkuParams) -> Result<SkuRow, AppError> {
        // Build dynamic SET clauses
        let row = sqlx::query_as::<_, SkuDbRow>(
            "UPDATE payment_skus SET \
               diamonds = COALESCE($2, diamonds), \
               display_price_usd = COALESCE($3, display_price_usd), \
               display_price_local = COALESCE($4, display_price_local), \
               display_currency = COALESCE($5, display_currency), \
               is_active = COALESCE($6, is_active), \
               sort_order = COALESCE($7, sort_order), \
               tag = COALESCE($8, tag), \
               updated_at = now() \
             WHERE sku_id = $1 \
             RETURNING sku_id, provider::text, diamonds, display_price_usd, \
                       display_price_local, display_currency, is_active, \
                       sort_order, tag, created_at, updated_at",
        )
        .bind(sku_id)
        .bind(params.diamonds)
        .bind(params.display_price_usd)
        .bind(params.display_price_local)
        .bind(params.display_currency)
        .bind(params.is_active)
        .bind(params.sort_order)
        .bind(params.tag)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("SKU {sku_id} not found")))?;
        Ok(row.into())
    }

    async fn soft_delete(&self, sku_id: &str) -> Result<SkuRow, AppError> {
        let row = sqlx::query_as::<_, SkuDbRow>(
            "UPDATE payment_skus SET is_active = false, updated_at = now() \
             WHERE sku_id = $1 \
             RETURNING sku_id, provider::text, diamonds, display_price_usd, \
                       display_price_local, display_currency, is_active, \
                       sort_order, tag, created_at, updated_at",
        )
        .bind(sku_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("SKU {sku_id} not found")))?;
        Ok(row.into())
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_sku(sku_id: &str, is_active: bool) -> SkuRow {
        SkuRow {
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
        }
    }

    // ── SK-01: list 返回全部（含下架）────────────────────────────────────

    #[tokio::test]
    async fn sk01_list_includes_inactive() {
        let repo = FakeSkuRepository::default();
        repo.seed(make_sku("diamond_600", true));
        repo.seed(make_sku("diamond_9999", false));

        let skus = repo.list_all().await.unwrap();
        assert_eq!(skus.len(), 2, "SK-01: list must include inactive SKU");
    }

    // ── SK-02: insert 成功 ────────────────────────────────────────────────

    #[tokio::test]
    async fn sk02_insert_succeeds() {
        let repo = FakeSkuRepository::default();
        let row = repo
            .insert(CreateSkuParams {
                sku_id: "diamond_600".to_string(),
                provider: "google_play".to_string(),
                diamonds: 600,
                display_price_usd: "9.99".to_string(),
                display_price_local: None,
                display_currency: None,
                sort_order: 30,
                tag: None,
                is_active: true,
            })
            .await
            .unwrap();
        assert_eq!(row.sku_id, "diamond_600");
        assert_eq!(row.diamonds, 600);
        assert!(row.is_active);
    }

    // ── SK-12: 重复 sku_id → SkuConflict ─────────────────────────────────

    #[tokio::test]
    async fn sk12_duplicate_sku_id_returns_conflict() {
        let repo = FakeSkuRepository::default();
        repo.seed(make_sku("diamond_600", true));

        let err = repo
            .insert(CreateSkuParams {
                sku_id: "diamond_600".to_string(),
                provider: "google_play".to_string(),
                diamonds: 600,
                display_price_usd: "9.99".to_string(),
                display_price_local: None,
                display_currency: None,
                sort_order: 30,
                tag: None,
                is_active: true,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::SkuConflict(_)), "{err:?}");
    }

    // ── SK-06: soft_delete → is_active=false ─────────────────────────────

    #[tokio::test]
    async fn sk06_soft_delete_sets_inactive() {
        let repo = FakeSkuRepository::default();
        repo.seed(make_sku("diamond_600", true));

        let result = repo.soft_delete("diamond_600").await.unwrap();
        assert!(!result.is_active, "SK-06: soft_delete must set is_active=false");

        // 行仍存在
        let found = repo.find_by_id("diamond_600").await.unwrap();
        assert!(found.is_some(), "SK-06: soft_deleted row must still exist");
        assert!(!found.unwrap().is_active);
    }

    // ── SK-14: 删除不存在的 sku → NotFound ────────────────────────────────

    #[tokio::test]
    async fn sk14_soft_delete_nonexistent_returns_not_found() {
        let repo = FakeSkuRepository::default();
        let err = repo.soft_delete("nonexistent_sku").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    // ── SK-04: update sort_order only → success ───────────────────────────

    #[tokio::test]
    async fn sk04_update_sort_order_only() {
        let repo = FakeSkuRepository::default();
        repo.seed(make_sku("diamond_600", true));

        let result = repo
            .update(
                "diamond_600",
                UpdateSkuParams {
                    sort_order: Some(99),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(result.sort_order, 99);
        assert_eq!(result.diamonds, 600, "other fields unchanged");
    }
}
