use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
};

use super::dto::StatsOverviewQuery;

/// GET /api/v1/admin/stats/overview
///
/// 统计概览接口，需要 StatsRead 权限。
/// - super_admin / operator / finance 可访问
/// - cs 角色被拒绝（403/40301）
pub async fn stats_overview_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(query): Query<StatsOverviewQuery>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::StatsRead) {
        return err_response(e, rc.request_id());
    }

    match state.stats_service.get_overview(query).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}
