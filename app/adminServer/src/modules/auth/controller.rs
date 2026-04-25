use axum::{extract::State, response::IntoResponse, Extension, Json};

use crate::{
    bootstrap::AppState,
    common::{error::err_response, response::ApiResponse, RequestContext},
    infrastructure::logging::extract_client_ip,
};

use super::dto::AdminLoginRequest;

/// POST /api/v1/admin/login
///
/// 管理员账号密码登录接口（无需鉴权）。
/// 成功返回 JWT（有效期 7 天，含 admin_id, role）及管理员基础信息。
pub async fn login_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AdminLoginRequest>,
) -> axum::response::Response {
    let ip_addr = extract_client_ip(&headers);

    match state
        .auth_service
        .login(&req.username, &req.password, ip_addr)
        .await
    {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}
