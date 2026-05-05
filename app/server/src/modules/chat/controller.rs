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

use super::dto::{
    normalize_pagination, ChatMessageItem, ChatMessagesResponse, MessagesQuery,
    SendChatMessageRequest, SendChatMessageResponse, MAX_CONTENT_CHARS,
};

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

/// `POST /api/v1/chat-messages` — REST 写入聊天消息并广播（T-00045 BUG-CHAT-WS-BROADCAST）。
///
/// 流程：
/// 1. JWT 鉴权（`AuthContext`）
/// 2. 解析 / 校验 `room_id`（UUID）与 `content`（1..=500 chars）
/// 3. `chat_repo.insert_message` — DB 事务提交后才广播
/// 4. 通过 `ws::broadcaster::broadcast_to_room` 向房间内所有 WS 连接广播 `RoomMessage` envelope
///    （envelope-level msg_id 由广播器统一注入 UUID v4；payload.msg_id = DB row id）
/// 5. 房间不在内存（无 `RoomState`）时降级 `broadcast_to_room_no_state`，不写 recent_broadcasts
/// 6. 单连接 send 失败由 broadcaster 内部容忍（`let _ = sender.send(...)`），REST 不感知
pub async fn send_chat_message_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    ctx: AuthContext,
    Json(req): Json<SendChatMessageRequest>,
) -> axum::response::Response {
    // ── 1. 校验 room_id ───────────────────────────────────────────────────────
    let room_id = match uuid::Uuid::parse_str(&req.room_id) {
        Ok(u) => u,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid room id format: {:?}", req.room_id)),
                rc.request_id(),
            );
        }
    };

    // ── 2. 校验 content（1..=500 chars，按 Unicode chars，与 WS 路径一致）───
    let content_len = req.content.chars().count();
    if content_len == 0 {
        return err_response(
            AppError::ValidationError("content is required and must not be empty".to_string()),
            rc.request_id(),
        );
    }
    if content_len > MAX_CONTENT_CHARS {
        return err_response(
            AppError::ValidationError(format!(
                "message exceeds {} characters",
                MAX_CONTENT_CHARS
            )),
            rc.request_id(),
        );
    }

    // ── 3. INSERT（DB 事务在此提交）───────────────────────────────────────────
    let message_id = match state
        .chat_repo
        .insert_message(room_id, ctx.user_id, &req.content)
        .await
    {
        Ok(id) => id,
        Err(e) => return err_response(e, rc.request_id()),
    };

    // ── 4. 广播 RoomMessage envelope（与 WS SendMessage 路径形态一致）────────
    let envelope = serde_json::json!({
        "type": "RoomMessage",
        "payload": {
            "msg_id": message_id.to_string(),
            "user_id": ctx.user_id.to_string(),
            "content": req.content,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });

    if let Some(rs) = state.room_manager.get_room(room_id) {
        crate::ws::broadcaster::broadcast_to_room(&state.ws_registry, &rs, envelope);
    } else {
        // 房间未 JoinRoom 注册：降级广播（不写 recent_broadcasts，无续传支持）
        crate::ws::broadcaster::broadcast_to_room_no_state(
            &state.ws_registry,
            room_id,
            envelope,
        );
    }

    // ── 5. 返回 ──────────────────────────────────────────────────────────────
    (
        axum::http::StatusCode::OK,
        Json(ApiResponse::ok(
            SendChatMessageResponse { msg_id: message_id },
            rc.request_id(),
        )),
    )
        .into_response()
}
