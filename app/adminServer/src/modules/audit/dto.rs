use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// HTTP 查询参数 DTO（GET /api/v1/admin/logs）
#[derive(Debug, Deserialize)]
pub struct ListLogsQuery {
    pub admin_id: Option<Uuid>,
    pub action: Option<String>,
    /// RFC3339 字符串，由 service 层解析为 DateTime<Utc>
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    /// 页码，1-based，默认 1
    pub page: Option<i64>,
    /// 每页条数，默认 20，上限 100
    pub size: Option<i64>,
}

/// 查询接口响应结构
#[derive(Debug, Serialize)]
pub struct ListLogsResponse {
    pub total: i64,
    pub page: i64,
    pub size: i64,
    pub items: Vec<AdminLogItem>,
}

/// 单条审计日志
#[derive(Debug, Serialize)]
pub struct AdminLogItem {
    pub id: Uuid,
    pub admin_id: Uuid,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
