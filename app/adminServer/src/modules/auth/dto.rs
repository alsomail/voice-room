use serde::{Deserialize, Serialize};

/// POST /api/v1/admin/login 请求体（参见 doc/protocol.md §3.1）
#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

/// POST /api/v1/admin/login 成功响应 data
#[derive(Debug, Serialize, Clone)]
pub struct AdminLoginResponse {
    pub token: String,
    /// JWT 有效期（秒），固定 604800（7 天）
    pub expires_in: u64,
    pub admin: AdminInfo,
}

/// 登录成功后返回的管理员基础信息
#[derive(Debug, Serialize, Clone)]
pub struct AdminInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    pub display_name: Option<String>,
    pub last_login_at: Option<String>,
}
