//! ForceTakeMic / ForceLeaveMic 信令处理 — T-00030
//!
//! ## ForceTakeMic 处理流程
//! 1. 解析 payload（target_user_id / mic_index）；room_id 来自 session context
//! 2. 加载房间 Model → 权限校验（owner 或 admin）
//! 3. 检查 target 是否被禁麦（mic_muted Redis key）→ 40306
//! 4. 获取 RoomState → 原子占用麦位
//! 5. 广播 MicTaken { forced_by: operator_id }
//!
//! ## ForceLeaveMic 处理流程
//! 1. 解析 payload（target_user_id）；room_id 来自 session context
//! 2. 加载房间 Model → 权限校验（owner 或 admin）
//! 3. 管理员不能抱下房主 → 40302
//! 4. 获取 RoomState → 原子查找并清除麦位（→ 40404 若不在麦）
//! 5. 广播 MicLeft { forced: true, forced_by: operator_id }

use std::sync::Arc;

use uuid::Uuid;

use crate::modules::governance::mute::MuteRedis;
use crate::modules::room::service::RoomService;
use crate::room::manager::RoomManager;
use crate::room::state::TakeMicError;
use crate::ws::registry::ConnectionRegistry;

// ─── ForceTakeMicDeps ─────────────────────────────────────────────────────────

/// `handle_force_take_mic` 所需的全部服务依赖。
pub struct ForceTakeMicDeps {
    /// 房间运行时状态管理器（麦位操作）
    pub room_manager: Arc<RoomManager>,
    /// 房间服务（权限校验：owner_id + admin_user_id）
    pub room_service: Arc<RoomService>,
    /// 禁麦 Redis（检查 target 是否被禁麦）
    pub mute_redis: Arc<dyn MuteRedis>,
    /// WS 连接注册表（广播 MicTaken）
    pub registry: Arc<ConnectionRegistry>,
}

// ─── ForceLeaveMicDeps ────────────────────────────────────────────────────────

/// `handle_force_leave_mic` 所需的全部服务依赖。
pub struct ForceLeaveMicDeps {
    /// 房间运行时状态管理器（麦位操作）
    pub room_manager: Arc<RoomManager>,
    /// 房间服务（权限校验：owner_id + admin_user_id）
    pub room_service: Arc<RoomService>,
    /// WS 连接注册表（广播 MicLeft）
    pub registry: Arc<ConnectionRegistry>,
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

fn force_take_error(msg_id: Option<String>, code: i64, message: &str) -> String {
    crate::ws::broadcaster::build_outbound_result(
        "ForceTakeMicResult",
        msg_id,
        code,
        Some(serde_json::json!({ "message": message })),
    )
}

fn force_take_success(msg_id: Option<String>, mic_index: usize) -> String {
    crate::ws::broadcaster::build_outbound_result(
        "ForceTakeMicResult",
        msg_id,
        0,
        Some(serde_json::json!({ "mic_index": mic_index })),
    )
}

fn force_leave_error(msg_id: Option<String>, code: i64, message: &str) -> String {
    crate::ws::broadcaster::build_outbound_result(
        "ForceLeaveMicResult",
        msg_id,
        code,
        Some(serde_json::json!({ "message": message })),
    )
}

fn force_leave_success(msg_id: Option<String>, mic_index: usize) -> String {
    crate::ws::broadcaster::build_outbound_result(
        "ForceLeaveMicResult",
        msg_id,
        0,
        Some(serde_json::json!({ "mic_index": mic_index })),
    )
}

// ─── handle_force_take_mic ────────────────────────────────────────────────────

/// 处理 ForceTakeMic 信令，返回 JSON 字符串响应。
///
/// 仅 owner 或 admin 可调用；广播 `MicTaken { forced_by: operator_id }` 给房间所有成员。
///
/// # 参数
/// - `operator_room_id`：来自 WS session context（`registry.get_room_id(connection_id)`），
///   schema 规定 ForceTakeMic payload 不含 room_id（`additionalProperties: false`）。
///
// PROTO-BINDING: doc/protocol/schemas/ws/ForceTakeMic.schema.json (C→S)
// PROTO-BINDING: doc/protocol/schemas/ws/MicTaken.schema.json (S→Room broadcast)
// PROTO-BINDING: doc/protocol/schemas/ws/ForceTakeMicResult.schema.json (S→C result)
pub async fn handle_force_take_mic(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    operator_user_id: Uuid,
    operator_room_id: Option<Uuid>,
    deps: &ForceTakeMicDeps,
) -> String {
    let ForceTakeMicDeps {
        room_manager,
        room_service,
        mute_redis,
        registry,
    } = deps;

    // ── 1. room_id 来自 session context（不从 payload 读取，schema additionalProperties: false）
    let room_id = match operator_room_id {
        Some(id) => id,
        None => return force_take_error(msg_id, 40400, "operator not in any room"),
    };

    let target_user_id = match payload
        .as_ref()
        .and_then(|p| p.get("target_user_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return force_take_error(msg_id, 40002, "missing target_user_id"),
    };

    // schema 要求字段名为 mic_index（非 slot_index）
    let mic_index = match payload
        .as_ref()
        .and_then(|p| p.get("mic_index"))
        .and_then(|v| v.as_u64())
    {
        Some(i) if (i as usize) < 9 => i as usize,
        _ => return force_take_error(msg_id, 40002, "invalid or missing mic_index (0-8)"),
    };

    // ── 2. 加载房间 Model → 权限校验 ──────────────────────────────────────────
    let room = match room_service.get_active_room_model(room_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return force_take_error(msg_id, 40400, "room not found or closed"),
        Err(e) => {
            tracing::error!("get_active_room_model error: {e}");
            return force_take_error(msg_id, 50000, "internal error");
        }
    };

    let is_owner = room.owner_id == operator_user_id;
    let is_admin = room.admin_user_id == Some(operator_user_id);
    if !is_owner && !is_admin {
        return force_take_error(msg_id, 40301, "permission denied: owner or admin required");
    }

    // ── 3. 检查 target 是否被禁麦 ─────────────────────────────────────────────
    match mute_redis
        .get_mute_ttl("mic", room_id, target_user_id)
        .await
    {
        Ok(Some(_)) => return force_take_error(msg_id, 40306, "target is mic-muted"),
        Ok(None) => {} // 未被禁麦，继续
        Err(e) => {
            tracing::warn!("mic mute check failed: {e}");
            // 非阻断性，继续
        }
    }

    // ── 4. 获取房间运行时状态 ──────────────────────────────────────────────────
    let room_state = match room_manager.get_room(room_id) {
        Some(s) => s,
        None => return force_take_error(msg_id, 40400, "room not found in memory"),
    };

    // ── 5. 原子占用麦位 ───────────────────────────────────────────────────────
    match room_state.take_mic_slot(mic_index, target_user_id) {
        Ok(()) => {}
        Err(TakeMicError::SlotOccupied) => {
            return force_take_error(msg_id, 40907, "slot already occupied");
        }
        Err(TakeMicError::AlreadyOnMic) => {
            return force_take_error(msg_id, 40303, "target is already on mic");
        }
    }

    // ── 6. 广播 MicTaken { forced_by } ── 走统一出口 broadcast_to_room ────────
    // PROTO-BINDING: doc/protocol/schemas/ws/MicTaken.schema.json
    let mic_taken_envelope = serde_json::json!({
        "type": "MicTaken",
        "payload": {
            "mic_index": mic_index,
            "user_id": target_user_id.to_string(),
            "forced_by": operator_user_id.to_string(),
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    crate::ws::broadcaster::broadcast_to_room(registry, &room_state, mic_taken_envelope);

    force_take_success(msg_id, mic_index)
}

// ─── handle_force_leave_mic ───────────────────────────────────────────────────

/// 处理 ForceLeaveMic 信令，返回 JSON 字符串响应。
///
/// 仅 owner 或 admin 可调用；admin 不能抱下 owner（40302）。
/// 广播 `MicLeft { forced: true, forced_by: operator_id }` 给房间所有成员。
///
/// # 参数
/// - `operator_room_id`：来自 WS session context（`registry.get_room_id(connection_id)`），
///   schema 规定 ForceLeaveMic payload 不含 room_id（`additionalProperties: false`）。
///
// PROTO-BINDING: doc/protocol/schemas/ws/ForceLeaveMic.schema.json (C→S)
// PROTO-BINDING: doc/protocol/schemas/ws/MicLeft.schema.json (S→Room broadcast)
// PROTO-BINDING: doc/protocol/schemas/ws/ForceLeaveMicResult.schema.json (S→C result)
pub async fn handle_force_leave_mic(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    operator_user_id: Uuid,
    operator_room_id: Option<Uuid>,
    deps: &ForceLeaveMicDeps,
) -> String {
    let ForceLeaveMicDeps {
        room_manager,
        room_service,
        registry,
    } = deps;

    // ── 1. room_id 来自 session context（不从 payload 读取，schema additionalProperties: false）
    let room_id = match operator_room_id {
        Some(id) => id,
        None => return force_leave_error(msg_id, 40400, "operator not in any room"),
    };

    let target_user_id = match payload
        .as_ref()
        .and_then(|p| p.get("target_user_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return force_leave_error(msg_id, 40002, "missing target_user_id"),
    };

    // ── 2. 加载房间 Model → 权限校验 ──────────────────────────────────────────
    let room = match room_service.get_active_room_model(room_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return force_leave_error(msg_id, 40400, "room not found or closed"),
        Err(e) => {
            tracing::error!("get_active_room_model error: {e}");
            return force_leave_error(msg_id, 50000, "internal error");
        }
    };

    let is_owner = room.owner_id == operator_user_id;
    let is_admin = room.admin_user_id == Some(operator_user_id);
    if !is_owner && !is_admin {
        return force_leave_error(msg_id, 40301, "permission denied: owner or admin required");
    }

    // ── 3. 管理员不能抱下房主 ──────────────────────────────────────────────────
    // 注意：is_owner 优先（房主可以抱下任何人，包括自己）
    if is_admin && !is_owner && target_user_id == room.owner_id {
        return force_leave_error(msg_id, 40302, "admin cannot force owner off mic");
    }

    // ── 4. 获取房间运行时状态 ──────────────────────────────────────────────────
    let room_state = match room_manager.get_room(room_id) {
        Some(s) => s,
        None => return force_leave_error(msg_id, 40400, "room not found in memory"),
    };

    // ── 5. 原子查找并清除麦位 ─────────────────────────────────────────────────
    let mic_index = match room_state.leave_mic_slot(target_user_id) {
        Some(idx) => idx,
        None => return force_leave_error(msg_id, 40404, "target not on mic"),
    };

    // ── 6. 广播 MicLeft { forced: true, forced_by } — 走统一出口 broadcast_to_room
    // PROTO-BINDING: doc/protocol/schemas/ws/MicLeft.schema.json
    let mic_left_envelope = serde_json::json!({
        "type": "MicLeft",
        "payload": {
            "mic_index": mic_index,
            "user_id": target_user_id.to_string(),
            "forced": true,
            "forced_by": operator_user_id.to_string(),
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    crate::ws::broadcaster::broadcast_to_room(registry, &room_state, mic_left_envelope);

    force_leave_success(msg_id, mic_index)
}
