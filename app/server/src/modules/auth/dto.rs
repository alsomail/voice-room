use serde::{Deserialize, Serialize};

/// POST /api/v1/auth/verification-codes
///
/// `#[serde(deny_unknown_fields)]` — 拒绝含未知字段的请求体，防止字段注入。
/// PROTO-BINDING: doc/protocol/HTTP POST /api/v1/auth/verification-codes
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendCodeRequest {
    pub phone: String,
}

/// POST /api/v1/auth/verification-codes 成功响应 data
#[derive(Debug, Serialize)]
pub struct SendCodeResponse {
    pub expires_in: u64,
    pub cooldown: u64,
}

/// POST /api/v1/auth/login
///
/// `#[serde(deny_unknown_fields)]` — 拒绝含未知字段的请求体，防止字段注入。
/// PROTO-BINDING: doc/protocol/HTTP POST /api/v1/auth/login
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    pub phone: String,
    pub code: String,
}

/// POST /api/v1/auth/login 响应 data（参见 protocol.md §2.2）
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
    pub user: LoginUserInfo,
}

/// POST /api/v1/auth/login 中的用户信息（含 is_new，参见 protocol.md §2.2）
#[derive(Debug, Serialize, Clone)]
pub struct LoginUserInfo {
    pub id: String,
    pub phone: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub coin_balance: i64,
    pub vip_level: i16,
    pub is_new: bool,
    pub created_at: String,
}

/// GET /api/v1/users/me 响应 data（不含 is_banned / is_new，参见 protocol.md §2.3）
#[derive(Debug, Serialize, Clone)]
pub struct UserResponse {
    pub id: String,
    pub phone: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub coin_balance: i64,
    pub vip_level: i16,
    pub created_at: String,
}
