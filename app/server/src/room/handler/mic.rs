//! TakeMic / LeaveMic 信令处理 + broadcast_mic_left 共享广播工具
//!
//! 职责：处理上下麦信令（含麦位锁、禁麦前置拦截、Forced 字段）+ 提供
//! `broadcast_mic_left` 给 lifecycle / governance 共用。

use std::sync::Arc;

use uuid::Uuid;

use crate::modules::governance::mute::MuteRedis;
use crate::room::manager::RoomManager;
use crate::room::state::RoomState;
use crate::ws::registry::ConnectionRegistry;

// ─── TakeMicDeps ─────────────────────────────────────────────────────────────

/// `handle_take_mic` 所需的依赖。
///
/// 上麦无需验证房间 active 状态，也不需要用户详情，比 `JoinRoomDeps` 更轻量。
pub struct TakeMicDeps {
    pub room_manager: Arc<RoomManager>,
    pub registry: Arc<ConnectionRegistry>,
    /// 禁麦 Redis（T-00029 前置拦截）；None = 跳过拦截
    pub mute_redis: Option<Arc<dyn MuteRedis>>,
}

// ─── handle_take_mic ─────────────────────────────────────────────────────────

/// 处理 TakeMic 信令，返回 JSON 字符串响应。
///
/// 7 步流程：解析参数 → 查连接房间 → 查房间状态 → 禁麦检查 → 原子占位 → 广播 → 返回结果
pub async fn handle_take_mic(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    connection_id: Uuid,
    user_id: Uuid,
    deps: &TakeMicDeps,
) -> String {
    use crate::room::state::TakeMicError;

    // ── 1. 解析并校验 mic_index（0-8）────────────────────────────────────────
    let mic_index = match payload
        .as_ref()
        .and_then(|p| p.get("mic_index"))
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .filter(|&i| i <= 8)
    {
        Some(i) => i,
        None => {
            return take_mic_error_response(msg_id, 40002, "invalid or missing mic_index");
        }
    };

    // ── 2. 获取连接所在房间 ───────────────────────────────────────────────────
    let room_id = match deps.registry.get_room_id(connection_id) {
        Some(id) => id,
        None => {
            return take_mic_error_response(msg_id, 40400, "user not in room");
        }
    };

    // ── 2.5 禁麦 Redis 前置拦截（T-00029）────────────────────────────────────
    // MU29-03: 被禁麦用户 TakeMic → 40306 MIC_MUTED
    if let Some(ref mr) = deps.mute_redis {
        match mr.get_mute_ttl("mic", room_id, user_id).await {
            Ok(Some(_)) => {
                return take_mic_error_response(msg_id, 40306, "user is mic-muted");
            }
            Ok(None) => {} // 未被禁麦，继续
            Err(e) => {
                tracing::warn!("mic_muted check failed: {e}");
                // 非阻断性，继续
            }
        }
    }

    // ── 3. 获取房间状态（防御性检查）─────────────────────────────────────────
    let room_state = match deps.room_manager.get_room(room_id) {
        Some(s) => s,
        None => {
            return take_mic_error_response(msg_id, 40400, "room not found");
        }
    };

    // ── 4. 禁麦检查 ───────────────────────────────────────────────────────────
    if room_state.banned_mics.contains(&user_id) {
        return take_mic_error_response(msg_id, 40302, "user is banned from mic");
    }

    // ── 5. 原子占用麦位 ───────────────────────────────────────────────────────
    match room_state.take_mic_slot(mic_index, user_id) {
        Ok(()) => {}
        Err(TakeMicError::AlreadyOnMic) => {
            return take_mic_error_response(msg_id, 40301, "user already on mic");
        }
        Err(TakeMicError::SlotOccupied) => {
            return take_mic_error_response(msg_id, 40303, "slot already occupied");
        }
    }

    // ── 6. 广播 MicTaken 给房间内所有连接（含请求方）— 走统一出口 broadcast_to_room
    let mic_taken_envelope = serde_json::json!({
        "type": "MicTaken",
        "payload": {
            "mic_index": mic_index,
            "user_id": user_id.to_string(),
        },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    crate::ws::broadcaster::broadcast_to_room(&deps.registry, &room_state, mic_taken_envelope);

    // ── 7. 返回 TakeMicResult ────────────────────────────────────────────────
    let resp = serde_json::json!({
        "type": "TakeMicResult",
        "msg_id": msg_id,
        "code": 0,
        "payload": { "mic_index": mic_index },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

fn take_mic_error_response(msg_id: Option<String>, code: i64, message: &str) -> String {
    let resp = serde_json::json!({
        "type": "TakeMicResult",
        "msg_id": msg_id,
        "code": code,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

// ─── LeaveMicDeps ─────────────────────────────────────────────────────────────

/// `handle_leave_mic` 所需的 2 个服务依赖。
///
/// 与 `TakeMicDeps` 同构：下麦无需验证房间 active 状态，也不需要用户详情。
pub struct LeaveMicDeps {
    pub room_manager: Arc<RoomManager>,
    pub registry: Arc<ConnectionRegistry>,
}

// ─── handle_leave_mic ────────────────────────────────────────────────────────

/// 处理 LeaveMic 信令，返回 JSON 字符串响应。
///
/// 5 步流程：查连接房间 → 查房间状态 → 原子下麦 → 广播 MicLeft → 返回结果
pub async fn handle_leave_mic(
    msg_id: Option<String>,
    connection_id: Uuid,
    user_id: Uuid,
    deps: &LeaveMicDeps,
) -> String {
    // ── 1. 获取连接所在房间 ───────────────────────────────────────────────────
    let room_id = match deps.registry.get_room_id(connection_id) {
        Some(id) => id,
        None => {
            return leave_mic_error_response(msg_id, 40400, "user not in room");
        }
    };

    // ── 2. 获取房间状态（防御性检查）─────────────────────────────────────────
    let room_state = match deps.room_manager.get_room(room_id) {
        Some(s) => s,
        None => {
            return leave_mic_error_response(msg_id, 40400, "room not found");
        }
    };

    // ── 3. 原子查找并清除麦位 ─────────────────────────────────────────────────
    let mic_index = match room_state.leave_mic_slot(user_id) {
        Some(idx) => idx,
        None => {
            return leave_mic_error_response(msg_id, 40304, "user not on mic");
        }
    };

    // ── 4. 广播 MicLeft 给房间内所有连接（含请求方）— 走统一出口 broadcast_to_room
    broadcast_mic_left(&deps.registry, &room_state, mic_index, user_id, false);

    // ── 5. 返回 LeaveMicResult ────────────────────────────────────────────────
    let resp = serde_json::json!({
        "type": "LeaveMicResult",
        "msg_id": msg_id,
        "code": 0,
        "payload": { "mic_index": mic_index },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

fn leave_mic_error_response(msg_id: Option<String>, code: i64, message: &str) -> String {
    let resp = serde_json::json!({
        "type": "LeaveMicResult",
        "msg_id": msg_id,
        "code": code,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

/// 广播 MicLeft 给房间内所有连接（T-00028 新增 forced 字段）— 走统一出口 broadcast_to_room
///
/// - `forced = false`：主动下麦（LeaveRoom / LeaveMic）
/// - `forced = true`：被踢下麦（KickUser）
pub(crate) fn broadcast_mic_left(
    registry: &ConnectionRegistry,
    room_state: &RoomState,
    mic_index: usize,
    user_id: Uuid,
    forced: bool,
) {
    let envelope = serde_json::json!({
        "type": "MicLeft",
        "payload": {
            "mic_index": mic_index,
            "user_id": user_id.to_string(),
            "forced": forced,
        },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    crate::ws::broadcaster::broadcast_to_room(registry, room_state, envelope);
}
