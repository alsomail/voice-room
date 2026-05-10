//! T-10028: 财务报告 HTTP handler

use axum::{
    extract::{Extension, Query, State},
    response::{IntoResponse, Response},
};

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::{err_response, AppError},
        response::ApiResponse,
        RequestContext,
    },
};

use super::report_dto::ReportQuery;

// ─── GET /api/v1/admin/payments/reports ─────────────────────────────────────

pub async fn summary_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Query(query): Query<ReportQuery>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::PaymentReport) {
        return err_response(e, req_ctx.request_id());
    }

    // 校验参数
    let (from, to) = match query.validate() {
        Ok(v) => v,
        Err(msg) => return err_response(AppError::ValidationError(msg), req_ctx.request_id()),
    };

    match state
        .report_service
        .build_report(&query.granularity, from, to)
        .await
    {
        Ok(report) => ApiResponse::ok(report, req_ctx.request_id()).into_response(),
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}
