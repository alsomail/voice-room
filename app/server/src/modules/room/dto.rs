use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// POST /api/v1/rooms 请求体
#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub title: String,
    pub room_type: String,
    pub password: Option<String>,
}

/// POST /api/v1/rooms 成功响应 data
#[derive(Debug, Serialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
    pub title: String,
    pub room_type: String,
    pub created_at: String,
}

/// repository::create 所需的新房间数据（已验证 + 已哈希）
pub struct NewRoom {
    pub owner_id: Uuid,
    pub title: String,
    pub room_type: String,
    pub password_hash: Option<String>,
}

// ─── T-00008: 房间列表 ────────────────────────────────────────────────────────

/// GET /api/v1/rooms 查询参数
#[derive(Debug, Deserialize)]
pub struct RoomListQuery {
    pub page: Option<i64>,
    pub size: Option<i64>,
}

/// 房间列表单条记录（含房主信息）
#[derive(Debug, Serialize)]
pub struct RoomListItem {
    pub room_id: String,
    pub title: String,
    pub room_type: String,
    pub member_count: i32,
    pub max_members: i32,
    pub owner_id: String,
    pub owner_nickname: String,
    pub owner_avatar: Option<String>,
    pub created_at: String,
}

/// GET /api/v1/rooms 成功响应 data
#[derive(Debug, Serialize)]
pub struct RoomListResponse {
    pub total: i64,
    pub page: i64,
    pub size: i64,
    pub items: Vec<RoomListItem>,
}

// ─── T-00009: 房间详情 ────────────────────────────────────────────────────────

/// 房主信息（嵌套在 RoomDetailResponse 中）
#[derive(Debug, Serialize)]
pub struct OwnerInfo {
    pub user_id: String,
    pub nickname: String,
    pub avatar: Option<String>,
}

/// GET /api/v1/rooms/:id 成功响应 data
#[derive(Debug, Serialize)]
pub struct RoomDetailResponse {
    pub room_id: String,
    pub title: String,
    pub room_type: String,
    pub member_count: i32,
    pub max_members: i32,
    pub owner: OwnerInfo,
    pub mic_slots: Vec<serde_json::Value>, // MVP 固定空数组
    pub created_at: String,
}
