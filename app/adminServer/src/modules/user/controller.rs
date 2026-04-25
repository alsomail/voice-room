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

use super::dto::{AdminBanUserRequest, AdminUserListQuery};

/// GET /api/v1/admin/users
///
/// 管理员用户列表接口，需要 UserRead 权限。
/// - super_admin / operator / cs 可访问
/// - finance 角色被拒绝（403/40301）
pub async fn list_users_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(query): Query<AdminUserListQuery>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::UserRead) {
        return err_response(e, rc.request_id());
    }

    match state.user_service.list_users(query).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/admin/users/:id
///
/// 管理员用户详情接口，需要 UserRead 权限。
/// - super_admin / operator / cs 可访问
/// - finance 角色被拒绝（403/40301）
/// - 用户不存在或已软删除返回 404/40401
/// - 路径参数 id 必须是合法 UUID，否则返回 400/40003
pub async fn get_user_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(id): Path<String>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::UserRead) {
        return err_response(e, rc.request_id());
    }

    // 手动解析 UUID（路径参数为字符串，解析失败返回 ValidationError）
    let user_id = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid user id: '{}'", id)),
                rc.request_id(),
            );
        }
    };

    match state.user_service.get_user_detail(user_id).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// POST /api/v1/admin/users/:id/ban
///
/// 封禁/解封用户接口，需要 UserWrite 权限。
/// - super_admin / operator 可访问
/// - cs / finance 角色被拒绝（403/40301）
/// - 路径参数 id 必须是合法 UUID，否则返回 400/40003
/// - action 必须是 "ban" 或 "unban"，否则返回 400/40003
/// - 用户不存在或已软删除返回 404/40401
/// - 幂等冲突（已封禁再次 ban / 已正常再次 unban）返回 409/40900
pub async fn ban_user_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<AdminBanUserRequest>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::UserWrite) {
        return err_response(e, rc.request_id());
    }

    // 手动解析 UUID
    let user_id = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid user id: '{}'", id)),
                rc.request_id(),
            );
        }
    };

    // action 校验
    if req.action != "ban" && req.action != "unban" {
        return err_response(
            AppError::ValidationError(format!(
                "invalid action: '{}', must be 'ban' or 'unban'",
                req.action
            )),
            rc.request_id(),
        );
    }

    // 在移交所有权前捕获需要的字段
    let action_str = if req.action == "ban" {
        "ban_user"
    } else {
        "unban_user"
    };
    let ip = extract_ip(&headers);

    // P1-7: 提前捕获 ban 详情字段供审计 detail 使用
    let audit_detail = serde_json::json!({
        "action": req.action,
        "ban_type": req.ban_type,
        "duration_hours": req.duration_hours,
        "reason": req.reason,
    });

    let result = state
        .user_service
        .ban_user(ctx.admin_id, user_id, req)
        .await;

    // 业务成功后写入审计日志（fire-and-forget：失败仅 warn，不影响响应）
    if result.is_ok() {
        state
            .audit_logger
            .log_action(
                ctx.admin_id,
                action_str,
                Some("user"),
                Some(user_id),
                ip,
                Some(audit_detail),
            )
            .await;
    }

    match result {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}
