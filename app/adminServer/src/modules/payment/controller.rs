//! T-10025/26: 订单查询 + 补单/退款 HTTP Handler

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    Extension, Json,
};
use uuid::Uuid;

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
};

use super::{
    dto::{ListOrdersQuery, ListOrdersResponse},
    repo::OrderFilter,
};

// ─── GET /api/v1/admin/payments/orders ───────────────────────────────────────

/// 订单列表查询（分页 + 多条件过滤）。
///
/// 权限：PaymentRead（super_admin / finance / operator）
pub async fn list_orders_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Query(query): Query<ListOrdersQuery>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::PaymentRead) {
        return err_response(e, req_ctx.request_id());
    }

    let (page, page_size) = match query.validate() {
        Ok(v) => v,
        Err(msg) => {
            return err_response(
                crate::common::error::AppError::ValidationError(msg),
                req_ctx.request_id(),
            );
        }
    };

    let filter = OrderFilter {
        user_id: query.user_id,
        state: query.state.clone(),
        provider: query.provider.clone(),
        created_from: query.created_from,
        created_to: query.created_to,
        amount_min: query.amount_min,
        amount_max: query.amount_max,
        offset: ((page - 1) * page_size) as i64,
        limit: page_size as i64,
    };

    match state.payment_order_repo.list_orders(filter).await {
        Ok((total, rows)) => {
            // fire-and-forget audit log
            {
                let logger = state.audit_logger.clone();
                let admin_id = ctx.admin_id;
                let detail = serde_json::json!({
                    "action": "list_payment_orders",
                    "filters": { "page": page, "page_size": page_size },
                });
                tokio::spawn(async move {
                    logger
                        .log_action(
                            admin_id,
                            "list_payment_orders",
                            Some("payment_order"),
                            None,
                            None,
                            Some(detail),
                        )
                        .await;
                });
            }

            let data = rows.into_iter().map(|r| r.to_list_item()).collect();
            ApiResponse::ok(
                ListOrdersResponse {
                    data,
                    total,
                    page,
                    page_size,
                },
                req_ctx.request_id(),
            )
            .into_response()
        }
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

// ─── GET /api/v1/admin/payments/orders/:id ────────────────────────────────────

/// 订单详情查询（含 state_history + Google raw response）。
///
/// 权限：PaymentRead
pub async fn detail_order_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::PaymentRead) {
        return err_response(e, req_ctx.request_id());
    }

    match state.payment_order_repo.find_by_id(order_id).await {
        Ok(Some(row)) => {
            // fire-and-forget audit log
            {
                let logger = state.audit_logger.clone();
                let admin_id = ctx.admin_id;
                tokio::spawn(async move {
                    logger
                        .log_action(
                            admin_id,
                            "view_payment_order_detail",
                            Some("payment_order"),
                            Some(order_id),
                            None,
                            None,
                        )
                        .await;
                });
            }
            ApiResponse::ok(row.to_detail(), req_ctx.request_id()).into_response()
        }
        Ok(None) => err_response(
            crate::common::error::AppError::OrderNotFound(order_id.to_string()),
            req_ctx.request_id(),
        ),
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

// ─── POST /api/v1/admin/payments/orders/:id/recredit ─────────────────────────

#[derive(serde::Deserialize)]
pub struct ReasonRequest {
    pub reason: String,
}

/// 手动补单（FAILED/CANCELLED → CREDITED）。
///
/// 权限：super_admin 角色（require_role）
pub async fn recredit_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
    Json(req): Json<ReasonRequest>,
) -> Response {
    if let Err(e) = ctx.require_role("super_admin") {
        return err_response(e, req_ctx.request_id());
    }

    match state
        .payment_admin_service
        .recredit_order(order_id, ctx.admin_id, &req.reason)
        .await
    {
        Ok(result) => ApiResponse::ok(
            serde_json::json!({
                "order_id": result.order_id,
                "new_state": result.new_state,
                "diamonds_credited": result.diamonds_credited,
            }),
            req_ctx.request_id(),
        )
        .into_response(),
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

// ─── POST /api/v1/admin/payments/orders/:id/refund ───────────────────────────

/// 手动退款（ACKED/CREDITED → REFUNDED）。
///
/// 权限：super_admin 角色
pub async fn refund_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
    Json(req): Json<ReasonRequest>,
) -> Response {
    if let Err(e) = ctx.require_role("super_admin") {
        return err_response(e, req_ctx.request_id());
    }

    match state
        .payment_admin_service
        .refund_order(order_id, ctx.admin_id, &req.reason)
        .await
    {
        Ok(result) => ApiResponse::ok(
            serde_json::json!({
                "order_id": result.order_id,
                "new_state": result.new_state,
                "diamonds_deducted": result.diamonds_deducted,
            }),
            req_ctx.request_id(),
        )
        .into_response(),
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}
