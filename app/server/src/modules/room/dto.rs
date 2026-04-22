use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// POST /api/v1/rooms 请求体（T-00025 扩展：新增 cover_url/category/announcement）
#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub title: String,
    pub room_type: String,
    /// 封面 URL（可选，必须满足白名单前缀）
    pub cover_url: Option<String>,
    /// 房间分类（可选，枚举值之一）
    pub category: Option<String>,
    /// 房间公告（可选，≤200 Unicode 字符）
    pub announcement: Option<String>,
    /// 密码（仅 room_type=password 时需提供，必须为 6 位数字）
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
    /// 封面 URL（空串 = 无封面）
    pub cover_url: String,
    /// 房间分类（默认 "chat"）
    pub category: String,
    /// 房间公告（可选）
    pub announcement: Option<String>,
}

/// PATCH /api/v1/rooms/:id 请求体（T-00025 新增）
///
/// 至少一个字段非 None，否则 40003。
/// `announcement: Some("")` 表示清空公告。
#[derive(Debug, Deserialize)]
pub struct PatchRoomRequest {
    pub title: Option<String>,
    /// `Some("")` = 清空公告，`Some("text")` = 设置，`None` = 不变
    pub announcement: Option<String>,
    pub category: Option<String>,
}

/// PATCH /api/v1/rooms/:id 成功响应 data（T-00025 新增）
#[derive(Debug, Serialize, Clone)]
pub struct PatchRoomResponse {
    pub room_id: String,
    pub title: String,
    pub announcement: Option<String>,
    pub category: String,
    pub cover_url: String,
    pub has_password: bool,
}

/// repository::update_room_fields 的 partial update 数据（T-00025 新增）
pub struct RoomFieldsUpdate {
    pub title: Option<String>,
    /// `Some("")` = 清空到 NULL，`Some("text")` = 覆盖，`None` = 不变
    pub announcement: Option<String>,
    pub category: Option<String>,
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
