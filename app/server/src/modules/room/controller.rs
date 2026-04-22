use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Extension, Json};

use crate::{
    bootstrap::AppState,
    common::{auth::AuthContext, error::err_response, response::ApiResponse, RequestContext},
    ws::broadcaster::{broadcast_room_info_updated, RoomInfoUpdatedPayload},
};

use super::dto::{CreateRoomRequest, PatchRoomRequest, RoomListQuery};

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

    match state.room_service.patch_room(room_id, ctx.user_id, req).await {
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
            broadcast_room_info_updated(&state.ws_registry, &payload);

            (StatusCode::OK, Json(ApiResponse::ok(resp, rc.request_id()))).into_response()
        }
        Err(e) => err_response(e, rc.request_id()),
    }
}
