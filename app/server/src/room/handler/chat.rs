//! SendMessage 信令处理
//!
//! 流程：解析 content → 长度校验 → 查连接房间 → 查房间状态 →
//! 禁言检查 → 幂等去重 → 敏感词过滤+广播 → 返回结果

use std::sync::Arc;

use uuid::Uuid;

use crate::modules::chat::ChatRepository;
use crate::modules::governance::mute::MuteRedis;
use crate::room::manager::RoomManager;
use crate::ws::registry::ConnectionRegistry;

// ─── SendMessageDeps ──────────────────────────────────────────────────────────

/// `handle_send_message` 所需的服务依赖。
pub struct SendMessageDeps {
    pub room_manager: Arc<RoomManager>,
    pub registry: Arc<ConnectionRegistry>,
    /// 禁言 Redis（T-00029 前置拦截）；None = 跳过拦截
    pub mute_redis: Option<Arc<dyn MuteRedis>>,
    /// 聊天消息持久化（T-00043）；None = 跳过持久化（兼容旧测试）
    pub chat_repo: Option<Arc<dyn ChatRepository>>,
}

// ─── handle_send_message ──────────────────────────────────────────────────────

/// 处理 SendMessage 信令，返回 JSON 字符串响应。
///
/// 8 步流程：解析 content → 长度校验 → 查连接房间 → 查房间状态 →
///          禁言检查 → 幂等去重 → 敏感词过滤+广播 → 返回结果
pub async fn handle_send_message(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    connection_id: Uuid,
    user_id: Uuid,
    deps: &SendMessageDeps,
) -> String {
    use crate::room::filter::filter_content;

    // ── 1. 解析 payload.content，空值/缺失 → code:40002 ──────────────────────
    let content = match payload
        .as_ref()
        .and_then(|p| p.get("content"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
    {
        Some(c) => c,
        None => {
            return send_message_error_response(
                msg_id,
                40002,
                "content is required and must not be empty",
            );
        }
    };

    // ── 2. 长度校验（Unicode chars）：> 500 → code:40001 ─────────────────────
    if content.chars().count() > 500 {
        return send_message_error_response(msg_id, 40001, "message exceeds 500 characters");
    }

    // ── 3. 获取连接所在房间 ───────────────────────────────────────────────────
    let room_id = match deps.registry.get_room_id(connection_id) {
        Some(id) => id,
        None => {
            return send_message_error_response(msg_id, 40400, "user not in room");
        }
    };

    // ── 3.5 禁言 Redis 前置拦截（T-00029）────────────────────────────────────
    // MU29-04: 被禁言用户 SendMessage → 40305 CHAT_MUTED
    if let Some(ref mr) = deps.mute_redis {
        match mr.get_mute_ttl("chat", room_id, user_id).await {
            Ok(Some(_)) => {
                return send_message_error_response(msg_id, 40305, "user is chat-muted");
            }
            Ok(None) => {} // 未被禁言，继续
            Err(e) => {
                tracing::warn!("chat_muted check failed: {e}");
                // 非阻断性，继续
            }
        }
    }

    // ── 4. 获取房间状态（防御性检查）─────────────────────────────────────────
    let room_state = match deps.room_manager.get_room(room_id) {
        Some(s) => s,
        None => {
            return send_message_error_response(msg_id, 40400, "room not found");
        }
    };

    // ── 5. 禁言检查 ───────────────────────────────────────────────────────────
    if room_state.muted_users.contains(&user_id) {
        return send_message_error_response(msg_id, 40303, "user is muted");
    }

    // ── 6. 幂等去重：msg_id 已处理则直接返回 code:0，不广播 ───────────────────
    let msg_id_str = msg_id.as_deref().unwrap_or("").to_string();
    if !msg_id_str.is_empty() && room_state.processed_msg_ids.contains(&msg_id_str) {
        return send_message_success_response(msg_id);
    }

    // ── 7. 记录 msg_id + 敏感词过滤 ──────────────────────────────────────────
    if !msg_id_str.is_empty() {
        room_state.processed_msg_ids.insert(msg_id_str.clone());
    }
    let filtered_content = filter_content(&content);

    // ── 7.5 持久化到 DB（T-00043）─────────────────────────────────────────────
    // 使用过滤后的 content（与广播一致，便于审计与 REST 历史回放视图相符）。
    // 失败 → 50000 + 不广播；保证 DB / 广播状态一致。
    let mut db_message_id: Option<String> = None;
    if let Some(ref repo) = deps.chat_repo {
        match repo.insert_message(room_id, user_id, &filtered_content).await {
            Ok(id) => {
                db_message_id = Some(id.to_string());
            }
            Err(e) => {
                tracing::error!(
                    room_id = %room_id,
                    user_id = %user_id,
                    error = %e,
                    "chat_messages insert failed"
                );
                return send_message_error_response(
                    msg_id,
                    50000,
                    "chat persistence failed, please retry",
                );
            }
        }
    }

    // ── 8. 广播 RoomMessage（payload.msg_id 优先使用 DB id，T-00043 U-2）
    let broadcast_payload_msg_id = db_message_id
        .clone()
        .unwrap_or_else(|| msg_id_str.clone());
    let room_msg_envelope = serde_json::json!({
        "type": "RoomMessage",
        "payload": {
            "msg_id": broadcast_payload_msg_id,
            "user_id": user_id.to_string(),
            "content": filtered_content,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    crate::ws::broadcaster::broadcast_to_room(&deps.registry, &room_state, room_msg_envelope);

    // ── 9. 返回 SendMessageResult { code:0 } ─────────────────────────────────
    send_message_success_response(msg_id)
}

fn send_message_error_response(msg_id: Option<String>, code: i64, message: &str) -> String {
    let resp = serde_json::json!({
        "type": "SendMessageResult",
        "msg_id": msg_id,
        "code": code,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

fn send_message_success_response(msg_id: Option<String>) -> String {
    let resp = serde_json::json!({
        "type": "SendMessageResult",
        "msg_id": msg_id,
        "code": 0,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}
