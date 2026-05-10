// PROTO-BINDING: doc/protocol/nobility_api.md §10.2 数据模型
//! NobilityRepo trait + FakeNobilityRepo（内存）+ PgNobilityRepo（生产）

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::error::AppError;

use super::dto::{NobleHistoryItem, TierResponse, UserNobleItem, UserNobleResponse};

// ─── Data rows ──────────────────────────────────────────────────────────────

/// DB row for noble_tiers
#[derive(Debug, Clone)]
pub struct NobleTierRow {
    pub tier_id: String,
    pub name_en: String,
    pub name_ar: String,
    pub level: i16,
    pub monthly_diamonds: i64,
    pub monthly_usd: String,
    pub usd_sku_id: Option<String>,
    pub privileges: Value,
    pub icon_url: String,
    pub frame_url: String,
    pub entrance_animation_url: Option<String>,
    pub bgm_url: Option<String>,
    pub badge_color: String,
    pub bubble_style_id: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<NobleTierRow> for TierResponse {
    fn from(r: NobleTierRow) -> Self {
        TierResponse {
            tier_id: r.tier_id,
            name_en: r.name_en,
            name_ar: r.name_ar,
            level: r.level,
            monthly_diamonds: r.monthly_diamonds,
            monthly_usd: r.monthly_usd,
            usd_sku_id: r.usd_sku_id,
            privileges: r.privileges,
            icon_url: r.icon_url,
            frame_url: r.frame_url,
            entrance_animation_url: r.entrance_animation_url,
            bgm_url: r.bgm_url,
            badge_color: r.badge_color,
            bubble_style_id: r.bubble_style_id,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// DB row for user_nobles
#[derive(Debug, Clone)]
pub struct UserNobleRow {
    pub user_id: Uuid,
    pub tier_id: String,
    pub start_at: DateTime<Utc>,
    pub current_period_start: DateTime<Utc>,
    pub expire_at: DateTime<Utc>,
    pub auto_renew: bool,
    pub renew_channel: String,
    pub total_paid_diamonds: i64,
    pub total_paid_usd_micros: i64,
}

impl From<UserNobleRow> for UserNobleResponse {
    fn from(r: UserNobleRow) -> Self {
        UserNobleResponse {
            user_id: r.user_id,
            tier_id: r.tier_id,
            start_at: r.start_at,
            current_period_start: r.current_period_start,
            expire_at: r.expire_at,
            auto_renew: r.auto_renew,
            renew_channel: r.renew_channel,
            total_paid_diamonds: r.total_paid_diamonds,
            total_paid_usd_micros: r.total_paid_usd_micros,
        }
    }
}

// ─── Create/Update data structs ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CreateTierData {
    pub tier_id: String,
    pub name_en: String,
    pub name_ar: String,
    pub level: i16,
    pub monthly_diamonds: i64,
    pub monthly_usd: String,
    pub usd_sku_id: Option<String>,
    pub privileges: Value,
    pub icon_url: String,
    pub frame_url: String,
    pub entrance_animation_url: Option<String>,
    pub bgm_url: Option<String>,
    pub badge_color: String,
    pub bubble_style_id: String,
}

#[derive(Debug, Clone)]
pub struct UpdateTierData {
    pub name_en: Option<String>,
    pub name_ar: Option<String>,
    pub monthly_diamonds: Option<i64>,
    pub monthly_usd: Option<String>,
    pub usd_sku_id: Option<String>,
    pub privileges: Option<Value>,
    pub icon_url: Option<String>,
    pub frame_url: Option<String>,
    pub entrance_animation_url: Option<String>,
    pub bgm_url: Option<String>,
    pub badge_color: Option<String>,
    pub bubble_style_id: Option<String>,
}

/// 手动赠送参数
#[derive(Debug, Clone)]
pub struct GrantData {
    pub user_id: Uuid,
    pub tier_id: String,
    pub duration_days: i32,
}

/// 用户贵族列表过滤器
#[derive(Debug, Default, Clone)]
pub struct UserNobleFilter {
    pub tier_id: Option<String>,
    /// true → expire_at > now; false → expire_at <= now; None → all
    pub active_only: Option<bool>,
    pub expire_before: Option<DateTime<Utc>>,
}

// ─── Trait ──────────────────────────────────────────────────────────────────

#[async_trait]
pub trait NobilityRepo: Send + Sync {
    // ─── Tier CRUD ──────────────────────────────────────────────────────────
    async fn list_tiers(
        &self,
        page: i64,
        size: i64,
    ) -> Result<(i64, Vec<NobleTierRow>), AppError>;
    async fn get_tier(&self, tier_id: &str) -> Result<Option<NobleTierRow>, AppError>;
    /// 检查 level 是否已被其他 tier 占用
    async fn level_exists(&self, level: i16, exclude_tier_id: Option<&str>)
        -> Result<bool, AppError>;
    async fn create_tier(&self, data: CreateTierData) -> Result<NobleTierRow, AppError>;
    async fn update_tier(
        &self,
        tier_id: &str,
        data: UpdateTierData,
    ) -> Result<NobleTierRow, AppError>;
    /// 软删（is_active → false）
    async fn soft_delete_tier(&self, tier_id: &str) -> Result<NobleTierRow, AppError>;

    // ─── User noble grant/revoke ─────────────────────────────────────────────
    async fn get_user_noble(&self, user_id: Uuid) -> Result<Option<UserNobleRow>, AppError>;
    async fn upsert_user_noble_grant(
        &self,
        data: GrantData,
    ) -> Result<UserNobleRow, AppError>;
    async fn revoke_user_noble(&self, user_id: Uuid) -> Result<UserNobleRow, AppError>;
    async fn insert_noble_history(
        &self,
        user_id: Uuid,
        event: &str,
        from_tier: Option<&str>,
        to_tier: Option<&str>,
        actor: &str,
        payload: Value,
    ) -> Result<(), AppError>;

    // ─── User query ──────────────────────────────────────────────────────────
    async fn list_noble_users(
        &self,
        filter: &UserNobleFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserNobleItem>, AppError>;
    async fn count_noble_users(&self, filter: &UserNobleFilter) -> Result<i64, AppError>;
    async fn get_noble_history(&self, user_id: Uuid) -> Result<Vec<NobleHistoryItem>, AppError>;
}

// ─── FakeNobilityRepo (for tests) ────────────────────────────────────────────

#[derive(Debug)]
struct FakeTierEntry {
    row: NobleTierRow,
}

#[derive(Debug)]
struct FakeUserNoble {
    row: UserNobleRow,
    nickname: String,
    avatar_url: Option<String>,
    tier_name_en: String,
    tier_name_ar: String,
    tier_level: i16,
    badge_color: String,
}

pub struct FakeNobilityRepo {
    tiers: Arc<Mutex<Vec<FakeTierEntry>>>,
    user_nobles: Arc<Mutex<Vec<FakeUserNoble>>>,
    history: Arc<Mutex<Vec<NobleHistoryItem>>>,
    /// Call log: (method_name)
    pub calls: Arc<Mutex<Vec<String>>>,
}

impl Default for FakeNobilityRepo {
    fn default() -> Self {
        Self {
            tiers: Arc::new(Mutex::new(vec![])),
            user_nobles: Arc::new(Mutex::new(vec![])),
            history: Arc::new(Mutex::new(vec![])),
            calls: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl FakeNobilityRepo {
    fn record(&self, method: &str) {
        self.calls.lock().unwrap().push(method.to_string());
    }

    /// Seed a tier row (for list/get tests)
    pub fn push_tier(&self, row: NobleTierRow) {
        self.tiers.lock().unwrap().push(FakeTierEntry { row });
    }

    /// Seed a user noble (for query tests)
    pub fn push_user_noble(
        &self,
        row: UserNobleRow,
        nickname: impl Into<String>,
        avatar_url: Option<String>,
        tier_name_en: impl Into<String>,
        tier_name_ar: impl Into<String>,
        tier_level: i16,
        badge_color: impl Into<String>,
    ) {
        self.user_nobles.lock().unwrap().push(FakeUserNoble {
            row,
            nickname: nickname.into(),
            avatar_url,
            tier_name_en: tier_name_en.into(),
            tier_name_ar: tier_name_ar.into(),
            tier_level,
            badge_color: badge_color.into(),
        });
    }

    pub fn get_history(&self) -> Vec<NobleHistoryItem> {
        self.history.lock().unwrap().clone()
    }
}

#[async_trait]
impl NobilityRepo for FakeNobilityRepo {
    async fn list_tiers(
        &self,
        page: i64,
        size: i64,
    ) -> Result<(i64, Vec<NobleTierRow>), AppError> {
        self.record("list_tiers");
        let guard = self.tiers.lock().unwrap();
        let active: Vec<_> = guard.iter().filter(|e| e.row.is_active).collect();
        let total = active.len() as i64;
        let offset = ((page - 1) * size) as usize;
        let items: Vec<NobleTierRow> = active
            .into_iter()
            .skip(offset)
            .take(size as usize)
            .map(|e| e.row.clone())
            .collect();
        Ok((total, items))
    }

    async fn get_tier(&self, tier_id: &str) -> Result<Option<NobleTierRow>, AppError> {
        self.record("get_tier");
        let guard = self.tiers.lock().unwrap();
        Ok(guard
            .iter()
            .find(|e| e.row.tier_id == tier_id)
            .map(|e| e.row.clone()))
    }

    async fn level_exists(
        &self,
        level: i16,
        exclude_tier_id: Option<&str>,
    ) -> Result<bool, AppError> {
        self.record("level_exists");
        let guard = self.tiers.lock().unwrap();
        Ok(guard.iter().any(|e| {
            e.row.level == level
                && e.row.is_active
                && exclude_tier_id.map(|id| id != e.row.tier_id).unwrap_or(true)
        }))
    }

    async fn create_tier(&self, data: CreateTierData) -> Result<NobleTierRow, AppError> {
        self.record("create_tier");
        let now = Utc::now();
        let row = NobleTierRow {
            tier_id: data.tier_id,
            name_en: data.name_en,
            name_ar: data.name_ar,
            level: data.level,
            monthly_diamonds: data.monthly_diamonds,
            monthly_usd: data.monthly_usd,
            usd_sku_id: data.usd_sku_id,
            privileges: data.privileges,
            icon_url: data.icon_url,
            frame_url: data.frame_url,
            entrance_animation_url: data.entrance_animation_url,
            bgm_url: data.bgm_url,
            badge_color: data.badge_color,
            bubble_style_id: data.bubble_style_id,
            is_active: true,
            created_at: now,
            updated_at: now,
        };
        self.tiers
            .lock()
            .unwrap()
            .push(FakeTierEntry { row: row.clone() });
        Ok(row)
    }

    async fn update_tier(
        &self,
        tier_id: &str,
        data: UpdateTierData,
    ) -> Result<NobleTierRow, AppError> {
        self.record("update_tier");
        let mut guard = self.tiers.lock().unwrap();
        let entry = guard
            .iter_mut()
            .find(|e| e.row.tier_id == tier_id)
            .ok_or_else(|| AppError::NotFound(format!("tier '{tier_id}'")))?;

        let row = &mut entry.row;
        if let Some(v) = data.name_en {
            row.name_en = v;
        }
        if let Some(v) = data.name_ar {
            row.name_ar = v;
        }
        if let Some(v) = data.monthly_diamonds {
            row.monthly_diamonds = v;
        }
        if let Some(v) = data.monthly_usd {
            row.monthly_usd = v;
        }
        if let Some(v) = data.usd_sku_id {
            row.usd_sku_id = Some(v);
        }
        if let Some(v) = data.privileges {
            row.privileges = v;
        }
        if let Some(v) = data.icon_url {
            row.icon_url = v;
        }
        if let Some(v) = data.frame_url {
            row.frame_url = v;
        }
        if let Some(v) = data.entrance_animation_url {
            row.entrance_animation_url = Some(v);
        }
        if let Some(v) = data.bgm_url {
            row.bgm_url = Some(v);
        }
        if let Some(v) = data.badge_color {
            row.badge_color = v;
        }
        if let Some(v) = data.bubble_style_id {
            row.bubble_style_id = v;
        }
        row.updated_at = Utc::now();
        Ok(row.clone())
    }

    async fn soft_delete_tier(&self, tier_id: &str) -> Result<NobleTierRow, AppError> {
        self.record("soft_delete_tier");
        let mut guard = self.tiers.lock().unwrap();
        let entry = guard
            .iter_mut()
            .find(|e| e.row.tier_id == tier_id)
            .ok_or_else(|| AppError::NotFound(format!("tier '{tier_id}'")))?;
        entry.row.is_active = false;
        entry.row.updated_at = Utc::now();
        Ok(entry.row.clone())
    }

    async fn get_user_noble(&self, user_id: Uuid) -> Result<Option<UserNobleRow>, AppError> {
        self.record("get_user_noble");
        let guard = self.user_nobles.lock().unwrap();
        Ok(guard
            .iter()
            .find(|un| un.row.user_id == user_id)
            .map(|un| un.row.clone()))
    }

    async fn upsert_user_noble_grant(
        &self,
        data: GrantData,
    ) -> Result<UserNobleRow, AppError> {
        self.record("upsert_user_noble_grant");
        let now = Utc::now();
        let duration = chrono::Duration::days(data.duration_days as i64);
        let mut guard = self.user_nobles.lock().unwrap();

        if let Some(existing) = guard.iter_mut().find(|un| un.row.user_id == data.user_id) {
            // Merge expire_at: MAX(expire_at, now) + duration
            let base = if existing.row.expire_at > now {
                existing.row.expire_at
            } else {
                now
            };
            existing.row.expire_at = base + duration;
            existing.row.tier_id = data.tier_id;
            existing.row.renew_channel = "admin_grant".to_string();
            existing.row.auto_renew = false;
            Ok(existing.row.clone())
        } else {
            let expire_at = now + duration;
            let row = UserNobleRow {
                user_id: data.user_id,
                tier_id: data.tier_id,
                start_at: now,
                current_period_start: now,
                expire_at,
                auto_renew: false,
                renew_channel: "admin_grant".to_string(),
                total_paid_diamonds: 0,
                total_paid_usd_micros: 0,
            };
            guard.push(FakeUserNoble {
                row: row.clone(),
                nickname: "".to_string(),
                avatar_url: None,
                tier_name_en: "".to_string(),
                tier_name_ar: "".to_string(),
                tier_level: 1,
                badge_color: "#000000".to_string(),
            });
            Ok(row)
        }
    }

    async fn revoke_user_noble(&self, user_id: Uuid) -> Result<UserNobleRow, AppError> {
        self.record("revoke_user_noble");
        let mut guard = self.user_nobles.lock().unwrap();
        let pos = guard
            .iter()
            .position(|un| un.row.user_id == user_id)
            .ok_or_else(|| AppError::NotFound(format!("user noble for '{user_id}'")))?;
        let removed = guard.remove(pos);
        Ok(removed.row)
    }

    async fn insert_noble_history(
        &self,
        user_id: Uuid,
        event: &str,
        from_tier: Option<&str>,
        to_tier: Option<&str>,
        actor: &str,
        payload: Value,
    ) -> Result<(), AppError> {
        self.record("insert_noble_history");
        let id = {
            let guard = self.history.lock().unwrap();
            guard.len() as i64 + 1
        };
        let item = NobleHistoryItem {
            id,
            user_id,
            event: event.to_string(),
            from_tier: from_tier.map(|s| s.to_string()),
            to_tier: to_tier.map(|s| s.to_string()),
            actor: actor.to_string(),
            payload,
            created_at: Utc::now(),
        };
        self.history.lock().unwrap().push(item);
        Ok(())
    }

    async fn list_noble_users(
        &self,
        filter: &UserNobleFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserNobleItem>, AppError> {
        self.record("list_noble_users");
        let now = Utc::now();
        let guard = self.user_nobles.lock().unwrap();
        let items: Vec<UserNobleItem> = guard
            .iter()
            .filter(|un| {
                if let Some(ref tid) = filter.tier_id {
                    if &un.row.tier_id != tid {
                        return false;
                    }
                }
                match filter.active_only {
                    Some(true) => {
                        if un.row.expire_at <= now {
                            return false;
                        }
                    }
                    Some(false) => {
                        if un.row.expire_at > now {
                            return false;
                        }
                    }
                    None => {}
                }
                if let Some(before) = filter.expire_before {
                    if un.row.expire_at >= before {
                        return false;
                    }
                }
                true
            })
            .map(|un| UserNobleItem {
                user_id: un.row.user_id,
                nickname: un.nickname.clone(),
                avatar_url: un.avatar_url.clone(),
                tier_id: un.row.tier_id.clone(),
                tier_name_en: un.tier_name_en.clone(),
                tier_name_ar: un.tier_name_ar.clone(),
                tier_level: un.tier_level,
                badge_color: un.badge_color.clone(),
                start_at: un.row.start_at,
                current_period_start: un.row.current_period_start,
                expire_at: un.row.expire_at,
                auto_renew: un.row.auto_renew,
                renew_channel: un.row.renew_channel.clone(),
                total_paid_diamonds: un.row.total_paid_diamonds,
                total_paid_usd_micros: un.row.total_paid_usd_micros,
            })
            .skip(offset as usize)
            .take(limit as usize)
            .collect();
        Ok(items)
    }

    async fn count_noble_users(&self, filter: &UserNobleFilter) -> Result<i64, AppError> {
        self.record("count_noble_users");
        let now = Utc::now();
        let guard = self.user_nobles.lock().unwrap();
        let count = guard
            .iter()
            .filter(|un| {
                if let Some(ref tid) = filter.tier_id {
                    if &un.row.tier_id != tid {
                        return false;
                    }
                }
                match filter.active_only {
                    Some(true) => {
                        if un.row.expire_at <= now {
                            return false;
                        }
                    }
                    Some(false) => {
                        if un.row.expire_at > now {
                            return false;
                        }
                    }
                    None => {}
                }
                if let Some(before) = filter.expire_before {
                    if un.row.expire_at >= before {
                        return false;
                    }
                }
                true
            })
            .count() as i64;
        Ok(count)
    }

    async fn get_noble_history(&self, user_id: Uuid) -> Result<Vec<NobleHistoryItem>, AppError> {
        self.record("get_noble_history");
        let guard = self.history.lock().unwrap();
        let mut items: Vec<NobleHistoryItem> = guard
            .iter()
            .filter(|h| h.user_id == user_id)
            .cloned()
            .collect();
        // ORDER BY created_at DESC
        items.sort_by_key(|h| std::cmp::Reverse(h.created_at));
        Ok(items.into_iter().take(100).collect())
    }
}

// ─── PostgreSQL 生产实现（占位）──────────────────────────────────────────────

/// PgNobilityRepo — SQLx + PostgreSQL 生产实现。
/// 所有方法均调用生产 DB；单元测试使用 FakeNobilityRepo。
pub struct PgNobilityRepo {
    pub pool: PgPool,
}

impl PgNobilityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NobilityRepo for PgNobilityRepo {
    async fn list_tiers(
        &self,
        page: i64,
        size: i64,
    ) -> Result<(i64, Vec<NobleTierRow>), AppError> {
        let offset = (page - 1) * size;
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM noble_tiers WHERE is_active = true",
        )
        .fetch_one(&self.pool)
        .await?;

        let rows: Vec<PgTierRow> = sqlx::query_as(
            "SELECT tier_id, name_en, name_ar, level, monthly_diamonds, \
             monthly_usd::text, usd_sku_id, privileges, icon_url, frame_url, \
             entrance_animation_url, bgm_url, badge_color, bubble_style_id, \
             is_active, created_at, updated_at \
             FROM noble_tiers WHERE is_active = true \
             ORDER BY level ASC LIMIT $1 OFFSET $2",
        )
        .bind(size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((count, rows.into_iter().map(Into::into).collect()))
    }

    async fn get_tier(&self, tier_id: &str) -> Result<Option<NobleTierRow>, AppError> {
        let row: Option<PgTierRow> = sqlx::query_as(
            "SELECT tier_id, name_en, name_ar, level, monthly_diamonds, \
             monthly_usd::text, usd_sku_id, privileges, icon_url, frame_url, \
             entrance_animation_url, bgm_url, badge_color, bubble_style_id, \
             is_active, created_at, updated_at \
             FROM noble_tiers WHERE tier_id = $1",
        )
        .bind(tier_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn level_exists(
        &self,
        level: i16,
        exclude_tier_id: Option<&str>,
    ) -> Result<bool, AppError> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM noble_tiers \
             WHERE level = $1 AND is_active = true \
               AND ($2::text IS NULL OR tier_id != $2)",
        )
        .bind(level)
        .bind(exclude_tier_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    async fn create_tier(&self, data: CreateTierData) -> Result<NobleTierRow, AppError> {
        let row: PgTierRow = sqlx::query_as(
            "INSERT INTO noble_tiers (tier_id, name_en, name_ar, level, monthly_diamonds, \
             monthly_usd, usd_sku_id, privileges, icon_url, frame_url, \
             entrance_animation_url, bgm_url, badge_color, bubble_style_id) \
             VALUES ($1,$2,$3,$4,$5,$6::numeric,$7,$8,$9,$10,$11,$12,$13,$14) \
             RETURNING tier_id, name_en, name_ar, level, monthly_diamonds, \
             monthly_usd::text, usd_sku_id, privileges, icon_url, frame_url, \
             entrance_animation_url, bgm_url, badge_color, bubble_style_id, \
             is_active, created_at, updated_at",
        )
        .bind(&data.tier_id)
        .bind(&data.name_en)
        .bind(&data.name_ar)
        .bind(data.level)
        .bind(data.monthly_diamonds)
        .bind(&data.monthly_usd)
        .bind(data.usd_sku_id.as_deref())
        .bind(&data.privileges)
        .bind(&data.icon_url)
        .bind(&data.frame_url)
        .bind(data.entrance_animation_url.as_deref())
        .bind(data.bgm_url.as_deref())
        .bind(&data.badge_color)
        .bind(&data.bubble_style_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn update_tier(
        &self,
        tier_id: &str,
        data: UpdateTierData,
    ) -> Result<NobleTierRow, AppError> {
        // Build dynamic UPDATE using COALESCE
        let row: PgTierRow = sqlx::query_as(
            "UPDATE noble_tiers SET \
             name_en = COALESCE($2, name_en), \
             name_ar = COALESCE($3, name_ar), \
             monthly_diamonds = COALESCE($4, monthly_diamonds), \
             monthly_usd = COALESCE($5::numeric, monthly_usd), \
             usd_sku_id = COALESCE($6, usd_sku_id), \
             privileges = COALESCE($7, privileges), \
             icon_url = COALESCE($8, icon_url), \
             frame_url = COALESCE($9, frame_url), \
             entrance_animation_url = COALESCE($10, entrance_animation_url), \
             bgm_url = COALESCE($11, bgm_url), \
             badge_color = COALESCE($12, badge_color), \
             bubble_style_id = COALESCE($13, bubble_style_id), \
             updated_at = now() \
             WHERE tier_id = $1 \
             RETURNING tier_id, name_en, name_ar, level, monthly_diamonds, \
             monthly_usd::text, usd_sku_id, privileges, icon_url, frame_url, \
             entrance_animation_url, bgm_url, badge_color, bubble_style_id, \
             is_active, created_at, updated_at",
        )
        .bind(tier_id)
        .bind(data.name_en.as_deref())
        .bind(data.name_ar.as_deref())
        .bind(data.monthly_diamonds)
        .bind(data.monthly_usd.as_deref())
        .bind(data.usd_sku_id.as_deref())
        .bind(data.privileges.as_ref())
        .bind(data.icon_url.as_deref())
        .bind(data.frame_url.as_deref())
        .bind(data.entrance_animation_url.as_deref())
        .bind(data.bgm_url.as_deref())
        .bind(data.badge_color.as_deref())
        .bind(data.bubble_style_id.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("tier '{tier_id}'")),
            other => AppError::from(other),
        })?;
        Ok(row.into())
    }

    async fn soft_delete_tier(&self, tier_id: &str) -> Result<NobleTierRow, AppError> {
        let row: PgTierRow = sqlx::query_as(
            "UPDATE noble_tiers SET is_active = false, updated_at = now() \
             WHERE tier_id = $1 \
             RETURNING tier_id, name_en, name_ar, level, monthly_diamonds, \
             monthly_usd::text, usd_sku_id, privileges, icon_url, frame_url, \
             entrance_animation_url, bgm_url, badge_color, bubble_style_id, \
             is_active, created_at, updated_at",
        )
        .bind(tier_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("tier '{tier_id}'")),
            other => AppError::from(other),
        })?;
        Ok(row.into())
    }

    async fn get_user_noble(&self, user_id: Uuid) -> Result<Option<UserNobleRow>, AppError> {
        let row: Option<PgUserNobleRow> = sqlx::query_as(
            "SELECT user_id, tier_id, start_at, current_period_start, expire_at, \
             auto_renew, renew_channel, total_paid_diamonds, total_paid_usd_micros \
             FROM user_nobles WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn upsert_user_noble_grant(
        &self,
        data: GrantData,
    ) -> Result<UserNobleRow, AppError> {
        let row: PgUserNobleRow = sqlx::query_as(
            "INSERT INTO user_nobles \
             (user_id, tier_id, start_at, current_period_start, expire_at, \
              auto_renew, renew_channel) \
             VALUES ($1, $2, now(), now(), now() + $3 * interval '1 day', false, 'admin_grant') \
             ON CONFLICT (user_id) DO UPDATE SET \
               tier_id = EXCLUDED.tier_id, \
               expire_at = GREATEST(user_nobles.expire_at, now()) + $3 * interval '1 day', \
               renew_channel = 'admin_grant', \
               auto_renew = false, \
               updated_at = now() \
             RETURNING user_id, tier_id, start_at, current_period_start, expire_at, \
             auto_renew, renew_channel, total_paid_diamonds, total_paid_usd_micros",
        )
        .bind(data.user_id)
        .bind(&data.tier_id)
        .bind(data.duration_days)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn revoke_user_noble(&self, user_id: Uuid) -> Result<UserNobleRow, AppError> {
        // Get row before deletion
        let existing = self
            .get_user_noble(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("user noble for '{user_id}'")))?;
        sqlx::query("DELETE FROM user_nobles WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(existing)
    }

    async fn insert_noble_history(
        &self,
        user_id: Uuid,
        event: &str,
        from_tier: Option<&str>,
        to_tier: Option<&str>,
        actor: &str,
        payload: Value,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO noble_history (user_id, event, from_tier, to_tier, actor, payload) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(user_id)
        .bind(event)
        .bind(from_tier)
        .bind(to_tier)
        .bind(actor)
        .bind(&payload)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_noble_users(
        &self,
        filter: &UserNobleFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserNobleItem>, AppError> {
        // Dynamic WHERE using nullable params
        let rows: Vec<PgUserNobleItemRow> = sqlx::query_as(
            "SELECT un.user_id, u.nickname, u.avatar_url, \
             un.tier_id, nt.name_en as tier_name_en, nt.name_ar as tier_name_ar, \
             nt.level as tier_level, nt.badge_color, \
             un.start_at, un.current_period_start, un.expire_at, \
             un.auto_renew, un.renew_channel, \
             un.total_paid_diamonds, un.total_paid_usd_micros \
             FROM user_nobles un \
             JOIN users u ON u.id = un.user_id \
             JOIN noble_tiers nt ON nt.tier_id = un.tier_id \
             WHERE ($1::text IS NULL OR un.tier_id = $1) \
               AND ($2::boolean IS NULL OR \
                 CASE WHEN $2 THEN un.expire_at > now() ELSE un.expire_at <= now() END) \
               AND ($3::timestamptz IS NULL OR un.expire_at < $3) \
             ORDER BY un.expire_at ASC \
             LIMIT $4 OFFSET $5",
        )
        .bind(filter.tier_id.as_deref())
        .bind(filter.active_only)
        .bind(filter.expire_before)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn count_noble_users(&self, filter: &UserNobleFilter) -> Result<i64, AppError> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM user_nobles un \
             WHERE ($1::text IS NULL OR un.tier_id = $1) \
               AND ($2::boolean IS NULL OR \
                 CASE WHEN $2 THEN un.expire_at > now() ELSE un.expire_at <= now() END) \
               AND ($3::timestamptz IS NULL OR un.expire_at < $3)",
        )
        .bind(filter.tier_id.as_deref())
        .bind(filter.active_only)
        .bind(filter.expire_before)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    async fn get_noble_history(&self, user_id: Uuid) -> Result<Vec<NobleHistoryItem>, AppError> {
        let rows: Vec<PgHistoryRow> = sqlx::query_as(
            "SELECT id, user_id, event, from_tier, to_tier, actor, payload, created_at \
             FROM noble_history \
             WHERE user_id = $1 \
             ORDER BY created_at DESC \
             LIMIT 100",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

// ─── DB row types ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct PgTierRow {
    tier_id: String,
    name_en: String,
    name_ar: String,
    level: i16,
    monthly_diamonds: i64,
    monthly_usd: Option<String>,
    usd_sku_id: Option<String>,
    privileges: Value,
    icon_url: String,
    frame_url: String,
    entrance_animation_url: Option<String>,
    bgm_url: Option<String>,
    badge_color: String,
    bubble_style_id: String,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PgTierRow> for NobleTierRow {
    fn from(r: PgTierRow) -> Self {
        NobleTierRow {
            tier_id: r.tier_id,
            name_en: r.name_en,
            name_ar: r.name_ar,
            level: r.level,
            monthly_diamonds: r.monthly_diamonds,
            monthly_usd: r.monthly_usd.unwrap_or_default(),
            usd_sku_id: r.usd_sku_id,
            privileges: r.privileges,
            icon_url: r.icon_url,
            frame_url: r.frame_url,
            entrance_animation_url: r.entrance_animation_url,
            bgm_url: r.bgm_url,
            badge_color: r.badge_color,
            bubble_style_id: r.bubble_style_id,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PgUserNobleRow {
    user_id: Uuid,
    tier_id: String,
    start_at: DateTime<Utc>,
    current_period_start: DateTime<Utc>,
    expire_at: DateTime<Utc>,
    auto_renew: bool,
    renew_channel: String,
    total_paid_diamonds: i64,
    total_paid_usd_micros: i64,
}

impl From<PgUserNobleRow> for UserNobleRow {
    fn from(r: PgUserNobleRow) -> Self {
        UserNobleRow {
            user_id: r.user_id,
            tier_id: r.tier_id,
            start_at: r.start_at,
            current_period_start: r.current_period_start,
            expire_at: r.expire_at,
            auto_renew: r.auto_renew,
            renew_channel: r.renew_channel,
            total_paid_diamonds: r.total_paid_diamonds,
            total_paid_usd_micros: r.total_paid_usd_micros,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PgUserNobleItemRow {
    user_id: Uuid,
    nickname: String,
    avatar_url: Option<String>,
    tier_id: String,
    tier_name_en: String,
    tier_name_ar: String,
    tier_level: i16,
    badge_color: String,
    start_at: DateTime<Utc>,
    current_period_start: DateTime<Utc>,
    expire_at: DateTime<Utc>,
    auto_renew: bool,
    renew_channel: String,
    total_paid_diamonds: i64,
    total_paid_usd_micros: i64,
}

impl From<PgUserNobleItemRow> for UserNobleItem {
    fn from(r: PgUserNobleItemRow) -> Self {
        UserNobleItem {
            user_id: r.user_id,
            nickname: r.nickname,
            avatar_url: r.avatar_url,
            tier_id: r.tier_id,
            tier_name_en: r.tier_name_en,
            tier_name_ar: r.tier_name_ar,
            tier_level: r.tier_level,
            badge_color: r.badge_color,
            start_at: r.start_at,
            current_period_start: r.current_period_start,
            expire_at: r.expire_at,
            auto_renew: r.auto_renew,
            renew_channel: r.renew_channel,
            total_paid_diamonds: r.total_paid_diamonds,
            total_paid_usd_micros: r.total_paid_usd_micros,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PgHistoryRow {
    id: i64,
    user_id: Uuid,
    event: String,
    from_tier: Option<String>,
    to_tier: Option<String>,
    actor: String,
    payload: Value,
    created_at: DateTime<Utc>,
}

impl From<PgHistoryRow> for NobleHistoryItem {
    fn from(r: PgHistoryRow) -> Self {
        NobleHistoryItem {
            id: r.id,
            user_id: r.user_id,
            event: r.event,
            from_tier: r.from_tier,
            to_tier: r.to_tier,
            actor: r.actor,
            payload: r.payload,
            created_at: r.created_at,
        }
    }
}
