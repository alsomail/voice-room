use serde::{Deserialize, Serialize};

/// POST /auth/send-code
#[derive(Debug, Deserialize)]
pub struct SendCodeRequest {
    pub phone: String,
}

/// POST /auth/login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub phone: String,
    pub code: String,
}

/// POST /auth/login 响应体
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserResponse,
}

/// GET /users/me 响应体（也作为 login 内嵌 user 使用）
#[derive(Debug, Serialize, Clone)]
pub struct UserResponse {
    pub id: String,
    pub phone: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub coin_balance: i64,
    pub vip_level: i16,
    pub is_banned: bool,
}
