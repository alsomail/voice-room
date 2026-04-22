//! handle_join_room — JoinRoom 信令处理
//!
//! 流程：
//! 1. 解析 payload → room_id（失败 → code:40002）
//! 2. room_service.get_active_room_detail(room_id) → None → code:40400
//! 3. 密码房校验 access_token（T-00026）
//!    - 无 token → 40104 PASSWORD_REQUIRED
//!    - token 过期 → 40105 TOKEN_EXPIRED
//!    - room_id 不匹配 → 40106 INVALID_TOKEN
//! 4. auth_service.get_user_by_id(user_id) → 获取 nickname/avatar
//! 5. room_manager.get_or_create_room(room_id)
//! 6. room_state.members.insert(user_id, MemberInfo)
//! 7. registry.set_room_id(connection_id, room_id)
//! 8. stats.user_join_room(room_id).await.ok()
//! 9. 广播 UserJoined 给 registry.get_connections_in_room(room_id)
//! 10. 返回 JoinRoomResult { code:0, payload: room_snapshot }

use std::sync::Arc;

use jsonwebtoken::errors::ErrorKind;
use uuid::Uuid;
use voice_room_shared::auth::room_access::decode_room_access_token;

use crate::modules::auth::service::AuthService;
use crate::modules::governance::kick::KickRedis;
use crate::modules::governance::mute::MuteRedis;
use crate::modules::room::service::RoomService;
use crate::stats::StatsPort;
use crate::ws::registry::ConnectionRegistry;

use super::manager::RoomManager;
use super::state::MemberInfo;

// ─── 依赖容器 ─────────────────────────────────────────────────────────────────

/// `handle_join_room` 所需的 5 个服务依赖，聚合为一个 struct，
/// 消除 9 参数签名并避免 `clippy::too_many_arguments`。
pub struct JoinRoomDeps {
    pub room_manager: Arc<RoomManager>,
    pub room_service: Arc<RoomService>,
    pub auth_service: Arc<AuthService>,
    pub registry: Arc<ConnectionRegistry>,
    pub stats: Arc<dyn StatsPort>,
    /// JWT 密钥（用于 room access token 验证，T-00026）
    pub jwt_secret: String,
    /// 踢人冷却 Redis（T-00028 JoinRoom 前置检查）；None = 跳过检查
    pub kick_redis: Option<Arc<dyn KickRedis>>,
}

// ─── handle_join_room ────────────────────────────────────────────────────────

/// 处理 JoinRoom 信令，返回 JSON 字符串响应。
///
/// 所有重依赖通过 `deps: &JoinRoomDeps` 传入，调用方保持所有权。
pub async fn handle_join_room(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    connection_id: Uuid,
    user_id: Uuid,
    deps: &JoinRoomDeps,
) -> String {
    let JoinRoomDeps {
        room_manager,
        room_service,
        auth_service,
        registry,
        stats,
        jwt_secret,
        kick_redis,
    } = deps;

    // ── 1. 解析 room_id ───────────────────────────────────────────────────────
    let room_id = match payload
        .as_ref()
        .and_then(|p| p.get("room_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => {
            return error_response(msg_id, 40002, "invalid or missing room_id");
        }
    };

    // ── 2. 验证房间存在且 active ────────────────────────────────────────────────
    let room_detail = match room_service.get_active_room_detail(room_id).await {
        Ok(Some(detail)) => detail,
        Ok(None) => {
            return error_response(msg_id, 40400, "room not found or closed");
        }
        Err(e) => {
            tracing::error!("get_active_room_detail error: {e}");
            return error_response(msg_id, 50000, "internal error");
        }
    };

    // ── 2.5 踢出冷却检查（T-00028）────────────────────────────────────────────
    // K28-07: 被踢 10min 内重进 → 42911 + remaining_sec
    if let Some(ref kr) = kick_redis {
        match kr.get_kick_remaining_sec(room_id, user_id).await {
            Ok(Some(remaining)) => {
                return join_room_kick_cooldown_response(msg_id, remaining);
            }
            Ok(None) => {} // 未被踢或已过冷却，继续
            Err(e) => {
                tracing::warn!("kick cooldown check failed: {e}");
                // 非阻断性，继续加入
            }
        }
    }

    // ── 3. 密码房：校验 access_token（T-00026）──────────────────────────────
    if room_detail.room_type == "password" {
        let token = match payload
            .as_ref()
            .and_then(|p| p.get("access_token"))
            .and_then(|v| v.as_str())
        {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => {
                return error_response(msg_id, 40104, "password required: missing access_token");
            }
        };

        match decode_room_access_token(&token, jwt_secret.as_bytes()) {
            Err(e) if e.kind() == &ErrorKind::ExpiredSignature => {
                return error_response(msg_id, 40105, "access_token expired");
            }
            Err(_) => {
                return error_response(msg_id, 40106, "invalid access_token");
            }
            Ok(claims) => {
                // 校验 sub（user_id）
                if claims.sub != user_id.to_string() {
                    return error_response(msg_id, 40106, "invalid access_token: user mismatch");
                }
                // 校验 room_id
                if claims.room_id != room_id.to_string() {
                    return error_response(msg_id, 40106, "invalid access_token: room_id mismatch");
                }
            }
        }
    }

    // ── 4. 获取用户信息 ────────────────────────────────────────────────────────
    // Ok(None)：用户在 DB 中不存在，使用兜底昵称继续加入（非阻断性错误）
    // Err(e)：DB 故障，记录警告日志并返回 50000 内部错误
    let (nickname, avatar) = match auth_service.get_user_by_id(user_id).await {
        Ok(Some(user)) => (user.nickname, user.avatar),
        Ok(None) => ("Unknown".to_string(), None),
        Err(e) => {
            tracing::warn!("get_user_by_id failed for {user_id}: {e}");
            return error_response(msg_id, 50000, "internal error");
        }
    };

    // ── 5. 获取或创建内存房间状态 ───────────────────────────────────────────────
    let room_state = room_manager.get_or_create_room(room_id);

    // ── 6. 加入成员表 ──────────────────────────────────────────────────────────
    room_state.members.insert(
        user_id,
        MemberInfo::new(user_id, nickname.clone(), avatar.clone()),
    );

    // ── 7. 标记连接所属房间 ─────────────────────────────────────────────────────
    registry.set_room_id(connection_id, room_id);

    // ── 7. 统计：活跃房间 ──────────────────────────────────────────────────────
    stats.user_join_room(room_id).await.ok();

    // ── 8. 广播 UserJoined 给房间内所有连接（含自己）────────────────────────────
    let joined_msg = serde_json::json!({
        "type": "UserJoined",
        "payload": {
            "user_id": user_id.to_string(),
            "nickname": nickname,
            "avatar": avatar,
        },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    let joined_str = serde_json::to_string(&joined_msg).unwrap_or_default();

    for (_, sender) in registry.get_connections_in_room(room_id) {
        let _ = sender.send(joined_str.clone());
    }

    // ── 9. 返回 JoinRoomResult ──────────────────────────────────────────────────
    let mic_slots_json: Vec<serde_json::Value> = room_state
        .mic_slots_snapshot()
        .into_iter()
        .map(|slot| match slot {
            Some(uid) => serde_json::Value::String(uid.to_string()),
            None => serde_json::Value::Null,
        })
        .collect();

    let resp = serde_json::json!({
        "type": "JoinRoomResult",
        "msg_id": msg_id,
        "code": 0,
        "payload": {
            "room": {
                "room_id": room_state.room_id.to_string(),
                "title": room_detail.title,
                "owner_id": room_detail.owner_user_id.to_string(),
                "member_count": room_state.member_count(),
                "mic_slots": mic_slots_json,
            }
        },
        "timestamp": chrono::Utc::now().timestamp(),
    });

    serde_json::to_string(&resp).unwrap_or_default()
}

// ─── LeaveRoomDeps ───────────────────────────────────────────────────────────

/// `handle_leave_room` / `do_leave_room` 所需的 3 个服务依赖。
///
/// 比 `JoinRoomDeps` 更轻量：离开不需要验证房间是否 active，也不需要用户信息。
pub struct LeaveRoomDeps {
    pub room_manager: Arc<RoomManager>,
    pub registry: Arc<ConnectionRegistry>,
    pub stats: Arc<dyn StatsPort>,
}

// ─── do_leave_room ───────────────────────────────────────────────────────────

/// 核心离开逻辑（9 步），主动离开与断线共享此路径。
///
/// 任何静默返回点均视为非错误（用户不在房间是正常状态）。
pub async fn do_leave_room(
    connection_id: Uuid,
    user_id: Uuid,
    deps: &LeaveRoomDeps,
) {
    // 1. 获取 room_id，None 则用户不在任何房间，静默返回
    let room_id = match deps.registry.get_room_id(connection_id) {
        Some(id) => id,
        None => return,
    };

    // 2. 获取 room state，None 则房间已不存在，清除 room_id 后静默返回
    let room_state = match deps.room_manager.get_room(room_id) {
        Some(s) => s,
        None => {
            deps.registry.clear_room_id(connection_id);
            return;
        }
    };

    // 3. 移除成员
    room_state.members.remove(&user_id);

    // 4. 自动下麦（若在麦上），暂存麦位索引，广播延迟到 clear_room_id 之后
    let left_mic_index = room_state.leave_mic_slot(user_id);

    // 5. 先清除 room_id，使后续广播时自然排除离开者
    deps.registry.clear_room_id(connection_id);

    // 6. 统计：活跃房间（失败不阻断主流程）
    deps.stats.user_leave_room(room_id).await.ok();

    // 7. 广播 UserLeft 给房间剩余成员（离开者已被步骤 5 排除）
    let user_left = serde_json::json!({
        "type": "UserLeft",
        "payload": { "user_id": user_id.to_string() },
        "timestamp": chrono::Utc::now().timestamp(),
    })
    .to_string();

    let receivers = deps.registry.get_connections_in_room(room_id);
    for (_, sender) in receivers {
        let _ = sender.send(user_left.clone());
    }

    // 4'（被动下麦广播）：在 clear_room_id 之后广播 MicLeft，离开者已被排除
    if let Some(mic_index) = left_mic_index {
        broadcast_mic_left(&deps.registry, room_id, mic_index, user_id, false);
    }

    // 8. 空房间即时清理
    if room_state.member_count() == 0 {
        deps.room_manager.remove_room(room_id);
    }
}

// ─── handle_leave_room ───────────────────────────────────────────────────────

/// 处理 LeaveRoom 信令，调用 `do_leave_room` 后返回 JSON 响应字符串。
///
/// 仅主动离开时调用（断线路径直接调用 `do_leave_room`）。
pub async fn handle_leave_room(
    msg_id: Option<String>,
    connection_id: Uuid,
    user_id: Uuid,
    deps: &LeaveRoomDeps,
) -> String {
    do_leave_room(connection_id, user_id, deps).await;

    let resp = serde_json::json!({
        "type": "LeaveRoomResult",
        "msg_id": msg_id,
        "code": 0,
        "timestamp": chrono::Utc::now().timestamp(),
    });

    serde_json::to_string(&resp).unwrap_or_default()
}

// ─── 内部辅助 ─────────────────────────────────────────────────────────────────

fn error_response(msg_id: Option<String>, code: i64, message: &str) -> String {
    let resp = serde_json::json!({
        "type": "JoinRoomResult",
        "msg_id": msg_id,
        "code": code,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

/// 踢出冷却响应（42911）：包含 remaining_sec（T-00028 K28-07）
fn join_room_kick_cooldown_response(msg_id: Option<String>, remaining_sec: i64) -> String {
    let resp = serde_json::json!({
        "type": "JoinRoomResult",
        "msg_id": msg_id,
        "code": 42911,
        "message": "kicked cooldown",
        "payload": { "remaining_sec": remaining_sec },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

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
    use super::state::TakeMicError;

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

    // ── 6. 广播 MicTaken 给房间内所有连接（含请求方）────────────────────────
    let mic_taken = serde_json::json!({
        "type": "MicTaken",
        "payload": {
            "mic_index": mic_index,
            "user_id": user_id.to_string(),
        },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    let mic_taken_str = serde_json::to_string(&mic_taken).unwrap_or_default();
    for (_, sender) in deps.registry.get_connections_in_room(room_id) {
        let _ = sender.send(mic_taken_str.clone());
    }

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

    // ── 4. 广播 MicLeft 给房间内所有连接（含请求方）──────────────────────────
    broadcast_mic_left(&deps.registry, room_id, mic_index, user_id, false);

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

/// 广播 MicLeft 给房间内所有连接（T-00028 新增 forced 字段）
///
/// - `forced = false`：主动下麦（LeaveRoom / LeaveMic）
/// - `forced = true`：被踢下麦（KickUser）
pub(crate) fn broadcast_mic_left(
    registry: &ConnectionRegistry,
    room_id: Uuid,
    mic_index: usize,
    user_id: Uuid,
    forced: bool,
) {
    let mic_left = serde_json::json!({
        "type": "MicLeft",
        "payload": {
            "mic_index": mic_index,
            "user_id": user_id.to_string(),
            "forced": forced,
        },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    let mic_left_str = serde_json::to_string(&mic_left).unwrap_or_default();
    for (_, sender) in registry.get_connections_in_room(room_id) {
        let _ = sender.send(mic_left_str.clone());
    }
}

// ─── SendMessageDeps ──────────────────────────────────────────────────────────

/// `handle_send_message` 所需的服务依赖。
///
/// 发送消息只需要 room_manager 和 registry，与 TakeMicDeps / LeaveMicDeps 同构。
pub struct SendMessageDeps {
    pub room_manager: Arc<RoomManager>,
    pub registry: Arc<ConnectionRegistry>,
    /// 禁言 Redis（T-00029 前置拦截）；None = 跳过拦截
    pub mute_redis: Option<Arc<dyn MuteRedis>>,
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
    use super::filter::filter_content;

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
            return send_message_error_response(msg_id, 40002, "content is required and must not be empty");
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

    // ── 8. 广播 RoomMessage 给房间内所有连接（含请求方）──────────────────────
    let room_msg = serde_json::json!({
        "type": "RoomMessage",
        "payload": {
            "msg_id": msg_id_str,
            "user_id": user_id.to_string(),
            "content": filtered_content,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    let room_msg_str = serde_json::to_string(&room_msg).unwrap_or_default();
    for (_, sender) in deps.registry.get_connections_in_room(room_id) {
        let _ = sender.send(room_msg_str.clone());
    }

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

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;

    use chrono::Utc;
    use tokio::sync::mpsc;
    use uuid::Uuid;
    use voice_room_shared::models::room::RoomModel;
    use voice_room_shared::models::user::UserModel;

    use crate::infrastructure::redis_store::FakeCodeStore;
    use crate::infrastructure::third_party::sms::MockSmsProvider;
    use crate::modules::auth::repository::{FailingUserRepository, FakeUserRepository};
    use crate::modules::room::repository::FakeRoomRepository;
    use crate::stats::FakeStatsService;
    use crate::ws::registry::{ConnectionHandle, ConnectionRegistry};

    // ── 测试辅助 ──────────────────────────────────────────────────────────────

    /// 创建空 RoomService（无任何房间）— 模拟"房间不存在"场景
    fn empty_room_service() -> Arc<RoomService> {
        Arc::new(RoomService::new(Arc::new(FakeRoomRepository::default())))
    }

    /// 创建含指定房间的 RoomService
    fn room_service_with(room_id: Uuid) -> Arc<RoomService> {
        let repo = Arc::new(FakeRoomRepository::default());
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id: Uuid::new_v4(),
            title: "Test Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
        Arc::new(RoomService::new(repo))
    }

    /// 创建含指定用户的 AuthService
    fn auth_service_with(user_id: Uuid, nickname: &str) -> Arc<AuthService> {
        let user_repo = Arc::new(FakeUserRepository::default());
        let now = Utc::now();
        user_repo.seed(UserModel {
            id: user_id,
            phone: "+8613800000000".to_string(),
            nickname: nickname.to_string(),
            avatar: Some("https://example.com/avatar.jpg".to_string()),
            coin_balance: 0,
            diamond_balance: 0,
            charm_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        });
        Arc::new(AuthService::new(
            user_repo,
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider::default()),
            "test-secret".to_string(),
        ))
    }

    /// 创建空 AuthService（无任何用户）
    fn empty_auth_service() -> Arc<AuthService> {
        Arc::new(AuthService::new(
            Arc::new(FakeUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider::default()),
            "test-secret".to_string(),
        ))
    }

    /// 创建总是返回 DB 错误的 AuthService（用于 I-02 测试）
    fn failing_auth_service() -> Arc<AuthService> {
        Arc::new(AuthService::new(
            Arc::new(FailingUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider::default()),
            "test-secret".to_string(),
        ))
    }

    /// 向 registry 注册一个连接，返回 (connection_id, rx)
    fn register_connection(
        registry: &Arc<ConnectionRegistry>,
        user_id: Uuid,
        room_id: Option<Uuid>,
    ) -> (Uuid, mpsc::UnboundedReceiver<String>) {
        let conn_id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel();
        registry.register(ConnectionHandle {
            connection_id: conn_id,
            user_id,
            room_id,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });
        (conn_id, rx)
    }

    /// 构建 JoinRoom payload
    fn join_payload(room_id: Uuid) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "room_id": room_id.to_string() }))
    }

    /// 构建 JoinRoomDeps（测试辅助，减少重复代码）
    fn build_deps(
        room_manager: &Arc<RoomManager>,
        room_service: &Arc<RoomService>,
        auth_service: &Arc<AuthService>,
        registry: &Arc<ConnectionRegistry>,
        stats: &Arc<dyn StatsPort>,
    ) -> JoinRoomDeps {
        JoinRoomDeps {
            room_manager: room_manager.clone(),
            room_service: room_service.clone(),
            auth_service: auth_service.clone(),
            registry: registry.clone(),
            stats: stats.clone(),
            jwt_secret: "test-secret".to_string(),
            kick_redis: None,
        }
    }

    // J03: FakeRoomService 返回 None → 响应 code=40400
    #[tokio::test]
    async fn j03_room_not_found_returns_40400() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = empty_room_service(); // 无房间，get_active_room_detail 返回 None
        let auth_service = empty_auth_service();
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        let response = handle_join_room(
            join_payload(room_id),
            Some("msg-j03".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40400, "should return code 40400 when room not found");
        assert_eq!(json["type"], "JoinRoomResult");
        assert_eq!(json["msg_id"], "msg-j03");
    }

    // J04: 成功加入 → members 包含 user_id
    #[tokio::test]
    async fn j04_success_members_contains_user_id() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "Alice");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        handle_join_room(join_payload(room_id), None, conn_id, user_id, &deps).await;

        let room_state = room_manager.get_room(room_id).expect("room should exist in manager");
        assert!(
            room_state.members.contains_key(&user_id),
            "members should contain user_id after successful join"
        );
    }

    // J05: 成功加入 → registry.get_connections_in_room 包含该连接
    #[tokio::test]
    async fn j05_success_registry_contains_connection() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "Bob");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        handle_join_room(join_payload(room_id), None, conn_id, user_id, &deps).await;

        let conns = registry.get_connections_in_room(room_id);
        assert!(
            conns.iter().any(|(cid, _)| *cid == conn_id),
            "get_connections_in_room should include the newly joined connection"
        );
    }

    // J06: 成功加入 → FakeStatsService.active_rooms 包含 room_id
    #[tokio::test]
    async fn j06_success_stats_active_rooms_contains_room_id() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "Carol");
        let registry = Arc::new(ConnectionRegistry::new());
        let fake_stats = Arc::new(FakeStatsService::default());
        let stats: Arc<dyn StatsPort> = fake_stats.clone();

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        handle_join_room(join_payload(room_id), None, conn_id, user_id, &deps).await;

        assert!(
            fake_stats.active_rooms.lock().unwrap().contains(&room_id),
            "FakeStatsService.active_rooms should contain room_id after join"
        );
    }

    // J07: 成功加入 → 已有连接的 rx 收到 UserJoined
    #[tokio::test]
    async fn j07_success_existing_connection_receives_user_joined() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_a_id = Uuid::new_v4();
        let user_b_id = Uuid::new_v4();

        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_b_id, "UserB");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 用户 A 已在房间中（直接设置 room_id）
        let (conn_a_id, mut rx_a) = register_connection(&registry, user_a_id, Some(room_id));
        let _ = conn_a_id;

        // 用户 B 加入房间
        let (conn_b_id, _rx_b) = register_connection(&registry, user_b_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        handle_join_room(join_payload(room_id), None, conn_b_id, user_b_id, &deps).await;

        // 用户 A 的 rx 应该收到 UserJoined 消息
        let msg = tokio::time::timeout(Duration::from_millis(200), rx_a.recv())
            .await
            .expect("rx_a should not timeout")
            .expect("channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "UserJoined", "existing connection should receive UserJoined");
    }

    // J08: 成功加入 → 响应 code=0，payload.room.member_count >= 1
    #[tokio::test]
    async fn j08_success_response_code_0_and_member_count() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "Dave");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        let response = handle_join_room(
            join_payload(room_id),
            Some("msg-j08".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "success response should have code=0");
        assert_eq!(json["type"], "JoinRoomResult");

        let member_count = json["payload"]["room"]["member_count"]
            .as_i64()
            .expect("member_count should be i64");
        assert!(
            member_count >= 1,
            "member_count should be >= 1 after joining; got {member_count}"
        );

        // mic_slots 应为 9 个元素
        let mic_slots = json["payload"]["room"]["mic_slots"]
            .as_array()
            .expect("mic_slots should be array");
        assert_eq!(mic_slots.len(), 9, "mic_slots should have 9 elements");
    }

    // J09: 用户 B 加入 → 用户 A 收到 UserJoined(user_id=B)
    #[tokio::test]
    async fn j09_user_b_joins_user_a_receives_user_joined_with_b_user_id() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_a_id = Uuid::new_v4();
        let user_b_id = Uuid::new_v4();

        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id);
        let auth_service = auth_service_with(user_b_id, "UserB");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 用户 A 已在房间
        let (_conn_a_id, mut rx_a) = register_connection(&registry, user_a_id, Some(room_id));

        // 用户 B 加入
        let (conn_b_id, _rx_b) = register_connection(&registry, user_b_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        handle_join_room(join_payload(room_id), None, conn_b_id, user_b_id, &deps).await;

        let msg = tokio::time::timeout(Duration::from_millis(200), rx_a.recv())
            .await
            .expect("rx_a should not timeout")
            .expect("channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "UserJoined");
        assert_eq!(
            json["payload"]["user_id"],
            user_b_id.to_string(),
            "UserJoined payload.user_id must be user B's ID"
        );
        assert_eq!(
            json["payload"]["nickname"], "UserB",
            "UserJoined payload.nickname must match user B's nickname"
        );
    }

    // ── LeaveRoom 测试辅助 ────────────────────────────────────────────────────

    /// 构建 LeaveRoomDeps（测试辅助）
    fn build_leave_deps(
        room_manager: &Arc<RoomManager>,
        registry: &Arc<ConnectionRegistry>,
        stats: &Arc<dyn StatsPort>,
    ) -> LeaveRoomDeps {
        LeaveRoomDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
            stats: stats.clone(),
        }
    }

    // L01: do_leave_room 成员被移除（room_state.members.get(&user_id) 为 None）
    #[tokio::test]
    async fn l01_do_leave_room_removes_member() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 先加入：手动建立 room_state 并插入成员
        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_id,
            MemberInfo::new(user_id, "Alice".to_string(), None),
        );

        // 注册连接并设置 room_id
        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        assert!(
            room_state.members.get(&user_id).is_none(),
            "L01: member should be removed from room_state.members after do_leave_room"
        );
    }

    // L02: 用户未加入房间（registry.get_room_id = None），do_leave_room 静默返回无 panic
    #[tokio::test]
    async fn l02_do_leave_room_no_room_id_silent_return() {
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let user_id = Uuid::new_v4();
        // 注册连接但不设置 room_id（room_id = None）
        let (conn_id, _rx) = register_connection(&registry, user_id, None);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        // 不应 panic
        do_leave_room(conn_id, user_id, &deps).await;

        assert_eq!(room_manager.room_count(), 0, "L02: no rooms should be created");
    }

    // L03: 在麦用户离开后 mic_slots_snapshot()[slot_idx] 为 None
    #[tokio::test]
    async fn l03_do_leave_room_removes_user_from_mic_slot() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_id,
            MemberInfo::new(user_id, "Bob".to_string(), None),
        );
        // 将用户放到麦位 2
        {
            let mut slots = room_state.mic_slots.write().unwrap();
            slots[2] = Some(user_id);
        }

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot[2].is_none(),
            "L03: mic slot 2 should be None after on-mic user leaves"
        );
    }

    // L04: 广播 UserLeft，已有成员的 rx 收到含 user_id 的消息
    #[tokio::test]
    async fn l04_do_leave_room_broadcasts_user_left_to_remaining_members() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let leaver_id = Uuid::new_v4();
        let stayer_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            leaver_id,
            MemberInfo::new(leaver_id, "Leaver".to_string(), None),
        );
        room_state.members.insert(
            stayer_id,
            MemberInfo::new(stayer_id, "Stayer".to_string(), None),
        );

        // leaver 的连接
        let (leaver_conn, _rx_leaver) = register_connection(&registry, leaver_id, None);
        registry.set_room_id(leaver_conn, room_id);

        // stayer 的连接（已在房间）
        let (_stayer_conn, mut rx_stayer) = register_connection(&registry, stayer_id, Some(room_id));

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(leaver_conn, leaver_id, &deps).await;

        let msg = tokio::time::timeout(Duration::from_millis(200), rx_stayer.recv())
            .await
            .expect("L04: rx_stayer should not timeout")
            .expect("L04: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "UserLeft", "L04: broadcast type should be UserLeft");
        assert_eq!(
            json["payload"]["user_id"],
            leaver_id.to_string(),
            "L04: UserLeft payload.user_id should match leaver"
        );
    }

    // L05: 广播不含离开者本身（离开者的 rx 不收到 UserLeft）
    #[tokio::test]
    async fn l05_do_leave_room_does_not_broadcast_to_leaver() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let leaver_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            leaver_id,
            MemberInfo::new(leaver_id, "Solo".to_string(), None),
        );

        let (leaver_conn, mut rx_leaver) = register_connection(&registry, leaver_id, None);
        registry.set_room_id(leaver_conn, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(leaver_conn, leaver_id, &deps).await;

        // leaver 自己的 rx 不应收到 UserLeft（100ms 超时）
        let result = tokio::time::timeout(Duration::from_millis(100), rx_leaver.recv()).await;
        assert!(
            result.is_err(),
            "L05: leaver should NOT receive UserLeft broadcast (channel should timeout)"
        );
    }

    // L06: 最后一个成员离开 room_manager.room_count() == 0
    #[tokio::test]
    async fn l06_do_leave_room_last_member_removes_room() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_id,
            MemberInfo::new(user_id, "Last".to_string(), None),
        );

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        assert_eq!(room_manager.room_count(), 1);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        assert_eq!(
            room_manager.room_count(),
            0,
            "L06: room should be removed when last member leaves"
        );
    }

    // L07: FakeStatsService.active_rooms 不含 room_id（通过 user_leave_room 触发）
    #[tokio::test]
    async fn l07_do_leave_room_stats_active_rooms_does_not_contain_room_id() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let fake_stats = Arc::new(FakeStatsService::default());
        let stats: Arc<dyn StatsPort> = fake_stats.clone();

        // 预先将 room_id 加入 active_rooms（模拟 join 时的状态）
        fake_stats.active_rooms.lock().unwrap().insert(room_id);

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_id,
            MemberInfo::new(user_id, "Stat".to_string(), None),
        );

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        assert!(
            !fake_stats.active_rooms.lock().unwrap().contains(&room_id),
            "L07: active_rooms should NOT contain room_id after user leaves"
        );
    }

    // L08: handle_leave_room 返回 JSON {"type":"LeaveRoomResult","code":0,...}
    #[tokio::test]
    async fn l08_handle_leave_room_returns_code_0_and_correct_type() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_id,
            MemberInfo::new(user_id, "H8User".to_string(), None),
        );

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        let response = handle_leave_room(
            Some("msg-l08".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["type"], "LeaveRoomResult", "L08: type should be LeaveRoomResult");
        assert_eq!(json["code"], 0, "L08: code should be 0 for successful leave");
        assert_eq!(json["msg_id"], "msg-l08", "L08: msg_id should be echoed back");
    }

    // L10: 不在麦上的用户离开后 mic_slots_snapshot() 全部为 None
    #[tokio::test]
    async fn l10_do_leave_room_non_mic_user_slots_unchanged() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_id,
            MemberInfo::new(user_id, "NoMic".to_string(), None),
        );
        // 不向任何麦位插入用户（初始全为 None）

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        registry.set_room_id(conn_id, room_id);

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_id, user_id, &deps).await;

        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot.iter().all(|s| s.is_none()),
            "L10: all mic slots should remain None when a non-mic user leaves"
        );
    }

    // L11: 在麦上用户 do_leave_room 后，离开者 rx 在 100ms 内收不到 MicLeft；
    //      但旁听者能收到 MicLeft（包含正确的 mic_index 和 user_id）
    #[tokio::test]
    async fn l11_do_leave_room_mic_left_not_sent_to_leaver() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_a_id = Uuid::new_v4(); // 在麦位 0
        let user_b_id = Uuid::new_v4(); // 旁听

        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let room_state = room_manager.get_or_create_room(room_id);
        room_state.members.insert(
            user_a_id,
            MemberInfo::new(user_a_id, "OnMic".to_string(), None),
        );
        room_state.members.insert(
            user_b_id,
            MemberInfo::new(user_b_id, "Listener".to_string(), None),
        );
        // user_a 在麦位 0
        {
            let mut slots = room_state.mic_slots.write().unwrap();
            slots[0] = Some(user_a_id);
        }

        // user_a 的连接（即将离开）
        let (conn_a, mut rx_a) = register_connection(&registry, user_a_id, None);
        registry.set_room_id(conn_a, room_id);

        // user_b 的连接（旁听，留在房间）
        let (_conn_b, mut rx_b) = register_connection(&registry, user_b_id, Some(room_id));

        let deps = build_leave_deps(&room_manager, &registry, &stats);
        do_leave_room(conn_a, user_a_id, &deps).await;

        // 验证1：user_a（离开者）在 100ms 内收不到任何消息（含 MicLeft）
        let result_a = tokio::time::timeout(Duration::from_millis(100), rx_a.recv()).await;
        assert!(
            result_a.is_err(),
            "L11: leaver (user_a) should NOT receive MicLeft after do_leave_room"
        );

        // 验证2：user_b（旁听者）能收到 MicLeft，且内容正确
        let msg_b = tokio::time::timeout(Duration::from_millis(200), rx_b.recv())
            .await
            .expect("L11: rx_b should not timeout — listener must receive MicLeft")
            .expect("L11: rx_b channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg_b).unwrap();
        // 可能先收到 MicLeft 或 UserLeft，找到 MicLeft
        if json["type"] == "MicLeft" {
            assert_eq!(
                json["payload"]["mic_index"], 0,
                "L11: MicLeft payload.mic_index should be 0"
            );
            assert_eq!(
                json["payload"]["user_id"],
                user_a_id.to_string(),
                "L11: MicLeft payload.user_id should be user_a"
            );
        } else {
            // 第一条是 UserLeft，第二条应为 MicLeft
            let msg_b2 = tokio::time::timeout(Duration::from_millis(200), rx_b.recv())
                .await
                .expect("L11: rx_b second recv should not timeout")
                .expect("L11: rx_b second channel should not be closed");
            let json2: serde_json::Value = serde_json::from_str(&msg_b2).unwrap();
            assert_eq!(json2["type"], "MicLeft", "L11: second message to listener should be MicLeft");
            assert_eq!(
                json2["payload"]["mic_index"], 0,
                "L11: MicLeft payload.mic_index should be 0"
            );
            assert_eq!(
                json2["payload"]["user_id"],
                user_a_id.to_string(),
                "L11: MicLeft payload.user_id should be user_a"
            );
        }
    }

    // ── TakeMic 测试辅助 ──────────────────────────────────────────────────────

    /// 构建 TakeMicDeps（测试辅助）
    fn build_take_mic_deps(
        room_manager: &Arc<RoomManager>,
        registry: &Arc<ConnectionRegistry>,
    ) -> TakeMicDeps {
        TakeMicDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
            mute_redis: None,
        }
    }

    /// 构建 TakeMic payload
    fn take_mic_payload(mic_index: u64) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "mic_index": mic_index }))
    }

    // M01: 成功上麦，mic_slots_snapshot()[0] == Some(user_id)
    #[tokio::test]
    async fn m01_take_mic_success_slot_occupied() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        assert_eq!(
            room_state.mic_slots_snapshot()[0],
            Some(user_id),
            "M01: mic slot 0 should be occupied by user_id after successful take_mic"
        );
    }

    // M02: 麦位已被他人占用，code=40303
    #[tokio::test]
    async fn m02_take_mic_slot_occupied_returns_40303() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let other_user = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 预先占用 slot 0（另一个用户）
        room_state.take_mic_slot(0, other_user).expect("pre-fill should succeed");

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        let response = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40303, "M02: occupied slot should return code 40303");
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M03: 用户已在某麦位，再次上麦，code=40301
    #[tokio::test]
    async fn m03_take_mic_user_already_on_mic_returns_40301() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 用户已在 slot 1
        room_state.take_mic_slot(1, user_id).expect("pre-fill should succeed");

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        // 尝试占用 slot 0（用户已在 slot 1）
        let response = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40301, "M03: user already on mic should return code 40301");
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M04: 用户在 banned_mics 中，code=40302
    #[tokio::test]
    async fn m04_take_mic_user_banned_returns_40302() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 将用户加入禁麦列表
        room_state.banned_mics.insert(user_id);

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        let response = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40302, "M04: banned user should return code 40302");
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M05: 用户不在房间（get_room_id=None），code=40400
    #[tokio::test]
    async fn m05_take_mic_user_not_in_room_returns_40400() {
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        // 注册连接但 room_id=None（用户未进入任何房间）
        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_take_mic_deps(&room_manager, &registry);

        let response = handle_take_mic(take_mic_payload(0), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40400, "M05: user not in room should return code 40400");
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M06: mic_index=9（超出0-8），code=40002
    #[tokio::test]
    async fn m06_take_mic_index_out_of_range_returns_40002() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        // mic_index=9 超出有效范围 0-8
        let response = handle_take_mic(take_mic_payload(9), None, conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40002, "M06: mic_index=9 should return code 40002");
        assert_eq!(json["type"], "TakeMicResult");
    }

    // M07: 成功上麦后，其他连接的 rx 收到含正确 user_id 和 mic_index 的 MicTaken 广播
    #[tokio::test]
    async fn m07_take_mic_broadcasts_mic_taken_to_other_connections() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let observer_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);

        // 观察者已在房间
        let (_obs_conn, mut rx_observer) = register_connection(&registry, observer_id, Some(room_id));

        // 上麦者的连接
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        handle_take_mic(take_mic_payload(3), None, conn_id, user_id, &deps).await;

        // 观察者应收到 MicTaken 广播
        let msg = tokio::time::timeout(Duration::from_millis(200), rx_observer.recv())
            .await
            .expect("M07: rx_observer should not timeout")
            .expect("M07: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "MicTaken", "M07: broadcast type should be MicTaken");
        assert_eq!(
            json["payload"]["user_id"],
            user_id.to_string(),
            "M07: MicTaken payload.user_id should match the user who took the mic"
        );
        assert_eq!(
            json["payload"]["mic_index"], 3,
            "M07: MicTaken payload.mic_index should match the requested slot"
        );
    }

    // M08: 响应 code=0，type="TakeMicResult"，payload.mic_index 正确
    #[tokio::test]
    async fn m08_take_mic_success_response_code_0_and_correct_payload() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_take_mic_deps(&room_manager, &registry);

        let response = handle_take_mic(
            take_mic_payload(5),
            Some("msg-m08".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "M08: success response should have code=0");
        assert_eq!(json["type"], "TakeMicResult", "M08: type should be TakeMicResult");
        assert_eq!(json["msg_id"], "msg-m08", "M08: msg_id should be echoed back");
        assert_eq!(
            json["payload"]["mic_index"], 5,
            "M08: payload.mic_index should match the requested slot"
        );
    }

    // M09: 并发抢麦 — 两个 tokio::spawn 并发调用 take_mic_slot(0, ...)，只有一个 Ok
    #[tokio::test]
    async fn m09_concurrent_take_mic_slot_only_one_succeeds() {
        use crate::room::state::TakeMicError;

        let room_id = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let room_state = Arc::new(crate::room::state::RoomState::new(room_id));

        let state_a = room_state.clone();
        let state_b = room_state.clone();

        let task_a = tokio::spawn(async move { state_a.take_mic_slot(0, user_a) });
        let task_b = tokio::spawn(async move { state_b.take_mic_slot(0, user_b) });

        let result_a = task_a.await.expect("M09: task_a should not panic");
        let result_b = task_b.await.expect("M09: task_b should not panic");

        // 恰好一个成功，另一个返回 SlotOccupied
        let successes = [result_a.is_ok(), result_b.is_ok()]
            .iter()
            .filter(|&&x| x)
            .count();
        assert_eq!(successes, 1, "M09: exactly one concurrent take_mic_slot should succeed");

        // 失败者返回 SlotOccupied（而不是 AlreadyOnMic）
        let failure = if result_a.is_err() { result_a } else { result_b };
        assert_eq!(
            failure.unwrap_err(),
            TakeMicError::SlotOccupied,
            "M09: losing task should get SlotOccupied error"
        );

        // slot 0 恰好被一个用户占用
        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot[0].is_some(),
            "M09: slot 0 should be occupied by exactly one user after concurrent take"
        );
    }

    // ── LeaveMic 测试辅助 ─────────────────────────────────────────────────────

    /// 构建 LeaveMicDeps（测试辅助）
    fn build_leave_mic_deps(
        room_manager: &Arc<RoomManager>,
        registry: &Arc<ConnectionRegistry>,
    ) -> LeaveMicDeps {
        LeaveMicDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
        }
    }

    // N01: 成功下麦，mic_slots_snapshot()[idx] == None（麦位已被清空）
    #[tokio::test]
    async fn n01_leave_mic_success_slot_cleared() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 将用户预先放到麦位 2
        room_state.take_mic_slot(2, user_id).expect("pre-fill should succeed");

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);

        handle_leave_mic(None, conn_id, user_id, &deps).await;

        let snapshot = room_state.mic_slots_snapshot();
        assert!(
            snapshot[2].is_none(),
            "N01: mic slot 2 should be None after successful leave_mic"
        );
    }

    // N02: 用户不在麦上，返回 code=40304
    #[tokio::test]
    async fn n02_leave_mic_user_not_on_mic_returns_40304() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        // 用户在房间内，但未占用任何麦位
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);

        let response = handle_leave_mic(Some("msg-n02".to_string()), conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40304, "N02: user not on mic should return code 40304");
        assert_eq!(json["type"], "LeaveMicResult", "N02: type should be LeaveMicResult");
        assert_eq!(json["msg_id"], "msg-n02");
    }

    // N03: 用户不在房间（get_room_id=None），返回 code=40400
    #[tokio::test]
    async fn n03_leave_mic_user_not_in_room_returns_40400() {
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        // 注册连接但 room_id=None（用户未进入任何房间）
        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_leave_mic_deps(&room_manager, &registry);

        let response = handle_leave_mic(Some("msg-n03".to_string()), conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40400, "N03: user not in room should return code 40400");
        assert_eq!(json["type"], "LeaveMicResult");
        assert_eq!(json["msg_id"], "msg-n03");
    }

    // N04: 成功下麦，响应 code=0，payload.mic_index 与实际麦位一致
    #[tokio::test]
    async fn n04_leave_mic_success_response_code_0_and_correct_payload() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 用户在麦位 5
        room_state.take_mic_slot(5, user_id).expect("pre-fill should succeed");

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);

        let response = handle_leave_mic(Some("msg-n04".to_string()), conn_id, user_id, &deps).await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "N04: success response should have code=0");
        assert_eq!(json["type"], "LeaveMicResult", "N04: type should be LeaveMicResult");
        assert_eq!(json["msg_id"], "msg-n04");
        assert_eq!(
            json["payload"]["mic_index"], 5,
            "N04: payload.mic_index should match the slot the user was on"
        );
    }

    // N05: 成功下麦，房间内其他成员收到含正确 mic_index 和 user_id 的 MicLeft 广播
    #[tokio::test]
    async fn n05_leave_mic_broadcasts_mic_left_to_other_members() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let observer_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 用户在麦位 3
        room_state.take_mic_slot(3, user_id).expect("pre-fill should succeed");

        // 观察者已在房间
        let (_obs_conn, mut rx_observer) =
            register_connection(&registry, observer_id, Some(room_id));

        // 下麦者的连接
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);

        handle_leave_mic(None, conn_id, user_id, &deps).await;

        // 观察者应收到 MicLeft 广播
        let msg = tokio::time::timeout(Duration::from_millis(200), rx_observer.recv())
            .await
            .expect("N05: rx_observer should not timeout")
            .expect("N05: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "MicLeft", "N05: broadcast type should be MicLeft");
        assert_eq!(
            json["payload"]["mic_index"], 3,
            "N05: MicLeft payload.mic_index should match the slot vacated"
        );
        assert_eq!(
            json["payload"]["user_id"],
            user_id.to_string(),
            "N05: MicLeft payload.user_id should match the user who left the mic"
        );
    }

    // N06: 下麦不影响其他用户的麦位（其余槽位保持不变）
    #[tokio::test]
    async fn n06_leave_mic_does_not_affect_other_slots() {
        let room_id = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // user_a 在麦位 1，user_b 在麦位 4
        room_state.take_mic_slot(1, user_a).expect("pre-fill a should succeed");
        room_state.take_mic_slot(4, user_b).expect("pre-fill b should succeed");

        // user_a 下麦
        let (conn_a, _rx_a) = register_connection(&registry, user_a, Some(room_id));
        let deps = build_leave_mic_deps(&room_manager, &registry);
        handle_leave_mic(None, conn_a, user_a, &deps).await;

        // 验证：user_a 的麦位 1 已清空，user_b 的麦位 4 不受影响
        let snapshot = room_state.mic_slots_snapshot();
        assert!(snapshot[1].is_none(), "N06: slot 1 should be None after user_a leaves mic");
        assert_eq!(
            snapshot[4],
            Some(user_b),
            "N06: slot 4 should remain occupied by user_b"
        );
    }

    // N07: leave_mic_slot 用户在麦上时原子性地返回 Some(idx)
    #[tokio::test]
    async fn n07_leave_mic_slot_returns_some_idx_when_on_mic() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_state = crate::room::state::RoomState::new(room_id);
        // 将用户放到麦位 6
        room_state.take_mic_slot(6, user_id).expect("pre-fill should succeed");

        let result = room_state.leave_mic_slot(user_id);

        assert_eq!(
            result,
            Some(6),
            "N07: leave_mic_slot should return Some(6) when user is on slot 6"
        );
        // 且槽位已被置为 None
        assert!(
            room_state.mic_slots_snapshot()[6].is_none(),
            "N07: slot 6 should be None after leave_mic_slot"
        );
    }

    // N08: leave_mic_slot 对不在任何麦位的用户返回 None
    #[tokio::test]
    async fn n08_leave_mic_slot_returns_none_when_not_on_mic() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_state = crate::room::state::RoomState::new(room_id);
        // 用户未在任何麦位

        let result = room_state.leave_mic_slot(user_id);

        assert!(
            result.is_none(),
            "N08: leave_mic_slot should return None when user is not on any mic slot"
        );
    }

    // ── SendMessage 测试辅助 ──────────────────────────────────────────────────

    /// 构建 SendMessageDeps（测试辅助）
    fn build_send_message_deps(
        room_manager: &Arc<RoomManager>,
        registry: &Arc<ConnectionRegistry>,
    ) -> SendMessageDeps {
        SendMessageDeps {
            room_manager: room_manager.clone(),
            registry: registry.clone(),
            mute_redis: None,
        }
    }

    /// 构建 SendMessage payload
    fn send_message_payload(content: &str) -> Option<serde_json::Value> {
        Some(serde_json::json!({ "content": content }))
    }

    // S01: 成功发送，其他成员 rx 收到 RoomMessage，payload.content 正确
    #[tokio::test]
    async fn s01_send_message_success_other_member_receives_room_message() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);

        // receiver 已在房间
        let (_recv_conn, mut rx_receiver) =
            register_connection(&registry, receiver_id, Some(room_id));

        // sender 的连接，已在房间
        let (sender_conn, _rx_sender) = register_connection(&registry, sender_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        handle_send_message(
            send_message_payload("Hello everyone!"),
            Some("msg-s01".to_string()),
            sender_conn,
            sender_id,
            &deps,
        )
        .await;

        let msg = tokio::time::timeout(Duration::from_millis(200), rx_receiver.recv())
            .await
            .expect("S01: rx_receiver should not timeout")
            .expect("S01: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "RoomMessage", "S01: broadcast type should be RoomMessage");
        assert_eq!(
            json["payload"]["content"],
            "Hello everyone!",
            "S01: broadcast content should match sent content"
        );
    }

    // S02: 超过 500 字符，code=40001
    #[tokio::test]
    async fn s02_send_message_too_long_returns_40001() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        // 501 字符
        let long_content = "a".repeat(501);
        let response = handle_send_message(
            send_message_payload(&long_content),
            Some("msg-s02".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40001, "S02: content > 500 chars should return code 40001");
        assert_eq!(json["type"], "SendMessageResult");
    }

    // S03: 用户在 muted_users，code=40303
    #[tokio::test]
    async fn s03_send_message_muted_user_returns_40303() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        let room_state = room_manager.get_or_create_room(room_id);
        // 将用户加入禁言列表
        room_state.muted_users.insert(user_id);

        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        let response = handle_send_message(
            send_message_payload("Hello"),
            Some("msg-s03".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40303, "S03: muted user should return code 40303");
        assert_eq!(json["type"], "SendMessageResult");
    }

    // S04: 用户不在房间（get_room_id=None），code=40400
    #[tokio::test]
    async fn s04_send_message_user_not_in_room_returns_40400() {
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        // 注册连接但 room_id=None（用户未进入任何房间）
        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_send_message_deps(&room_manager, &registry);

        let response = handle_send_message(
            send_message_payload("Hello"),
            Some("msg-s04".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40400, "S04: user not in room should return code 40400");
        assert_eq!(json["type"], "SendMessageResult");
    }

    // S05: content 为空字符串，code=40002
    #[tokio::test]
    async fn s05_send_message_empty_content_returns_40002() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        let response = handle_send_message(
            send_message_payload(""), // 空字符串
            Some("msg-s05".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40002, "S05: empty content should return code 40002");
        assert_eq!(json["type"], "SendMessageResult");
    }

    // S06: 幂等：相同 msg_id 第二次调用 code=0，不触发第二次广播
    #[tokio::test]
    async fn s06_send_message_duplicate_msg_id_no_second_broadcast() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);

        // receiver 已在房间
        let (_recv_conn, mut rx_receiver) =
            register_connection(&registry, receiver_id, Some(room_id));

        // sender 的连接
        let (sender_conn, _rx_sender) = register_connection(&registry, sender_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        let msg_id = "msg-s06-unique-idempotent".to_string();

        // 第一次发送
        let response1 = handle_send_message(
            send_message_payload("Hello"),
            Some(msg_id.clone()),
            sender_conn,
            sender_id,
            &deps,
        )
        .await;

        // 第二次发送（相同 msg_id）
        let response2 = handle_send_message(
            send_message_payload("Hello"),
            Some(msg_id.clone()),
            sender_conn,
            sender_id,
            &deps,
        )
        .await;

        // 两次响应都是 code=0
        let json1: serde_json::Value = serde_json::from_str(&response1).unwrap();
        let json2: serde_json::Value = serde_json::from_str(&response2).unwrap();
        assert_eq!(json1["code"], 0, "S06: first send should return code=0");
        assert_eq!(json2["code"], 0, "S06: duplicate send should also return code=0");

        // rx_receiver 只收到 1 条 RoomMessage（第一次广播）
        let first_msg = tokio::time::timeout(Duration::from_millis(200), rx_receiver.recv())
            .await
            .expect("S06: should receive first broadcast")
            .expect("S06: channel should not be closed");

        let json_bc: serde_json::Value = serde_json::from_str(&first_msg).unwrap();
        assert_eq!(json_bc["type"], "RoomMessage", "S06: first broadcast should be RoomMessage");

        // 第二条不应到达（超时）
        let no_second =
            tokio::time::timeout(Duration::from_millis(100), rx_receiver.recv()).await;
        assert!(
            no_second.is_err(),
            "S06: duplicate msg_id should NOT trigger a second broadcast"
        );
    }

    // S07: 含敏感词，广播的 content 中敏感词被替换为 ***
    #[tokio::test]
    async fn s07_send_message_sensitive_word_replaced_in_broadcast() {
        use std::time::Duration;

        let room_id = Uuid::new_v4();
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);

        let (_recv_conn, mut rx_receiver) =
            register_connection(&registry, receiver_id, Some(room_id));
        let (sender_conn, _rx_sender) = register_connection(&registry, sender_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        // 发送含敏感词的消息
        handle_send_message(
            send_message_payload("Hello badword world"),
            Some("msg-s07".to_string()),
            sender_conn,
            sender_id,
            &deps,
        )
        .await;

        let msg = tokio::time::timeout(Duration::from_millis(200), rx_receiver.recv())
            .await
            .expect("S07: rx_receiver should not timeout")
            .expect("S07: channel should not be closed");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "RoomMessage");
        let content = json["payload"]["content"].as_str().expect("content should be string");
        assert!(
            !content.contains("badword"),
            "S07: sensitive word 'badword' should be replaced; got: {content}"
        );
        assert!(
            content.contains("***"),
            "S07: replaced content should contain ***; got: {content}"
        );
    }

    // S08: 响应 code=0，type="SendMessageResult"，msg_id 回写
    #[tokio::test]
    async fn s08_send_message_success_response_type_and_code() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        let response = handle_send_message(
            send_message_payload("Test message"),
            Some("msg-s08".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "S08: success response should have code=0");
        assert_eq!(json["type"], "SendMessageResult", "S08: type should be SendMessageResult");
        assert_eq!(json["msg_id"], "msg-s08", "S08: msg_id should be echoed back");
    }

    // S09: content 恰好 500 字符（边界值），成功发送
    #[tokio::test]
    async fn s09_send_message_exactly_500_chars_succeeds() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let registry = Arc::new(ConnectionRegistry::new());

        room_manager.get_or_create_room(room_id);
        let (conn_id, _rx) = register_connection(&registry, user_id, Some(room_id));
        let deps = build_send_message_deps(&room_manager, &registry);

        // 恰好 500 字符
        let content_500 = "a".repeat(500);
        let response = handle_send_message(
            send_message_payload(&content_500),
            Some("msg-s09".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "S09: exactly 500 chars should succeed with code=0");
        assert_eq!(json["type"], "SendMessageResult");
    }

    // J10: get_user_by_id 返回 Err(_) → 响应 code=50000
    //
    // 验证 I-02 修复：DB 故障时必须记录日志并返回 50000，不能静默吞掉错误。
    #[tokio::test]
    async fn j10_get_user_by_id_db_error_returns_50000() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = room_service_with(room_id); // 房间存在，跳过 step-2
        let auth_service = failing_auth_service(); // find_by_id 总返回 Err(_)
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        let response = handle_join_room(
            join_payload(room_id),
            Some("msg-j10".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 50000,
            "DB error in get_user_by_id should return code 50000; got: {}",
            json["code"]
        );
        assert_eq!(json["type"], "JoinRoomResult");
        assert_eq!(json["msg_id"], "msg-j10");
    }

    // ── PR26-02 ~ PR26-04, PR26-12: 密码房 access_token 校验（T-00026）────────

    /// 创建含密码房间的 RoomService（room_type="password"）
    fn password_room_service_with(room_id: Uuid) -> Arc<RoomService> {
        let repo = Arc::new(FakeRoomRepository::default());
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id: Uuid::new_v4(),
            title: "密码房".to_string(),
            room_type: "password".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: Some("$2b$04$hash".to_string()),
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });
        Arc::new(RoomService::new(repo))
    }

    /// 构建含有 access_token 的 JoinRoom payload
    fn join_payload_with_token(room_id: Uuid, access_token: &str) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "room_id": room_id.to_string(),
            "access_token": access_token,
        }))
    }

    // PR26-02: 带有效 token 进入密码房 → 成功（code=0）
    #[tokio::test]
    async fn pr26_02_valid_token_joins_password_room() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = password_room_service_with(room_id);
        let auth_service = auth_service_with(user_id, "TestUser");
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let secret = b"test-secret";
        let token = voice_room_shared::auth::room_access::encode_room_access_token(
            user_id, room_id, secret,
        ).expect("encode token");

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = JoinRoomDeps {
            room_manager: room_manager.clone(),
            room_service: room_service.clone(),
            auth_service: auth_service.clone(),
            registry: registry.clone(),
            stats: stats.clone(),
            jwt_secret: "test-secret".to_string(),
            kick_redis: None,
        };

        let response = handle_join_room(
            join_payload_with_token(room_id, &token),
            Some("msg-pr02".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "PR26-02: 有效 token 应成功进入，got code={}", json["code"]);
        assert_eq!(json["type"], "JoinRoomResult");
    }

    // PR26-03: 无 token 对密码房 WS JoinRoom → 40104 PASSWORD_REQUIRED
    #[tokio::test]
    async fn pr26_03_no_token_for_password_room_returns_40104() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = password_room_service_with(room_id);
        let auth_service = empty_auth_service();
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        // 不带 access_token
        let response = handle_join_room(
            join_payload(room_id),
            Some("msg-pr03".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40104,
            "PR26-03: 无 token 进入密码房应返回 40104, got {}", json["code"]
        );
    }

    // PR26-04: token 超 60s → 40105 TOKEN_EXPIRED
    #[tokio::test]
    async fn pr26_04_expired_token_returns_40105() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = password_room_service_with(room_id);
        let auth_service = empty_auth_service();
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 手动构造一个过期的 token（exp = iat - 10）
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = voice_room_shared::auth::room_access::RoomAccessClaims {
            sub: user_id.to_string(),
            room_id: room_id.to_string(),
            iat: now_secs,
            exp: now_secs - 10, // 已过期
            iss: "voiceroom-room-access".to_string(),
        };
        use jsonwebtoken::{encode, Header, EncodingKey};
        let expired_token = encode(&Header::default(), &claims, &EncodingKey::from_secret(b"test-secret"))
            .expect("encode expired token");

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        let response = handle_join_room(
            join_payload_with_token(room_id, &expired_token),
            Some("msg-pr04".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40105,
            "PR26-04: 过期 token 应返回 40105 TOKEN_EXPIRED, got {}", json["code"]
        );
    }

    // PR26-12: 为 B 房间颁发的 token 不能进入 A 房间（room_id 校验）
    #[tokio::test]
    async fn pr26_12_token_for_other_room_returns_40106() {
        let room_a_id = Uuid::new_v4();
        let room_b_id = Uuid::new_v4(); // 不同的房间
        let user_id = Uuid::new_v4();
        let room_manager = Arc::new(RoomManager::new());
        let room_service = password_room_service_with(room_a_id); // 进入 A 房间
        let auth_service = empty_auth_service();
        let registry = Arc::new(ConnectionRegistry::new());
        let stats: Arc<dyn StatsPort> = Arc::new(FakeStatsService::default());

        // 签发 B 房间的 token
        let token_for_b = voice_room_shared::auth::room_access::encode_room_access_token(
            user_id,
            room_b_id, // B 房间的 token
            b"test-secret",
        ).expect("encode token for room B");

        let (conn_id, _rx) = register_connection(&registry, user_id, None);
        let deps = build_deps(&room_manager, &room_service, &auth_service, &registry, &stats);

        // 尝试用 B 房间的 token 进入 A 房间
        let response = handle_join_room(
            join_payload_with_token(room_a_id, &token_for_b),
            Some("msg-pr12".to_string()),
            conn_id,
            user_id,
            &deps,
        )
        .await;

        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(
            json["code"], 40106,
            "PR26-12: B 房间 token 不能进入 A 房间，应返回 40106 INVALID_TOKEN, got {}", json["code"]
        );
    }
}
