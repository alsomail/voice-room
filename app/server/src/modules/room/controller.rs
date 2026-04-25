use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Serialize;

use crate::{
    bootstrap::AppState,
    common::{
        auth::AuthContext, error::err_response, error::AppError, response::ApiResponse,
        RequestContext,
    },
    modules::room::password::{verify_password, VerifyPasswordResult},
    ws::broadcaster::{broadcast_room_info_updated, RoomInfoUpdatedPayload},
};

use super::dto::{
    CreateRoomRequest, LockedData, PatchRoomRequest, RoomListQuery, VerifyPasswordRequest,
    VerifyPasswordResponse, WrongPasswordData,
};

/// POST /api/v1/rooms（需要 JWT 鉴权）
///
/// 成功：HTTP 201 + CreateRoomResponse
/// 失败：HTTP 400 / 401 / 409 + 错误体
pub async fn create_room(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Json(req): Json<CreateRoomRequest>,
) -> axum::response::Response {
    match state.room_service.create_room(ctx.user_id, req).await {
        Ok(resp) => (
            StatusCode::CREATED,
            Json(ApiResponse::ok(resp, rc.request_id())),
        )
            .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/rooms（无需鉴权）
///
/// 成功：HTTP 200 + RoomListResponse
/// 失败：HTTP 400 + 错误体
pub async fn list_rooms(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(query): Query<RoomListQuery>,
) -> axum::response::Response {
    match state.room_service.list_rooms(query).await {
        Ok(resp) => (StatusCode::OK, Json(ApiResponse::ok(resp, rc.request_id()))).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/rooms/:id（无需鉴权）
///
/// 成功：HTTP 200 + RoomDetailResponse
/// 失败：HTTP 400 (非法 UUID) / 404 (房间不存在) + 错误体
pub async fn get_room(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Path(id): Path<String>,
) -> axum::response::Response {
    // 手动解析 UUID，非法格式返回 ValidationError 400
    let room_id = match uuid::Uuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return err_response(
                crate::common::error::AppError::ValidationError(format!(
                    "invalid room id format: {id:?}"
                )),
                rc.request_id(),
            );
        }
    };

    match state.room_service.get_room_detail(room_id).await {
        Ok(resp) => (StatusCode::OK, Json(ApiResponse::ok(resp, rc.request_id()))).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// DELETE /api/v1/rooms/:id（需要 JWT 鉴权）
///
/// 成功：HTTP 200 + data: null
/// 失败：HTTP 400 (非法 UUID) / 401 (无/过期 token) / 403 (非房主) / 404 / 409 (已关闭) + 错误体
pub async fn close_room(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Path(id): Path<String>,
) -> axum::response::Response {
    // 手动解析 UUID，非法格式返回 ValidationError 400
    let room_id = match uuid::Uuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return err_response(
                crate::common::error::AppError::ValidationError(format!(
                    "invalid room id format: {id:?}"
                )),
                rc.request_id(),
            );
        }
    };

    match state.room_service.close_room(room_id, ctx.user_id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(ApiResponse::ok(serde_json::Value::Null, rc.request_id())),
        )
            .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// PATCH /api/v1/rooms/:id（需要 JWT 鉴权）— T-00025 新增
///
/// 仅房主可修改；修改成功后向房间内所有连接广播 `RoomInfoUpdated`。
///
/// 成功：HTTP 200 + PatchRoomResponse
/// 失败：HTTP 400 / 401 / 403 / 404 / 409 + 错误体
pub async fn patch_room(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Path(id): Path<String>,
    Json(req): Json<PatchRoomRequest>,
) -> axum::response::Response {
    // 手动解析 UUID，非法格式返回 ValidationError 400
    let room_id = match uuid::Uuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return err_response(
                crate::common::error::AppError::ValidationError(format!(
                    "invalid room id format: {id:?}"
                )),
                rc.request_id(),
            );
        }
    };

    match state
        .room_service
        .patch_room(room_id, ctx.user_id, req)
        .await
    {
        Ok(resp) => {
            // 广播 RoomInfoUpdated 到房间内所有 WS 连接
            let payload = RoomInfoUpdatedPayload {
                room_id: resp.room_id.clone(),
                title: resp.title.clone(),
                announcement: resp.announcement.clone(),
                category: resp.category.clone(),
                cover_url: resp.cover_url.clone(),
                has_password: resp.has_password,
            };
            broadcast_room_info_updated(&state.ws_registry, &state.room_manager, &payload);

            (StatusCode::OK, Json(ApiResponse::ok(resp, rc.request_id()))).into_response()
        }
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── 自定义错误响应构建（含 data 字段）──────────────────────────────────────────

#[derive(Serialize)]
struct ErrorWithData<T: Serialize> {
    code: i32,
    message: String,
    data: T,
    request_id: String,
}

fn error_with_data<T: Serialize>(
    status: StatusCode,
    code: i32,
    message: &str,
    data: T,
    request_id: &str,
) -> axum::response::Response {
    let body = ErrorWithData {
        code,
        message: message.to_string(),
        data,
        request_id: request_id.to_string(),
    };
    (status, Json(body)).into_response()
}

/// POST /api/v1/rooms/:id/verify-password（需要 JWT 鉴权）— T-00026
///
/// 成功：HTTP 200 + { access_token: "<jwt-60s>" }
/// 错误：
/// - 400/40003 密码格式非 6 位数字
/// - 404/40400 房间不存在或已关闭
/// - 400/40014 非密码房
/// - 401/40103 密码错误 + remaining_attempts
/// - 429/42910 已锁定 + locked_remaining_sec
pub async fn verify_password_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Path(id): Path<String>,
    Json(req): Json<VerifyPasswordRequest>,
) -> axum::response::Response {
    // ── 1. 解析 room_id ──────────────────────────────────────────────────────
    let room_id = match uuid::Uuid::parse_str(&id) {
        Ok(uid) => uid,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid room id format: {id:?}")),
                rc.request_id(),
            );
        }
    };

    // ── 2. 验证密码格式（6 位数字）──────────────────────────────────────────
    if let Err(e) = super::validator::validate_password(&req.password) {
        return err_response(e, rc.request_id());
    }

    // ── 3. 获取房间（含 password_hash）──────────────────────────────────────
    let room = match state.room_service.get_active_room_model(room_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return err_response(
                AppError::NotFound(format!("room {room_id}")),
                rc.request_id(),
            );
        }
        Err(e) => return err_response(e, rc.request_id()),
    };

    // ── 4. 校验是密码房 ──────────────────────────────────────────────────────
    if room.room_type != "password" {
        return err_response(AppError::NotPasswordRoom, rc.request_id());
    }

    // ── 5. 密码校验 + 锁定逻辑（通过 Redis 抽象）────────────────────────────
    match verify_password(
        &room,
        &req.password,
        ctx.user_id,
        &*state.room_password_redis,
        &state.jwt_secret,
    )
    .await
    {
        Err(e) => err_response(e, rc.request_id()),

        Ok(VerifyPasswordResult::Token(jwt)) => (
            StatusCode::OK,
            Json(ApiResponse::ok(
                VerifyPasswordResponse { access_token: jwt },
                rc.request_id(),
            )),
        )
            .into_response(),

        Ok(VerifyPasswordResult::WrongPassword { remaining_attempts }) => error_with_data(
            StatusCode::UNAUTHORIZED,
            40103,
            "wrong password",
            WrongPasswordData { remaining_attempts },
            rc.request_id(),
        ),

        Ok(VerifyPasswordResult::Locked { remaining_sec }) => error_with_data(
            StatusCode::TOO_MANY_REQUESTS,
            42910,
            "too many wrong attempts, locked",
            LockedData {
                locked_remaining_sec: remaining_sec,
            },
            rc.request_id(),
        ),
    }
}
