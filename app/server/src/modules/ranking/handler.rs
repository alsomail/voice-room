//! Ranking HTTP handler — GET /api/v1/ranking
//!
//! 参数校验（失败立即返回 40003）：
//! - `type`: "charm" | "wealth"（缺失或无效 → 40003）
//! - `period`: "day" | "week"（缺失或无效，默认 "day"）
//! - `limit`: 1-100（缺失时默认 50，超出范围 → 40003）
//!
//! 成功返回：
//! ```json
//! {
//!   "code": 0,
//!   "data": {
//!     "type": "charm", "period": "day", "period_key": "2026-04-21",
//!     "items": [{ "rank":1, "user_id":"...", "nickname":"...", "avatar":"...",
//!                 "score":123456, "medal":"gold" }],
//!     "me": { "rank": 42, "score": 6800 }
//!   }
//! }
//! ```

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::{
    bootstrap::AppState,
    common::{
        auth::AuthContext,
        error::{err_response, AppError},
        response::ApiResponse,
        RequestContext,
    },
};

use super::{Period, RankingType};

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    /// 榜单类型："charm" | "wealth"
    #[serde(rename = "type")]
    pub ty: Option<String>,
    /// 周期："day" | "week"（默认 "day"）
    pub period: Option<String>,
    /// 返回条数，默认 50，范围 1-100
    pub limit: Option<u32>,
}

/// GET /api/v1/ranking?type=charm|wealth&period=day|week&limit=50（需 JWT）
///
/// 参数非法时返回 HTTP 400 code=40003。
pub async fn get_ranking(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Query(query): Query<RankingQuery>,
) -> axum::response::Response {
    // ── 参数校验 ──────────────────────────────────────────────────────────────

    // type 必填，且必须是 charm/wealth
    let ty = match query.ty.as_deref() {
        Some("charm") => RankingType::Charm,
        Some("wealth") => RankingType::Wealth,
        _ => {
            return err_response(
                AppError::ValidationError("type must be 'charm' or 'wealth'".to_string()),
                rc.request_id(),
            );
        }
    };

    // period 可选，默认 day
    let period = match query.period.as_deref() {
        Some("week") => Period::Week,
        Some("day") | None => Period::Day,
        Some(other) => {
            return err_response(
                AppError::ValidationError(format!("invalid period: {other:?}; must be 'day' or 'week'")),
                rc.request_id(),
            );
        }
    };

    // limit 默认 50，范围 1-100
    let limit = query.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return err_response(
            AppError::ValidationError(format!(
                "limit must be between 1 and 100, got {limit}"
            )),
            rc.request_id(),
        );
    }

    // ── 调用服务 ──────────────────────────────────────────────────────────────

    match state
        .ranking_service
        .top(ty, period, limit as usize, Some(ctx.user_id))
        .await
    {
        Ok(data) => (
            StatusCode::OK,
            Json(ApiResponse::ok(data, rc.request_id())),
        )
            .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── 单元测试（Handler 参数解析逻辑）────────────────────────────────────────
#[cfg(test)]
mod tests {
    /// H-01: limit 范围检查逻辑（101 超出范围）
    #[test]
    fn limit_range_101_is_invalid() {
        let limit: u32 = 101;
        assert!(!(1..=100).contains(&limit), "101 should be out of 1-100 range");
    }

    /// H-02: limit 范围检查逻辑（0 超出范围）
    #[test]
    fn limit_range_0_is_invalid() {
        let limit: u32 = 0;
        assert!(!(1..=100).contains(&limit), "0 should be out of 1-100 range");
    }

    /// H-03: limit 1-100 合法
    #[test]
    fn limit_range_1_to_100_valid() {
        for i in 1u32..=100 {
            assert!((1..=100).contains(&i), "{i} should be valid");
        }
    }
}
