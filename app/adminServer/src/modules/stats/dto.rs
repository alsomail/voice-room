use serde::{Deserialize, Serialize};

/// GET /api/v1/admin/stats/overview 查询参数
#[derive(Debug, Deserialize, Default)]
pub struct StatsOverviewQuery {
    /// 统计起始日期（YYYY-MM-DD），缺省今天
    pub start_date: Option<String>,
    /// 统计截止日期（YYYY-MM-DD），缺省今天
    pub end_date: Option<String>,
}

/// 日期范围（回显用）
#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: String, // YYYY-MM-DD
    pub end: String,   // YYYY-MM-DD
}

/// GET /api/v1/admin/stats/overview 成功响应的 data 部分
#[derive(Debug, Serialize)]
pub struct StatsOverviewResponse {
    pub dau: i64,
    pub new_users: i64,
    pub active_rooms: i64,
    pub online_users: i64,
    pub date_range: DateRange,
}
