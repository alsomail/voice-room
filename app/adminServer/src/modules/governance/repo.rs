use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::error::AppError;

// ─── 数据结构 ─────────────────────────────────────────────────────────────────

/// 踢人日志条目（对应 room_kick_records 关联查询结果）
#[derive(Debug, Clone, Serialize)]
pub struct KickLogItem {
    pub id: Uuid,
    pub room_id: Uuid,
    pub room_title: String,
    pub target_user_id: Uuid,
    pub target_nickname: String,
    pub operator_user_id: Uuid,
    pub operator_nickname: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 禁言日志条目（对应 room_mute_records 关联查询结果）
#[derive(Debug, Clone, Serialize)]
pub struct MuteLogItem {
    pub id: Uuid,
    pub room_id: Uuid,
    pub room_title: String,
    pub target_user_id: Uuid,
    pub target_nickname: String,
    pub operator_user_id: Uuid,
    pub operator_nickname: String,
    /// 禁言类型：`mic`（麦克风）或 `chat`（聊天）
    pub mute_type: String,
    /// 禁言时长（秒），None 表示永久禁言
    pub duration_sec: Option<i64>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 治理日志通用查询过滤器
#[derive(Debug, Default, Clone)]
pub struct GovernanceFilter {
    pub room_id: Option<Uuid>,
    pub target_user_id: Option<Uuid>,
    pub operator_user_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    /// 仅用于 mutes 查询的类型过滤（`mic` / `chat`）
    pub mute_type: Option<String>,
}

// ─── Repository Trait ─────────────────────────────────────────────────────────

/// 治理日志数据访问抽象（踢人 + 禁言）。
#[async_trait]
pub trait GovernanceRepo: Send + Sync {
    /// 查询踢人记录，按 created_at DESC 排序。
    /// 返回 `(total_count, items)`。
    async fn find_kicks(
        &self,
        filter: &GovernanceFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<KickLogItem>), AppError>;

    /// 查询禁言记录，按 created_at DESC 排序。
    /// 返回 `(total_count, items)`。
    async fn find_mutes(
        &self,
        filter: &GovernanceFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<MuteLogItem>), AppError>;
}

// ─── Fake 实现（内存，用于单元/集成测试）──────────────────────────────────────

/// 测试专用：内存 GovernanceRepo。
pub struct FakeGovernanceRepo {
    kicks: Arc<Mutex<Vec<KickLogItem>>>,
    mutes: Arc<Mutex<Vec<MuteLogItem>>>,
}

impl Default for FakeGovernanceRepo {
    fn default() -> Self {
        Self {
            kicks: Arc::new(Mutex::new(vec![])),
            mutes: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl FakeGovernanceRepo {
    /// 插入一条踢人记录（供测试预填充数据）。
    pub fn push_kick(&self, item: KickLogItem) {
        self.kicks.lock().unwrap().push(item);
    }

    /// 插入一条禁言记录（供测试预填充数据）。
    pub fn push_mute(&self, item: MuteLogItem) {
        self.mutes.lock().unwrap().push(item);
    }
}

#[async_trait]
impl GovernanceRepo for FakeGovernanceRepo {
    async fn find_kicks(
        &self,
        filter: &GovernanceFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<KickLogItem>), AppError> {
        let mut items: Vec<KickLogItem> = {
            let kicks = self.kicks.lock().unwrap();
            kicks
                .iter()
                .filter(|k| {
                    // room_id 过滤
                    if let Some(rid) = filter.room_id {
                        if k.room_id != rid {
                            return false;
                        }
                    }
                    // target_user_id 过滤
                    if let Some(uid) = filter.target_user_id {
                        if k.target_user_id != uid {
                            return false;
                        }
                    }
                    // operator_user_id 过滤
                    if let Some(oid) = filter.operator_user_id {
                        if k.operator_user_id != oid {
                            return false;
                        }
                    }
                    // 时间范围过滤
                    if let Some(from) = filter.from {
                        if k.created_at < from {
                            return false;
                        }
                    }
                    if let Some(to) = filter.to {
                        if k.created_at > to {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect()
        };

        // 按 created_at DESC 排序
        items.sort_by_key(|k| std::cmp::Reverse(k.created_at));

        let total = items.len() as i64;
        let paged: Vec<KickLogItem> = items
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        Ok((total, paged))
    }

    async fn find_mutes(
        &self,
        filter: &GovernanceFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<MuteLogItem>), AppError> {
        let mut items: Vec<MuteLogItem> = {
            let mutes = self.mutes.lock().unwrap();
            mutes
                .iter()
                .filter(|m| {
                    // room_id 过滤
                    if let Some(rid) = filter.room_id {
                        if m.room_id != rid {
                            return false;
                        }
                    }
                    // target_user_id 过滤
                    if let Some(uid) = filter.target_user_id {
                        if m.target_user_id != uid {
                            return false;
                        }
                    }
                    // operator_user_id 过滤
                    if let Some(oid) = filter.operator_user_id {
                        if m.operator_user_id != oid {
                            return false;
                        }
                    }
                    // mute_type 过滤
                    if let Some(ref t) = filter.mute_type {
                        if &m.mute_type != t {
                            return false;
                        }
                    }
                    // 时间范围过滤
                    if let Some(from) = filter.from {
                        if m.created_at < from {
                            return false;
                        }
                    }
                    if let Some(to) = filter.to {
                        if m.created_at > to {
                            return false;
                        }
                    }
                    true
                })
                .cloned()
                .collect()
        };

        // 按 created_at DESC 排序
        items.sort_by_key(|k| std::cmp::Reverse(k.created_at));

        let total = items.len() as i64;
        let paged: Vec<MuteLogItem> = items
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

        Ok((total, paged))
    }
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

/// 基于 SQLx + PostgreSQL 的 GovernanceRepo 生产实现。
pub struct PgGovernanceRepo {
    pool: PgPool,
}

impl PgGovernanceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GovernanceRepo for PgGovernanceRepo {
    async fn find_kicks(
        &self,
        filter: &GovernanceFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<KickLogItem>), AppError> {
        let from = filter.from.unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
        let to = filter.to.unwrap_or_else(Utc::now);

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM room_kick_records k \
             WHERE ($1::uuid IS NULL OR k.room_id = $1) \
               AND ($2::uuid IS NULL OR k.target_user_id = $2) \
               AND ($3::uuid IS NULL OR k.operator_user_id = $3) \
               AND k.created_at BETWEEN $4 AND $5",
        )
        .bind(filter.room_id)
        .bind(filter.target_user_id)
        .bind(filter.operator_user_id)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await?;

        let rows: Vec<KickRow> = sqlx::query_as(
            "SELECT k.id, k.room_id, r.title as room_title, \
                    k.target_user_id, tu.nickname as target_nickname, \
                    k.operator_user_id, ou.nickname as operator_nickname, \
                    k.reason, k.created_at \
             FROM room_kick_records k \
             JOIN rooms r ON r.id = k.room_id \
             JOIN users tu ON tu.id = k.target_user_id \
             JOIN users ou ON ou.id = k.operator_user_id \
             WHERE ($1::uuid IS NULL OR k.room_id = $1) \
               AND ($2::uuid IS NULL OR k.target_user_id = $2) \
               AND ($3::uuid IS NULL OR k.operator_user_id = $3) \
               AND k.created_at BETWEEN $4 AND $5 \
             ORDER BY k.created_at DESC \
             LIMIT $6 OFFSET $7",
        )
        .bind(filter.room_id)
        .bind(filter.target_user_id)
        .bind(filter.operator_user_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|r| KickLogItem {
                id: r.id,
                room_id: r.room_id,
                room_title: r.room_title,
                target_user_id: r.target_user_id,
                target_nickname: r.target_nickname,
                operator_user_id: r.operator_user_id,
                operator_nickname: r.operator_nickname,
                reason: r.reason,
                created_at: r.created_at,
            })
            .collect();

        Ok((count.0, items))
    }

    async fn find_mutes(
        &self,
        filter: &GovernanceFilter,
        limit: i64,
        offset: i64,
    ) -> Result<(i64, Vec<MuteLogItem>), AppError> {
        let from = filter.from.unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
        let to = filter.to.unwrap_or_else(Utc::now);

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM room_mute_records m \
             WHERE ($1::uuid IS NULL OR m.room_id = $1) \
               AND ($2::uuid IS NULL OR m.target_user_id = $2) \
               AND ($3::uuid IS NULL OR m.operator_user_id = $3) \
               AND ($4::text IS NULL OR m.mute_type = $4) \
               AND m.created_at BETWEEN $5 AND $6",
        )
        .bind(filter.room_id)
        .bind(filter.target_user_id)
        .bind(filter.operator_user_id)
        .bind(filter.mute_type.as_deref())
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await?;

        let rows: Vec<MuteRow> = sqlx::query_as(
            "SELECT m.id, m.room_id, r.title as room_title, \
                    m.target_user_id, tu.nickname as target_nickname, \
                    m.operator_user_id, ou.nickname as operator_nickname, \
                    m.mute_type, m.duration_sec, m.reason, m.created_at \
             FROM room_mute_records m \
             JOIN rooms r ON r.id = m.room_id \
             JOIN users tu ON tu.id = m.target_user_id \
             JOIN users ou ON ou.id = m.operator_user_id \
             WHERE ($1::uuid IS NULL OR m.room_id = $1) \
               AND ($2::uuid IS NULL OR m.target_user_id = $2) \
               AND ($3::uuid IS NULL OR m.operator_user_id = $3) \
               AND ($4::text IS NULL OR m.mute_type = $4) \
               AND m.created_at BETWEEN $5 AND $6 \
             ORDER BY m.created_at DESC \
             LIMIT $7 OFFSET $8",
        )
        .bind(filter.room_id)
        .bind(filter.target_user_id)
        .bind(filter.operator_user_id)
        .bind(filter.mute_type.as_deref())
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|r| MuteLogItem {
                id: r.id,
                room_id: r.room_id,
                room_title: r.room_title,
                target_user_id: r.target_user_id,
                target_nickname: r.target_nickname,
                operator_user_id: r.operator_user_id,
                operator_nickname: r.operator_nickname,
                mute_type: r.mute_type,
                duration_sec: r.duration_sec,
                reason: r.reason,
                created_at: r.created_at,
            })
            .collect();

        Ok((count.0, items))
    }
}

// ─── DB 行类型（sqlx::FromRow）────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct KickRow {
    id: Uuid,
    room_id: Uuid,
    room_title: String,
    target_user_id: Uuid,
    target_nickname: String,
    operator_user_id: Uuid,
    operator_nickname: String,
    reason: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct MuteRow {
    id: Uuid,
    room_id: Uuid,
    room_title: String,
    target_user_id: Uuid,
    target_nickname: String,
    operator_user_id: Uuid,
    operator_nickname: String,
    mute_type: String,
    duration_sec: Option<i64>,
    reason: Option<String>,
    created_at: DateTime<Utc>,
}
