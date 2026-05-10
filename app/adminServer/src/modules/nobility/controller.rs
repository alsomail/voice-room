// PROTO-BINDING: doc/protocol/nobility_api.md §10.5 Admin REST
// Nobility admin HTTP handlers.
//
// Routes:
//   T-10030: GET/POST/PUT/DELETE /api/v1/admin/nobles/tiers
//   T-10031: POST /api/v1/admin/users/{id}/noble/grant
//            POST /api/v1/admin/users/{id}/noble/revoke
//   T-10032: GET /api/v1/admin/nobles/users
//            GET /api/v1/admin/nobles/users/{user_id}/history

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
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

use super::dto::{
    CreateTierRequest, GrantRequest, ListTiersQuery, ListUsersQuery, RevokeRequest,
    UpdateTierRequest,
};

// ─── T-10030: Tier CRUD ──────────────────────────────────────────────────────

/// GET /api/v1/admin/nobles/tiers
pub async fn list_tiers_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(query): Query<ListTiersQuery>,
) -> axum::response::Response {
    if let Err(e) = ctx.require_permission(Permission::NobleTierRead) {
        return err_response(e, rc.request_id());
    }
    match state
        .nobility_service
        .list_tiers(query.page, query.size)
        .await
    {
        Ok(resp) => ApiResponse::ok(resp, rc.request_id()).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/admin/nobles/tiers/{id}
pub async fn get_tier_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(tier_id): Path<String>,
) -> axum::response::Response {
    if let Err(e) = ctx.require_permission(Permission::NobleTierRead) {
        return err_response(e, rc.request_id());
    }
    match state.nobility_service.get_tier(&tier_id).await {
        Ok(resp) => ApiResponse::ok(resp, rc.request_id()).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// POST /api/v1/admin/nobles/tiers
pub async fn create_tier_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Json(body): Json<CreateTierRequest>,
) -> axum::response::Response {
    if let Err(e) = ctx.require_permission(Permission::NobleTierWrite) {
        return err_response(e, rc.request_id());
    }
    match state
        .nobility_service
        .create_tier(body, ctx.admin_id)
        .await
    {
        Ok(resp) => (StatusCode::CREATED, Json(ApiResponse::ok(resp, rc.request_id()))).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// PUT /api/v1/admin/nobles/tiers/{id}
pub async fn update_tier_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(tier_id): Path<String>,
    Json(body): Json<UpdateTierRequest>,
) -> axum::response::Response {
    if let Err(e) = ctx.require_permission(Permission::NobleTierWrite) {
        return err_response(e, rc.request_id());
    }
    match state
        .nobility_service
        .update_tier(&tier_id, body, ctx.admin_id)
        .await
    {
        Ok(resp) => ApiResponse::ok(resp, rc.request_id()).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// DELETE /api/v1/admin/nobles/tiers/{id} — 软删
pub async fn delete_tier_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(tier_id): Path<String>,
) -> axum::response::Response {
    if let Err(e) = ctx.require_permission(Permission::NobleTierWrite) {
        return err_response(e, rc.request_id());
    }
    match state
        .nobility_service
        .delete_tier(&tier_id, ctx.admin_id)
        .await
    {
        Ok(()) => ApiResponse::ok(serde_json::json!({}), rc.request_id()).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── T-10031: Grant/Revoke ───────────────────────────────────────────────────

/// POST /api/v1/admin/users/{id}/noble/grant
pub async fn grant_noble_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<GrantRequest>,
) -> axum::response::Response {
    // super_admin only
    if let Err(e) = ctx.require_role("super_admin") {
        return err_response(e, rc.request_id());
    }
    match state
        .nobility_service
        .grant_noble(user_id, body, ctx.admin_id)
        .await
    {
        Ok(resp) => ApiResponse::ok(serde_json::json!({ "user_noble": resp }), rc.request_id()).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// POST /api/v1/admin/users/{id}/noble/revoke
pub async fn revoke_noble_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<RevokeRequest>,
) -> axum::response::Response {
    // super_admin only
    if let Err(e) = ctx.require_role("super_admin") {
        return err_response(e, rc.request_id());
    }
    match state
        .nobility_service
        .revoke_noble(user_id, body, ctx.admin_id)
        .await
    {
        Ok(()) => ApiResponse::ok(serde_json::json!({}), rc.request_id()).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── T-10032: User Query ─────────────────────────────────────────────────────

/// GET /api/v1/admin/nobles/users
pub async fn list_noble_users_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(query): Query<ListUsersQuery>,
) -> axum::response::Response {
    if let Err(e) = ctx.require_permission(Permission::NobleTierRead) {
        return err_response(e, rc.request_id());
    }
    match state.nobility_service.list_noble_users(query).await {
        Ok(resp) => ApiResponse::ok(resp, rc.request_id()).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/admin/nobles/users/{user_id}/history
pub async fn get_noble_history_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(user_id): Path<Uuid>,
) -> axum::response::Response {
    if let Err(e) = ctx.require_permission(Permission::NobleTierRead) {
        return err_response(e, rc.request_id());
    }
    match state.nobility_service.get_noble_history(user_id).await {
        Ok(items) => {
            ApiResponse::ok(serde_json::json!({ "items": items }), rc.request_id()).into_response()
        }
        Err(e) => err_response(e, rc.request_id()),
    }
}

