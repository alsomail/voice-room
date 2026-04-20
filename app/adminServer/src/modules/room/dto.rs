use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── 请求 DTO ────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/rooms 查询参数。
#[derive(Debug, Deserialize)]
pub struct AdminRoomListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    /// 可选，"active" | "closed"
    pub status: Option<String>,
    /// 可选，按 title 关键词过滤（大小写不敏感）
    pub keyword: Option<String>,
}

// ─── 仓库过滤器 ──────────────────────────────────────────────────────────────

/// 传递给 AdminRoomRepository 的过滤条件。
#[derive(Debug, Clone, Default)]
pub struct AdminRoomFilter {
    pub status: Option<String>,
    pub keyword: Option<String>,
}

// ─── 仓库行 ──────────────────────────────────────────────────────────────────

/// rooms JOIN users 查询的单行结果（由仓库返回给 service）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminRoomListRow {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub room_type: String,
    pub member_count: i32,
    pub max_members: i32,
    pub owner_id: Uuid,
    pub owner_nickname: String,
    pub owner_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ─── 响应 DTO ────────────────────────────────────────────────────────────────

/// 房间列表接口响应中每个房间的数据。
#[derive(Debug, Clone, Serialize)]
pub struct AdminRoomItem {
    pub id: String,
    pub title: String,
    pub status: String,
    pub room_type: String,
    pub member_count: i32,
    pub max_members: i32,
    pub owner_id: String,
    pub owner_nickname: String,
    pub owner_avatar: Option<String>,
    pub created_at: String,
}

impl From<AdminRoomListRow> for AdminRoomItem {
    fn from(row: AdminRoomListRow) -> Self {
        Self {
            id: row.id.to_string(),
            title: row.title,
            status: row.status,
            room_type: row.room_type,
            member_count: row.member_count,
            max_members: row.max_members,
            owner_id: row.owner_id.to_string(),
            owner_nickname: row.owner_nickname,
            owner_avatar: row.owner_avatar,
            created_at: row.created_at.to_rfc3339(),
        }
    }
}

/// GET /api/v1/admin/rooms 成功响应的 data 部分。
#[derive(Debug, Serialize)]
pub struct AdminRoomListResponse {
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub items: Vec<AdminRoomItem>,
}

// ─── 房间详情 DTO（T-10005）──────────────────────────────────────────────────

/// rooms JOIN users 查询的单行详情结果（由仓库返回给 service）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminRoomDetailRow {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub room_type: String,
    pub member_count: i32,
    pub max_members: i32,
    pub owner_id: Uuid,
    pub owner_nickname: String,
    pub owner_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// GET /api/v1/admin/rooms/:id 响应中房主信息嵌套对象。
#[derive(Debug, Clone, Serialize)]
pub struct AdminOwnerInfo {
    pub user_id: String,
    pub nickname: String,
    pub avatar: Option<String>,
}

/// GET /api/v1/admin/rooms/:id 成功响应的 data 部分。
#[derive(Debug, Clone, Serialize)]
pub struct AdminRoomDetailResponse {
    pub room_id: String,
    pub title: String,
    pub status: String,
    pub room_type: String,
    pub member_count: i32,
    pub max_members: i32,
    pub owner: AdminOwnerInfo,
    /// MVP 阶段固定空数组，后续迭代填充麦位信息。
    pub mic_slots: Vec<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AdminRoomDetailRow> for AdminRoomDetailResponse {
    fn from(row: AdminRoomDetailRow) -> Self {
        Self {
            room_id: row.id.to_string(),
            title: row.title,
            status: row.status,
            room_type: row.room_type,
            member_count: row.member_count,
            max_members: row.max_members,
            owner: AdminOwnerInfo {
                user_id: row.owner_id.to_string(),
                nickname: row.owner_nickname,
                avatar: row.owner_avatar,
            },
            mic_slots: vec![],
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
        }
    }
}
