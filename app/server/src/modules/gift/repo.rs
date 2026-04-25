//! Gift 仓储层
//!
//! - `GiftRepoPort` — 抽象接口，供 GiftService 注入，支持 FakeGiftRepo 测试替身
//! - `PgGiftRepo`   — SQLx/PostgreSQL 真实实现
//! - `FakeGiftRepo` — 内存测试替身，带调用计数器（用于 G06 缓存命中测试）

use async_trait::async_trait;
use sqlx::PgPool;
use voice_room_shared::models::gift::GiftModel;

use crate::common::error::AppError;

// ─── GiftRepoPort trait ───────────────────────────────────────────────────────

/// 礼物仓储抽象接口
///
/// 隔离真实 DB 与测试 Fake，使 GiftService 可在无数据库环境下单元测试。
#[async_trait]
pub trait GiftRepoPort: Send + Sync {
    /// 查询所有上架（is_active=true）且未删除（is_deleted=false）的礼物，
    /// 按 `tier ASC, sort_order ASC` 排序。
    async fn list_active(&self) -> Result<Vec<GiftModel>, AppError>;
}

// ─── PgGiftRepo ───────────────────────────────────────────────────────────────

/// 礼物仓储 PostgreSQL 实现
pub struct PgGiftRepo {
    pool: PgPool,
}

impl PgGiftRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GiftRepoPort for PgGiftRepo {
    async fn list_active(&self) -> Result<Vec<GiftModel>, AppError> {
        let gifts = sqlx::query_as::<_, GiftModel>(
            "SELECT id, code, name_en, name_ar, icon_url, price, tier, effect_level, \
             animation_url, sort_order, is_active, is_deleted, created_at, updated_at \
             FROM gifts \
             WHERE is_active = true AND is_deleted = false \
             ORDER BY tier ASC, sort_order ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(gifts)
    }
}

// ─── FakeGiftRepo（仅测试/test-utils）────────────────────────────────────────

/// 内存测试替身，带调用计数器，用于验证缓存命中逻辑（G06）。
///
/// `call_count()` 返回 `list_active` 被实际调用的次数；
/// 若缓存命中，`GiftService` 不调用 repo，计数应保持不变。
#[cfg(any(test, feature = "test-utils"))]
pub struct FakeGiftRepo {
    gifts: Vec<GiftModel>,
    count: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeGiftRepo {
    /// 创建携带预置礼物数据的 FakeGiftRepo
    pub fn new(gifts: Vec<GiftModel>) -> Self {
        Self {
            gifts,
            count: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    /// 返回 `list_active` 被调用的次数
    pub fn call_count(&self) -> u32 {
        self.count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl GiftRepoPort for FakeGiftRepo {
    async fn list_active(&self) -> Result<Vec<GiftModel>, AppError> {
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(self.gifts.clone())
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    fn make_gift(code: &str, tier: i16, sort_order: i32) -> GiftModel {
        GiftModel {
            id: Uuid::new_v4(),
            code: code.to_string(),
            name_en: "Test Gift".to_string(),
            name_ar: "هدية اختبار".to_string(),
            icon_url: "/test.png".to_string(),
            price: 1,
            tier,
            effect_level: 1,
            animation_url: None,
            sort_order,
            is_active: true,
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // R01: FakeGiftRepo 返回预置数据
    #[tokio::test]
    async fn r01_fake_repo_returns_preset_gifts() {
        let gifts = vec![make_gift("rose_01", 1, 10), make_gift("camel_01", 3, 30)];
        let repo = FakeGiftRepo::new(gifts);
        let result = repo.list_active().await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].code, "rose_01");
        assert_eq!(result[1].code, "camel_01");
    }

    // R02: FakeGiftRepo 初始 call_count = 0
    #[test]
    fn r02_fake_repo_initial_call_count_zero() {
        let repo = FakeGiftRepo::new(vec![]);
        assert_eq!(repo.call_count(), 0);
    }

    // R03: FakeGiftRepo 每次调用 list_active 后 call_count +1
    #[tokio::test]
    async fn r03_fake_repo_call_count_increments() {
        let repo = FakeGiftRepo::new(vec![]);
        let _ = repo.list_active().await;
        assert_eq!(repo.call_count(), 1);
        let _ = repo.list_active().await;
        assert_eq!(repo.call_count(), 2);
    }

    // R04: FakeGiftRepo 空列表返回空 Vec
    #[tokio::test]
    async fn r04_fake_repo_empty_list() {
        let repo = FakeGiftRepo::new(vec![]);
        let result = repo.list_active().await.unwrap();
        assert!(result.is_empty());
    }

    // R05: FakeGiftRepo 满足 GiftRepoPort trait（Arc<dyn GiftRepoPort> 可构造）
    #[test]
    fn r05_fake_repo_is_gift_repo_port() {
        let _: Arc<dyn GiftRepoPort> = Arc::new(FakeGiftRepo::new(vec![]));
    }
}
