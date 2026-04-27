//! 历史消息查询 controller — `GET /api/v1/rooms/:room_id/messages`（T-00043）
//!
//! 鉴权：JWT（`AuthContext` extractor）
//! 分页：`?limit=&offset=`，默认 50 / 上限 100
//! 排序：`created_at DESC`

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};

use crate::{
    bootstrap::AppState,
    common::{
        auth::AuthContext,
        error::{err_response, AppError},
        response::ApiResponse,
        RequestContext,
    },
};

use super::dto::{normalize_pagination, ChatMessageItem, ChatMessagesResponse, MessagesQuery};

/// `GET /api/v1/rooms/:room_id/messages`
pub async fn list_room_messages_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    _ctx: AuthContext,
    Path(room_id_str): Path<String>,
    Query(query): Query<MessagesQuery>,
) -> axum::response::Response {
    let room_id = match uuid::Uuid::parse_str(&room_id_str) {
        Ok(u) => u,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid room id format: {room_id_str:?}")),
                rc.request_id(),
            );
        }
    };

    let (limit, offset) = normalize_pagination(&query);

    match state.chat_repo.list_messages(room_id, limit, offset).await {
        Ok((rows, total)) => {
            let resp = ChatMessagesResponse {
                items: rows.into_iter().map(ChatMessageItem::from).collect(),
                total,
                limit,
                offset,
            };
            (
                axum::http::StatusCode::OK,
                Json(ApiResponse::ok(resp, rc.request_id())),
            )
                .into_response()
        }
        Err(e) => err_response(e, rc.request_id()),
    }
}
