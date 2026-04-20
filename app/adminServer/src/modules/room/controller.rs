use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension, Json,
};

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::{err_response, AppError},
        response::ApiResponse,
        RequestContext,
    },
    modules::audit::controller::extract_ip,
};

use super::dto::AdminRoomListQuery;

/// GET /api/v1/admin/rooms
///
/// 管理员房间列表接口，需要 RoomRead 权限（operator / cs / super_admin 可访问，
/// finance 角色被拒绝）。
pub async fn list_rooms_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(query): Query<AdminRoomListQuery>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::RoomRead) {
        return err_response(e, rc.request_id());
    }

    match state.room_service.list(query).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/admin/rooms/:id
///
/// 管理员房间详情接口，需要 RoomRead 权限。
/// - 不过滤 status（closed 房间也返回 200）
/// - 软删除或不存在的房间返回 404/40400
/// - 路径参数 id 必须是合法 UUID，否则返回 400/40003
pub async fn get_room_detail_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(id): Path<String>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::RoomRead) {
        return err_response(e, rc.request_id());
    }

    // 手动解析 UUID（路径参数为字符串，解析失败返回 ValidationError）
    let room_id = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid room id: '{}'", id)),
                rc.request_id(),
            );
        }
    };

    match state.room_service.get_room_detail(room_id).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// DELETE /api/v1/admin/rooms/:id
///
/// 管理员强制关闭房间接口，需要 RoomForceClose 权限（operator / super_admin 可访问，
/// cs / finance 角色被拒绝）。
///
/// - 不做 owner 校验（管理员可关闭任何人的房间）
/// - 房间不存在（含软删除）返回 404/40400
/// - 房间已 closed 返回 409/40901
/// - 成功返回 200 + data:null
pub async fn force_close_room_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::RoomForceClose) {
        return err_response(e, rc.request_id());
    }

    // 手动解析 UUID（路径参数为字符串，解析失败返回 ValidationError）
    let room_id = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid room id: '{}'", id)),
                rc.request_id(),
            );
        }
    };

    let ip = extract_ip(&headers);
    let result = state.room_service.force_close_room(ctx.admin_id, room_id).await;

    // 业务成功后写入审计日志（fire-and-forget：失败仅 warn，不影响响应）
    if result.is_ok() {
        state
            .audit_logger
            .log_action(
                ctx.admin_id,
                "close_room",
                Some("room"),
                Some(room_id),
                ip,
                None,
            )
            .await;
    }

    match result {
        Ok(()) => {
            Json(ApiResponse::ok(serde_json::Value::Null, rc.request_id())).into_response()
        }
        Err(e) => err_response(e, rc.request_id()),
    }
}
