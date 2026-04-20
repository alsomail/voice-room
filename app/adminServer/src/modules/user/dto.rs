use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── 请求 DTO ────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/users 查询参数。
#[derive(Debug, Deserialize)]
pub struct AdminUserListQuery {
    pub phone: Option<String>,
    pub nickname: Option<String>,
    pub user_id: Option<Uuid>,
    /// 可选，"normal" | "banned"
    pub status: Option<String>,
    pub page: Option<u32>,
    pub size: Option<u32>,
}

// ─── 仓库过滤器 ──────────────────────────────────────────────────────────────

/// 传递给 AdminUserRepository 的过滤条件。
#[derive(Debug, Clone, Default)]
pub struct AdminUserFilter {
    pub phone: Option<String>,
    pub user_id: Option<Uuid>,
    pub nickname: Option<String>,
    /// None = 全部；Some(false) = normal；Some(true) = banned
    pub is_banned: Option<bool>,
}

// ─── 仓库行 ──────────────────────────────────────────────────────────────────

/// users 表查询的单行结果（由仓库返回给 service）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdminUserListRow {
    pub id: Uuid,
    pub phone: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub coin_balance: i64,
    pub vip_level: i16,
    pub is_banned: bool,
    pub created_at: DateTime<Utc>,
}

// ─── 响应 DTO ────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/users 响应中每个用户的 JSON 结构。
#[derive(Debug, Clone, Serialize)]
pub struct AdminUserItem {
    pub id: String,
    pub phone: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub coin_balance: i64,
    pub vip_level: i16,
    /// "normal" | "banned"，由 is_banned 映射
    pub status: String,
    pub created_at: String, // RFC 3339
}

impl From<AdminUserListRow> for AdminUserItem {
    fn from(row: AdminUserListRow) -> Self {
        Self {
            id: row.id.to_string(),
            phone: row.phone,
            nickname: row.nickname,
            avatar: row.avatar,
            coin_balance: row.coin_balance,
            vip_level: row.vip_level,
            status: if row.is_banned {
                "banned".to_string()
            } else {
                "normal".to_string()
            },
            created_at: row.created_at.to_rfc3339(),
        }
    }
}

/// GET /api/v1/admin/users 成功响应的 data 部分。
#[derive(Debug, Serialize)]
pub struct AdminUserListResponse {
    pub total: i64,
    pub page: u32,
    pub size: u32,
    pub items: Vec<AdminUserItem>,
}

// ─── T-10008: 用户详情 DTO ─────────────────────────────────────────────────

/// MVP 阶段充值记录占位结构（表尚未建立完整 schema）
#[derive(Debug, Clone, Serialize)]
pub struct RechargeRecordItem {
    pub amount: i64,
    pub created_at: String, // RFC 3339
}

/// MVP 阶段消费记录占位结构（表尚未建立完整 schema）
#[derive(Debug, Clone, Serialize)]
pub struct ConsumeRecordItem {
    pub amount: i64,
    /// 消费类型，如 "gift"
    pub r#type: String,
    pub created_at: String, // RFC 3339
}

/// MVP 阶段设备信息占位结构（表尚未建立完整 schema）
#[derive(Debug, Clone, Serialize)]
pub struct DeviceItem {
    pub device_id: String,
    /// 平台，如 "android" | "ios"
    pub platform: String,
    pub last_login_at: String, // RFC 3339
}

/// GET /api/v1/admin/users/:id 成功响应的 data 部分。
#[derive(Debug, Serialize)]
pub struct AdminUserDetailResponse {
    pub id: String,
    pub phone: String,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub coin_balance: i64,
    pub vip_level: i16,
    /// "normal" | "banned"，由 is_banned 映射
    pub status: String,
    pub created_at: String, // RFC 3339
    // TODO: 充值记录表尚未建立，MVP 阶段返回空数组
    pub recharge_records: Vec<RechargeRecordItem>,
    // TODO: 消费记录表尚未建立，MVP 阶段返回空数组
    pub consume_records: Vec<ConsumeRecordItem>,
    // TODO: 设备登录记录表尚未建立，MVP 阶段返回空数组
    pub devices: Vec<DeviceItem>,
}

impl From<AdminUserListRow> for AdminUserDetailResponse {
    fn from(row: AdminUserListRow) -> Self {
        Self {
            id: row.id.to_string(),
            phone: row.phone,
            nickname: row.nickname,
            avatar_url: row.avatar,
            coin_balance: row.coin_balance,
            vip_level: row.vip_level,
            status: if row.is_banned {
                "banned".to_string()
            } else {
                "normal".to_string()
            },
            created_at: row.created_at.to_rfc3339(),
            recharge_records: vec![],
            consume_records: vec![],
            devices: vec![],
        }
    }
}

// ─── T-10009: 封禁/解封 DTO ───────────────────────────────────────────────────

/// POST /api/v1/admin/users/:id/ban 请求体
#[derive(Debug, Deserialize)]
pub struct AdminBanUserRequest {
    /// "ban" | "unban"
    pub action: String,
    /// 仅 action="ban" 有效："permanent" | "temporary"；缺省 "permanent"
    pub ban_type: Option<String>,
    /// 仅 ban_type="temporary" 有效，单位小时
    pub duration_hours: Option<u32>,
    /// 封禁原因（可选，最大 255 字节；unban 时忽略）
    pub reason: Option<String>,
}

/// POST /api/v1/admin/users/:id/ban 成功响应的 data 部分
#[derive(Debug, Serialize)]
pub struct AdminBanUserResponse {
    pub id: String,
    /// "banned" | "normal"
    pub status: String,
}
