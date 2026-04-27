#[cfg(any(test, feature = "test-utils"))]
use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use voice_room_shared::models::room::RoomModel;

use crate::common::error::AppError;

use super::dto::{NewRoom, RoomFieldsUpdate};

// ─── RoomListRow：list 查询结果（JOIN rooms + users）────────────────────────

/// 房间列表查询行，从 rooms JOIN users 得到（含房主信息）
#[derive(Debug, FromRow)]
pub struct RoomListRow {
    pub room_id: Uuid,
    pub title: String,
    pub room_type: String,
    pub member_count: i32,
    pub max_members: i32,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub owner_nickname: String,
    pub owner_avatar: Option<String>,
}

// ─── RoomDetailRow：detail 查询结果（JOIN rooms + users）───────────────────

/// 房间详情查询行，从 rooms JOIN users 得到（含房主完整信息）
#[derive(Debug, FromRow)]
pub struct RoomDetailRow {
    pub room_id: Uuid,
    pub title: String,
    pub room_type: String,
    pub member_count: i32,
    pub max_members: i32,
    pub created_at: DateTime<Utc>,
    pub owner_user_id: Uuid,
    pub owner_nickname: String,
    pub owner_avatar: Option<String>,
}

/// 房间持久化抽象，隔离真实 DB 与测试 Fake。
#[async_trait]
pub trait RoomRepository: Send + Sync {
    async fn find_active_by_owner(&self, owner_id: Uuid) -> Result<Option<RoomModel>, AppError>;
    async fn create(&self, room: NewRoom) -> Result<RoomModel, AppError>;

    // ── T-00008 新增 ────────────────────────────────────────────────────
    /// 统计所有活跃（未关闭、未软删除）的房间数量
    async fn count_active_rooms(&self) -> Result<i64, AppError>;
    /// 分页查询活跃房间，按 member_count DESC, created_at DESC 排序
    async fn find_active_rooms(&self, page: i64, size: i64) -> Result<Vec<RoomListRow>, AppError>;

    // ── T-00009 新增 ────────────────────────────────────────────────────
    /// 按 ID 查询单个活跃（未关闭、未软删除）房间（含房主信息）
    async fn find_room_by_id(&self, room_id: Uuid) -> Result<Option<RoomDetailRow>, AppError>;

    // ── T-00010 新增 ────────────────────────────────────────────────────
    /// 按 ID 查询任意状态房间（不过滤 status，仅过滤软删除）
    async fn find_room_any_status(&self, room_id: Uuid) -> Result<Option<RoomModel>, AppError>;
    /// 将指定房间状态设为 closed（只执行 UPDATE，不做 owner 校验）
    async fn set_room_closed(&self, room_id: Uuid) -> Result<bool, AppError>;

    // ── T-00025 新增 ────────────────────────────────────────────────────
    /// Partial update：更新 title / announcement / category（None = 不变）
    async fn update_room_fields(
        &self,
        room_id: Uuid,
        update: RoomFieldsUpdate,
    ) -> Result<RoomModel, AppError>;
}

// ─── Postgres 实现 ────────────────────────────────────────────────────────────

pub struct PgRoomRepository {
    pool: PgPool,
}

impl PgRoomRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RoomRepository for PgRoomRepository {
    async fn find_active_by_owner(&self, owner_id: Uuid) -> Result<Option<RoomModel>, AppError> {
        let room = sqlx::query_as::<_, RoomModel>(
            "SELECT id, owner_id, title, room_type, member_count, status, password_hash, \
             max_members, created_at, updated_at, deleted_at, \
             cover_url, category, announcement, admin_user_id \
             FROM rooms \
             WHERE owner_id = $1 AND status = 'active' AND deleted_at IS NULL",
        )
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(room)
    }

    async fn create(&self, room: NewRoom) -> Result<RoomModel, AppError> {
        let model = sqlx::query_as::<_, RoomModel>(
            "INSERT INTO rooms (owner_id, title, room_type, password_hash, cover_url, category, announcement) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             RETURNING id, owner_id, title, room_type, member_count, status, password_hash, \
                       max_members, created_at, updated_at, deleted_at, \
                       cover_url, category, announcement, admin_user_id",
        )
        .bind(room.owner_id)
        .bind(room.title)
        .bind(room.room_type)
        .bind(room.password_hash)
        .bind(room.cover_url)
        .bind(room.category)
        .bind(room.announcement)
        .fetch_one(&self.pool)
        .await?;
        Ok(model)
    }

    async fn count_active_rooms(&self) -> Result<i64, AppError> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rooms WHERE status = 'active' AND deleted_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    async fn find_active_rooms(&self, page: i64, size: i64) -> Result<Vec<RoomListRow>, AppError> {
        let offset = (page - 1) * size;
        let rows = sqlx::query_as::<_, RoomListRow>(
            r#"
            SELECT r.id        AS room_id,
                   r.title,
                   r.room_type,
                   r.member_count,
                   r.max_members,
                   r.owner_id,
                   r.created_at,
                   u.nickname  AS owner_nickname,
                   u.avatar    AS owner_avatar
            FROM rooms r
            JOIN users u ON u.id = r.owner_id
            WHERE r.status = 'active' AND r.deleted_at IS NULL
            ORDER BY r.member_count DESC, r.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn find_room_by_id(&self, room_id: Uuid) -> Result<Option<RoomDetailRow>, AppError> {
        let row = sqlx::query_as::<_, RoomDetailRow>(
            r#"
            SELECT r.id         AS room_id,
                   r.title,
                   r.room_type,
                   r.member_count,
                   r.max_members,
                   r.created_at,
                   u.id         AS owner_user_id,
                   u.nickname   AS owner_nickname,
                   u.avatar     AS owner_avatar
            FROM rooms r
            JOIN users u ON u.id = r.owner_id
            WHERE r.id = $1
              AND r.status = 'active'
              AND r.deleted_at IS NULL
            "#,
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_room_any_status(&self, room_id: Uuid) -> Result<Option<RoomModel>, AppError> {
        let room = sqlx::query_as::<_, RoomModel>(
            "SELECT id, owner_id, title, room_type, member_count, status, password_hash, \
             max_members, created_at, updated_at, deleted_at, \
             cover_url, category, announcement, admin_user_id \
             FROM rooms \
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(room)
    }

    async fn set_room_closed(&self, room_id: Uuid) -> Result<bool, AppError> {
        // 关闭房间必须在 SQL 层做 status 守卫，保证并发关闭只有一次成功（缺陷 #3）。
        // rows_affected = 1 → 本次成功；= 0 → 已被并发关闭 / 已 closed / 已软删除。
        let result = sqlx::query(
            "UPDATE rooms SET status = 'closed', updated_at = NOW() \
             WHERE id = $1 AND status = 'active' AND deleted_at IS NULL",
        )
        .bind(room_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn update_room_fields(
        &self,
        room_id: Uuid,
        update: RoomFieldsUpdate,
    ) -> Result<RoomModel, AppError> {
        // COALESCE：None = 不变；announcement 用 CASE 区分"不更新"和"清空（空串→NULL）"
        let model = sqlx::query_as::<_, RoomModel>(
            r#"
            UPDATE rooms
            SET title       = COALESCE($2, title),
                category    = COALESCE($3, category),
                announcement = CASE
                                 WHEN $4::TEXT IS NOT NULL
                                 THEN NULLIF($4, '')
                                 ELSE announcement
                               END,
                updated_at  = NOW()
            WHERE id = $1
              AND deleted_at IS NULL
            RETURNING id, owner_id, title, room_type, member_count, status, password_hash,
                      max_members, created_at, updated_at, deleted_at,
                      cover_url, category, announcement, admin_user_id
            "#,
        )
        .bind(room_id)
        .bind(update.title)
        .bind(update.category)
        .bind(update.announcement)
        .fetch_one(&self.pool)
        .await?;
        Ok(model)
    }
}

// ─── Fake 实现（内存，用于单元测试）─────────────────────────────────────────
//
// 缺陷 #8：限制为 cfg(test) 或显式 `test-utils` feature 才编译，
// 防止生产二进制包含测试桩、且阻止生产代码误用 seed_* helpers。

#[cfg(any(test, feature = "test-utils"))]
#[derive(Default)]
pub struct FakeRoomRepository {
    rooms: Mutex<HashMap<Uuid, RoomModel>>,
    /// user_id → (nickname, avatar)
    users: Mutex<HashMap<Uuid, (String, Option<String>)>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeRoomRepository {
    /// 测试辅助：预置一个房间
    pub fn seed(&self, room: RoomModel) {
        self.rooms.lock().unwrap().insert(room.id, room);
    }

    /// 测试辅助：预置一个用户（nickname + avatar），用于 find_active_rooms JOIN
    pub fn seed_user(&self, id: Uuid, nickname: String, avatar: Option<String>) {
        self.users.lock().unwrap().insert(id, (nickname, avatar));
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl RoomRepository for FakeRoomRepository {
    async fn find_active_by_owner(&self, owner_id: Uuid) -> Result<Option<RoomModel>, AppError> {
        Ok(self
            .rooms
            .lock()
            .unwrap()
            .values()
            .find(|r| r.owner_id == owner_id && r.status == "active" && r.deleted_at.is_none())
            .cloned())
    }

    async fn create(&self, room: NewRoom) -> Result<RoomModel, AppError> {
        let now = Utc::now();
        let model = RoomModel {
            id: Uuid::new_v4(),
            owner_id: room.owner_id,
            title: room.title,
            room_type: room.room_type,
            member_count: 0,
            status: "active".to_string(),
            password_hash: room.password_hash,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            // T-00025: 使用请求传入的新字段
            cover_url: room.cover_url,
            category: room.category,
            announcement: room.announcement,
            admin_user_id: None,
        };
        self.rooms.lock().unwrap().insert(model.id, model.clone());
        Ok(model)
    }

    async fn count_active_rooms(&self) -> Result<i64, AppError> {
        let count = self
            .rooms
            .lock()
            .unwrap()
            .values()
            .filter(|r| r.status == "active" && r.deleted_at.is_none())
            .count() as i64;
        Ok(count)
    }

    async fn find_active_rooms(&self, page: i64, size: i64) -> Result<Vec<RoomListRow>, AppError> {
        let rooms_guard = self.rooms.lock().unwrap();
        let users_guard = self.users.lock().unwrap();

        // 过滤 active AND not soft-deleted
        let mut active: Vec<&RoomModel> = rooms_guard
            .values()
            .filter(|r| r.status == "active" && r.deleted_at.is_none())
            .collect();

        // 排序：member_count DESC, created_at DESC
        active.sort_by(|a, b| {
            b.member_count
                .cmp(&a.member_count)
                .then_with(|| b.created_at.cmp(&a.created_at))
        });

        // 分页
        let offset = ((page - 1) * size) as usize;
        let rows = active
            .into_iter()
            .skip(offset)
            .take(size as usize)
            .map(|r| {
                let (nickname, avatar) = users_guard
                    .get(&r.owner_id)
                    .cloned()
                    .unwrap_or_else(|| ("unknown".to_string(), None));
                RoomListRow {
                    room_id: r.id,
                    title: r.title.clone(),
                    room_type: r.room_type.clone(),
                    member_count: r.member_count,
                    max_members: r.max_members,
                    owner_id: r.owner_id,
                    created_at: r.created_at,
                    owner_nickname: nickname,
                    owner_avatar: avatar,
                }
            })
            .collect();

        Ok(rows)
    }

    async fn find_room_by_id(&self, room_id: Uuid) -> Result<Option<RoomDetailRow>, AppError> {
        let rooms_guard = self.rooms.lock().unwrap();
        let users_guard = self.users.lock().unwrap();

        let room = rooms_guard
            .get(&room_id)
            .filter(|r| r.status == "active" && r.deleted_at.is_none());

        match room {
            None => Ok(None),
            Some(r) => {
                let (nickname, avatar) = users_guard
                    .get(&r.owner_id)
                    .cloned()
                    .unwrap_or_else(|| ("unknown".to_string(), None));
                Ok(Some(RoomDetailRow {
                    room_id: r.id,
                    title: r.title.clone(),
                    room_type: r.room_type.clone(),
                    member_count: r.member_count,
                    max_members: r.max_members,
                    created_at: r.created_at,
                    owner_user_id: r.owner_id,
                    owner_nickname: nickname,
                    owner_avatar: avatar,
                }))
            }
        }
    }

    async fn find_room_any_status(&self, room_id: Uuid) -> Result<Option<RoomModel>, AppError> {
        Ok(self
            .rooms
            .lock()
            .unwrap()
            .get(&room_id)
            .filter(|r| r.deleted_at.is_none())
            .cloned())
    }

    async fn set_room_closed(&self, room_id: Uuid) -> Result<bool, AppError> {
        // 在锁内做 status 守卫：仅当 status="active" 才置为 closed，模拟 SQL 原子 UPDATE。
        let mut rooms = self.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(&room_id) {
            if room.deleted_at.is_none() && room.status == "active" {
                room.status = "closed".to_string();
                room.updated_at = Utc::now();
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn update_room_fields(
        &self,
        room_id: Uuid,
        update: RoomFieldsUpdate,
    ) -> Result<RoomModel, AppError> {
        let mut rooms = self.rooms.lock().unwrap();
        let room = rooms
            .get_mut(&room_id)
            .filter(|r| r.deleted_at.is_none())
            .ok_or_else(|| AppError::NotFound(format!("room {room_id}")))?;

        if let Some(title) = update.title {
            room.title = title;
        }
        if let Some(cat) = update.category {
            room.category = cat;
        }
        if let Some(ann) = update.announcement {
            // 空串 = 清空公告（设为 None）
            room.announcement = if ann.is_empty() { None } else { Some(ann) };
        }
        room.updated_at = Utc::now();

        Ok(room.clone())
    }
}
