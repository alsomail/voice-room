//! T-10027: SKU HTTP handlers

use axum::{
    extract::{Extension, Path, Query, State},
    response::{IntoResponse, Response},
    Json,
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

use super::sku_dto::{CreateSkuRequest, UpdateSkuQuery, UpdateSkuRequest};

// ─── GET /api/v1/admin/payments/skus ────────────────────────────────────────

pub async fn list_skus_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::PaymentRead) {
        return err_response(e, req_ctx.request_id());
    }

    match state.sku_service.list_skus().await {
        Ok(skus) => ApiResponse::ok(serde_json::json!({ "skus": skus }), req_ctx.request_id())
            .into_response(),
        Err(e) => err_response(e, req_ctx.request_id()),
    }
}

// ─── POST /api/v1/admin/payments/skus ───────────────────────────────────────

pub async fn create_sku_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Json(req): Json<CreateSkuRequest>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::PaymentWrite) {
        return err_response(e, req_ctx.request_id());
    }

    let sku_id_clone;
    let resp = match state.sku_service.create_sku(req).await {
        Ok(r) => r,
        Err(e) => return err_response(e, req_ctx.request_id()),
    };
    sku_id_clone = resp.sku.sku_id.clone();

    // 审计日志（fire-and-forget）
    let logger = state.audit_logger.clone();
    let admin_id = ctx.admin_id;
    tokio::spawn(async move {
        logger
            .log_action(
                admin_id,
                "create_sku",
                Some("payment_sku"),
                None,
                None,
                Some(serde_json::json!({ "sku_id": sku_id_clone })),
            )
            .await;
    });

    ApiResponse::ok(resp, req_ctx.request_id()).into_response()
}

// ─── PUT /api/v1/admin/payments/skus/:sku_id ────────────────────────────────

pub async fn update_sku_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Path(sku_id): Path<String>,
    Query(query): Query<UpdateSkuQuery>,
    Json(req): Json<UpdateSkuRequest>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::PaymentWrite) {
        return err_response(e, req_ctx.request_id());
    }

    let sku = match state.sku_service.update_sku(&sku_id, req, query).await {
        Ok(s) => s,
        Err(e) => return err_response(e, req_ctx.request_id()),
    };

    // 审计日志（fire-and-forget）
    let logger = state.audit_logger.clone();
    let admin_id = ctx.admin_id;
    let sid = sku.sku_id.clone();
    tokio::spawn(async move {
        logger
            .log_action(
                admin_id,
                "update_sku",
                Some("payment_sku"),
                None,
                None,
                Some(serde_json::json!({ "sku_id": sid })),
            )
            .await;
    });

    ApiResponse::ok(serde_json::json!({ "sku": sku }), req_ctx.request_id()).into_response()
}

// ─── DELETE /api/v1/admin/payments/skus/:sku_id ─────────────────────────────

pub async fn delete_sku_handler(
    ctx: AdminAuthContext,
    Extension(req_ctx): Extension<RequestContext>,
    State(state): State<AppState>,
    Path(sku_id): Path<String>,
) -> Response {
    if let Err(e) = ctx.require_permission(Permission::PaymentWrite) {
        return err_response(e, req_ctx.request_id());
    }

    let sku = match state.sku_service.delete_sku(&sku_id).await {
        Ok(s) => s,
        Err(e) => return err_response(e, req_ctx.request_id()),
    };

    // 审计日志（fire-and-forget）
    let logger = state.audit_logger.clone();
    let admin_id = ctx.admin_id;
    let sid = sku.sku_id.clone();
    tokio::spawn(async move {
        logger
            .log_action(
                admin_id,
                "delete_sku",
                Some("payment_sku"),
                None,
                None,
                Some(serde_json::json!({ "sku_id": sid })),
            )
            .await;
    });

    ApiResponse::ok(
        serde_json::json!({ "sku": sku, "deleted": true }),
        req_ctx.request_id(),
    )
    .into_response()
}
