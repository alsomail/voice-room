use axum::{extract::State, Extension, Json};

use crate::{
    bootstrap::AppState,
    common::{auth::AuthContext, error::AppError, response::ApiResponse, RequestContext},
};

use super::dto::{LoginRequest, LoginResponse, SendCodeRequest, SendCodeResponse, UserResponse};

/// POST /api/v1/auth/verification-codes
pub async fn send_code(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Json(req): Json<SendCodeRequest>,
) -> Result<Json<ApiResponse<SendCodeResponse>>, AppError> {
    let resp = state.auth_service.send_code(&req.phone).await?;
    Ok(Json(ApiResponse::ok(resp, rc.request_id())))
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let resp = state.auth_service.login(&req.phone, &req.code).await?;
    Ok(Json(ApiResponse::ok(resp, rc.request_id())))
}

/// GET /api/v1/users/me（需要 JWT 鉴权）
pub async fn get_me(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    let resp = state.auth_service.get_me(ctx.user_id).await?;
    Ok(Json(ApiResponse::ok(resp, rc.request_id())))
}
