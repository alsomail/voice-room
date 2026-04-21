use serde::{Deserialize, Serialize};

// ─── 请求 DTO ─────────────────────────────────────────────────────────────────

/// POST /api/v1/admin/users/:id/wallet/adjust 请求体
#[derive(Debug, Deserialize)]
pub struct AdjustBalanceRequest {
    /// 非零整数，正数=加，负数=扣；绝对值 ≤ 10,000,000
    pub amount: i64,
    /// 必填，2~200 字符
    pub reason: String,
}

// ─── 响应 DTO ─────────────────────────────────────────────────────────────────

/// POST /api/v1/admin/users/:id/wallet/adjust 成功响应 data 字段
#[derive(Debug, Serialize)]
pub struct AdjustBalanceResponse {
    pub user_id: String,
    pub new_balance: i64,
    pub delta: i64,
}
