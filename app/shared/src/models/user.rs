use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserModel {
    pub id: Uuid,
    pub phone: String,
    pub nickname: String,
    pub avatar: Option<String>,
    pub coin_balance: i64,
    /// 钻石余额（T-00017）；CHECK >= 0 由 DB 约束保证，默认 0
    pub diamond_balance: i64,
    pub vip_level: i16,
    pub is_banned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
