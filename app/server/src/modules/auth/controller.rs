use axum::{extract::State, response::IntoResponse, Extension, Json};

use crate::{
    bootstrap::AppState,
    common::{auth::AuthContext, error::err_response, response::ApiResponse, RequestContext},
};

use super::dto::{LoginRequest, SendCodeRequest};

/// POST /api/v1/auth/verification-codes
pub async fn send_code(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Json(req): Json<SendCodeRequest>,
) -> axum::response::Response {
    match state.auth_service.send_code(&req.phone).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Json(req): Json<LoginRequest>,
) -> axum::response::Response {
    match state.auth_service.login(&req.phone, &req.code).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/users/me（需要 JWT 鉴权）
pub async fn get_me(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
) -> axum::response::Response {
    match state.auth_service.get_me(ctx.user_id).await {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}
