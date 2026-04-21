use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use voice_room_shared::models::gift::GiftModel;

// ─── 内部数据传输结构 ─────────────────────────────────────────────────────────

/// 创建礼物所需数据（validated）
#[derive(Debug, Clone)]
pub struct CreateGiftData {
    pub code: String,
    pub name_en: String,
    pub name_ar: String,
    pub icon_url: String,
    pub price: i64,
    pub tier: i16,
    pub effect_level: i16,
    pub animation_url: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

/// 更新礼物所需数据（全部可选，None 表示不修改）
#[derive(Debug, Clone, Default)]
pub struct UpdateGiftData {
    pub name_en: Option<String>,
    pub name_ar: Option<String>,
    pub icon_url: Option<String>,
    pub price: Option<i64>,
    pub tier: Option<i16>,
    pub effect_level: Option<i16>,
    pub animation_url: Option<String>,
    pub sort_order: Option<i32>,
    pub is_active: Option<bool>,
}

// ─── Trait ────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait GiftRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<GiftModel>, sqlx::Error>;
    async fn find_by_code(&self, code: &str) -> Result<Option<GiftModel>, sqlx::Error>;
    async fn list(
        &self,
        include_inactive: bool,
        page: i64,
        size: i64,
    ) -> Result<(i64, Vec<GiftModel>), sqlx::Error>;
    async fn create(&self, data: CreateGiftData) -> Result<GiftModel, sqlx::Error>;
    async fn update(&self, id: Uuid, data: UpdateGiftData) -> Result<GiftModel, sqlx::Error>;
    /// 软删除礼物（is_deleted=true）。返回 false 表示礼物不存在或已软删。
    async fn soft_delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

pub struct PgGiftRepository {
    pool: PgPool,
}

impl PgGiftRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GiftRepository for PgGiftRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<GiftModel>, sqlx::Error> {
        sqlx::query_as::<_, GiftModel>(
            "SELECT id, code, name_en, name_ar, icon_url, price, tier, effect_level, \
             animation_url, sort_order, is_active, is_deleted, created_at, updated_at \
             FROM gifts WHERE id = $1 AND is_deleted = false",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<GiftModel>, sqlx::Error> {
        sqlx::query_as::<_, GiftModel>(
            "SELECT id, code, name_en, name_ar, icon_url, price, tier, effect_level, \
             animation_url, sort_order, is_active, is_deleted, created_at, updated_at \
             FROM gifts WHERE code = $1 AND is_deleted = false",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
    }

    async fn list(
        &self,
        include_inactive: bool,
        page: i64,
        size: i64,
    ) -> Result<(i64, Vec<GiftModel>), sqlx::Error> {
        let count: (i64,) = if include_inactive {
            sqlx::query_as(
                "SELECT COUNT(*) FROM gifts WHERE is_deleted = false",
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                "SELECT COUNT(*) FROM gifts WHERE is_deleted = false AND is_active = true",
            )
            .fetch_one(&self.pool)
            .await?
        };

        let offset = (page - 1) * size;
        let rows: Vec<GiftModel> = if include_inactive {
            sqlx::query_as::<_, GiftModel>(
                "SELECT id, code, name_en, name_ar, icon_url, price, tier, effect_level, \
                 animation_url, sort_order, is_active, is_deleted, created_at, updated_at \
                 FROM gifts WHERE is_deleted = false \
                 ORDER BY tier ASC, sort_order ASC \
                 LIMIT $1 OFFSET $2",
            )
            .bind(size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, GiftModel>(
                "SELECT id, code, name_en, name_ar, icon_url, price, tier, effect_level, \
                 animation_url, sort_order, is_active, is_deleted, created_at, updated_at \
                 FROM gifts WHERE is_deleted = false AND is_active = true \
                 ORDER BY tier ASC, sort_order ASC \
                 LIMIT $1 OFFSET $2",
            )
            .bind(size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok((count.0, rows))
    }

    async fn create(&self, data: CreateGiftData) -> Result<GiftModel, sqlx::Error> {
        sqlx::query_as::<_, GiftModel>(
            "INSERT INTO gifts (code, name_en, name_ar, icon_url, price, tier, effect_level, \
             animation_url, sort_order, is_active) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             RETURNING id, code, name_en, name_ar, icon_url, price, tier, effect_level, \
             animation_url, sort_order, is_active, is_deleted, created_at, updated_at",
        )
        .bind(data.code)
        .bind(data.name_en)
        .bind(data.name_ar)
        .bind(data.icon_url)
        .bind(data.price)
        .bind(data.tier)
        .bind(data.effect_level)
        .bind(data.animation_url)
        .bind(data.sort_order)
        .bind(data.is_active)
        .fetch_one(&self.pool)
        .await
    }

    async fn update(&self, id: Uuid, data: UpdateGiftData) -> Result<GiftModel, sqlx::Error> {
        sqlx::query_as::<_, GiftModel>(
            "UPDATE gifts SET \
             name_en = COALESCE($2, name_en), \
             name_ar = COALESCE($3, name_ar), \
             icon_url = COALESCE($4, icon_url), \
             price = COALESCE($5, price), \
             tier = COALESCE($6, tier), \
             effect_level = COALESCE($7, effect_level), \
             animation_url = COALESCE($8, animation_url), \
             sort_order = COALESCE($9, sort_order), \
             is_active = COALESCE($10, is_active), \
             updated_at = now() \
             WHERE id = $1 AND is_deleted = false \
             RETURNING id, code, name_en, name_ar, icon_url, price, tier, effect_level, \
             animation_url, sort_order, is_active, is_deleted, created_at, updated_at",
        )
        .bind(id)
        .bind(data.name_en)
        .bind(data.name_ar)
        .bind(data.icon_url)
        .bind(data.price)
        .bind(data.tier)
        .bind(data.effect_level)
        .bind(data.animation_url)
        .bind(data.sort_order)
        .bind(data.is_active)
        .fetch_one(&self.pool)
        .await
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let rows_affected = sqlx::query(
            "UPDATE gifts SET is_deleted = true, updated_at = now() \
             WHERE id = $1 AND is_deleted = false",
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(rows_affected > 0)
    }
}

// ─── Fake 实现（内存，用于单元/集成测试）──────────────────────────────────────

/// 测试专用：内存 GiftRepository。
pub struct FakeGiftRepository {
    gifts: Arc<Mutex<Vec<GiftModel>>>,
}

impl Default for FakeGiftRepository {
    fn default() -> Self {
        Self {
            gifts: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl FakeGiftRepository {
    /// 预置礼物数据
    pub fn seed(&self, gift: GiftModel) {
        self.gifts.lock().unwrap().push(gift);
    }

    /// 读取所有礼物（含软删）
    pub fn get_all(&self) -> Vec<GiftModel> {
        self.gifts.lock().unwrap().clone()
    }
}

#[async_trait]
impl GiftRepository for FakeGiftRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<GiftModel>, sqlx::Error> {
        let gifts = self.gifts.lock().unwrap();
        Ok(gifts.iter().find(|g| g.id == id && !g.is_deleted).cloned())
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<GiftModel>, sqlx::Error> {
        let gifts = self.gifts.lock().unwrap();
        Ok(gifts.iter().find(|g| g.code == code && !g.is_deleted).cloned())
    }

    async fn list(
        &self,
        include_inactive: bool,
        page: i64,
        size: i64,
    ) -> Result<(i64, Vec<GiftModel>), sqlx::Error> {
        let gifts = self.gifts.lock().unwrap();
        let filtered: Vec<GiftModel> = gifts
            .iter()
            .filter(|g| {
                if g.is_deleted {
                    return false;
                }
                if !include_inactive && !g.is_active {
                    return false;
                }
                true
            })
            .cloned()
            .collect();

        let total = filtered.len() as i64;
        let offset = ((page - 1) * size).max(0) as usize;
        let items: Vec<GiftModel> = filtered
            .into_iter()
            .skip(offset)
            .take(size as usize)
            .collect();

        Ok((total, items))
    }

    async fn create(&self, data: CreateGiftData) -> Result<GiftModel, sqlx::Error> {
        let now = Utc::now();
        let gift = GiftModel {
            id: Uuid::new_v4(),
            code: data.code,
            name_en: data.name_en,
            name_ar: data.name_ar,
            icon_url: data.icon_url,
            price: data.price,
            tier: data.tier,
            effect_level: data.effect_level,
            animation_url: data.animation_url,
            sort_order: data.sort_order,
            is_active: data.is_active,
            is_deleted: false,
            created_at: now,
            updated_at: now,
        };
        self.gifts.lock().unwrap().push(gift.clone());
        Ok(gift)
    }

    async fn update(&self, id: Uuid, data: UpdateGiftData) -> Result<GiftModel, sqlx::Error> {
        let mut gifts = self.gifts.lock().unwrap();
        let gift = gifts
            .iter_mut()
            .find(|g| g.id == id && !g.is_deleted)
            .ok_or(sqlx::Error::RowNotFound)?;

        if let Some(v) = data.name_en {
            gift.name_en = v;
        }
        if let Some(v) = data.name_ar {
            gift.name_ar = v;
        }
        if let Some(v) = data.icon_url {
            gift.icon_url = v;
        }
        if let Some(v) = data.price {
            gift.price = v;
        }
        if let Some(v) = data.tier {
            gift.tier = v;
        }
        if let Some(v) = data.effect_level {
            gift.effect_level = v;
        }
        if let Some(v) = data.animation_url {
            gift.animation_url = Some(v);
        }
        if let Some(v) = data.sort_order {
            gift.sort_order = v;
        }
        if let Some(v) = data.is_active {
            gift.is_active = v;
        }
        gift.updated_at = Utc::now();

        Ok(gift.clone())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let mut gifts = self.gifts.lock().unwrap();
        if let Some(gift) = gifts.iter_mut().find(|g| g.id == id && !g.is_deleted) {
            gift.is_deleted = true;
            gift.updated_at = Utc::now();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// ─── 单元测试（Repo 层）──────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_gift(code: &str, is_active: bool, is_deleted: bool) -> GiftModel {
        GiftModel {
            id: Uuid::new_v4(),
            code: code.to_string(),
            name_en: "Test Gift".to_string(),
            name_ar: "هدية".to_string(),
            icon_url: "/uploads/gifts/test.png".to_string(),
            price: 10,
            tier: 1,
            effect_level: 1,
            animation_url: None,
            sort_order: 10,
            is_active,
            is_deleted,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_create_data(code: &str) -> CreateGiftData {
        CreateGiftData {
            code: code.to_string(),
            name_en: "New Gift".to_string(),
            name_ar: "هدية جديدة".to_string(),
            icon_url: "/uploads/gifts/new.png".to_string(),
            price: 66,
            tier: 3,
            effect_level: 2,
            animation_url: None,
            sort_order: 30,
            is_active: true,
        }
    }

    /// GR-01: create 后 find_by_id 能找到
    #[tokio::test]
    async fn gr01_create_and_find_by_id() {
        let repo = FakeGiftRepository::default();
        let gift = repo.create(make_create_data("unicorn_01")).await.unwrap();
        let found = repo.find_by_id(gift.id).await.unwrap();
        assert!(found.is_some(), "GR-01: find_by_id 应能找到刚创建的礼物");
        assert_eq!(found.unwrap().code, "unicorn_01");
    }

    /// GR-02: find_by_code 正确
    #[tokio::test]
    async fn gr02_find_by_code() {
        let repo = FakeGiftRepository::default();
        repo.create(make_create_data("rose_02")).await.unwrap();
        let found = repo.find_by_code("rose_02").await.unwrap();
        assert!(found.is_some(), "GR-02: find_by_code 应能找到");
        assert_eq!(found.unwrap().code, "rose_02");
    }

    /// GR-03: list 默认不含 inactive
    #[tokio::test]
    async fn gr03_list_excludes_inactive_by_default() {
        let repo = FakeGiftRepository::default();
        repo.seed(make_gift("active_01", true, false));
        repo.seed(make_gift("inactive_01", false, false));
        repo.seed(make_gift("deleted_01", true, true));

        let (total, items) = repo.list(false, 1, 20).await.unwrap();
        assert_eq!(total, 1, "GR-03: 默认只返回 active+non-deleted");
        assert_eq!(items[0].code, "active_01");
    }

    /// GR-04: list include_inactive=true 返回所有非软删
    #[tokio::test]
    async fn gr04_list_include_inactive() {
        let repo = FakeGiftRepository::default();
        repo.seed(make_gift("active_01", true, false));
        repo.seed(make_gift("inactive_01", false, false));
        repo.seed(make_gift("deleted_01", true, true));

        let (total, items) = repo.list(true, 1, 20).await.unwrap();
        assert_eq!(total, 2, "GR-04: include_inactive 应返回 active+inactive（不含软删）");
        assert!(items.iter().all(|g| !g.is_deleted));
    }

    /// GR-05: soft_delete 标记 is_deleted=true，再次调用返回 false
    #[tokio::test]
    async fn gr05_soft_delete() {
        let repo = FakeGiftRepository::default();
        let gift = repo.create(make_create_data("delete_test")).await.unwrap();
        let id = gift.id;

        let ok = repo.soft_delete(id).await.unwrap();
        assert!(ok, "GR-05: 首次软删应返回 true");

        // 再次软删应返回 false
        let ok2 = repo.soft_delete(id).await.unwrap();
        assert!(!ok2, "GR-05: 重复软删应返回 false");

        // find_by_id 应找不到（因为已软删）
        let found = repo.find_by_id(id).await.unwrap();
        assert!(found.is_none(), "GR-05: 软删后 find_by_id 应返回 None");
    }

    /// GR-06: update 正确更新字段
    #[tokio::test]
    async fn gr06_update_fields() {
        let repo = FakeGiftRepository::default();
        let gift = repo.create(make_create_data("update_test")).await.unwrap();

        let updated = repo
            .update(
                gift.id,
                UpdateGiftData {
                    is_active: Some(false),
                    price: Some(99),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert!(!updated.is_active, "GR-06: is_active 应被更新为 false");
        assert_eq!(updated.price, 99, "GR-06: price 应被更新为 99");
        assert_eq!(updated.code, "update_test", "GR-06: code 不应被改变");
    }
}
