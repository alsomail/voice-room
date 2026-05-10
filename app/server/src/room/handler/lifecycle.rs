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
use crate::modules::nobility::NobilityServicePort;
use crate::modules::room::service::RoomService;
use crate::room::mic_lock::MicLock;
use crate::stats::StatsPort;
use crate::ws::registry::ConnectionRegistry;

use crate::room::manager::RoomManager;
use crate::room::state::MemberInfo;

use super::mic::broadcast_mic_left;

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
    /// 贵族服务（T-00069 UserJoined 广播携带 noble 字段）；None = 跳过
    pub nobility_service: Option<Arc<dyn NobilityServicePort>>,
}

// ─── handle_join_room ────────────────────────────────────────────────────────

/// 处理 JoinRoom 信令，返回 JSON 字符串响应。
///
/// 所有重依赖通过 `deps: &JoinRoomDeps` 传入，调用方保持所有权。
///
// PROTO-BINDING: doc/protocol/schemas/ws/JoinRoom.schema.json (C→S)
// PROTO-BINDING: doc/protocol/schemas/ws/UserJoined.schema.json (S→Room broadcast)
// PROTO-BINDING: doc/protocol/schemas/ws/JoinRoomResult.schema.json (S→C result)
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
        nobility_service,
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

    // ── 5.5 P1-6: last_msg_id 重连续传 — 在加入成员表 / 广播 UserJoined 之前回放 ──
    // 只回放给当前这条 connection（避免污染房内其他连接），且不写入 recent_broadcasts。
    if let Some(last_id) = payload
        .as_ref()
        .and_then(|p| p.get("last_msg_id"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        if let Some(missed) = room_state.recent_broadcasts.replay_after(last_id) {
            for entry in missed {
                if !registry.send_to(connection_id, &entry.json) {
                    tracing::warn!(%connection_id, "replay send failed (channel closed)");
                    break;
                }
            }
        } else {
            tracing::info!(
                %connection_id, %room_id, last_msg_id = %last_id,
                "last_msg_id out of replay window, skipping replay"
            );
        }
    }

    // ── 6. 加入成员表 ──────────────────────────────────────────────────────────
    room_state.members.insert(
        user_id,
        MemberInfo::new(user_id, nickname.clone(), avatar.clone()),
    );

    // ── 7. 标记连接所属房间 ─────────────────────────────────────────────────────
    registry.set_room_id(connection_id, room_id);

    // ── 7. 统计：活跃房间 ──────────────────────────────────────────────────────
    stats.user_join_room(room_id).await.ok();

    // ── 8. 广播 UserJoined 给房间内所有连接（含自己）— 走统一出口 broadcast_to_room ──
    // PROTO-BINDING: doc/protocol/schemas/ws/UserJoined.schema.json
    // T-00069: 若用户有贵族，附带 noble 字段
    let noble_dto = if let Some(svc) = nobility_service.as_ref() {
        svc.get_user_noble_dto(user_id).await
    } else {
        None
    };

    let joined_envelope = if let Some(noble) = noble_dto {
        serde_json::json!({
            "type": "UserJoined",
            "payload": {
                "user_id": user_id.to_string(),
                "nickname": nickname,
                "avatar": avatar,
                "noble": noble,
            },
            "timestamp": chrono::Utc::now().timestamp_millis(),
        })
    } else {
        serde_json::json!({
            "type": "UserJoined",
            "payload": {
                "user_id": user_id.to_string(),
                "nickname": nickname,
                "avatar": avatar,
            },
            "timestamp": chrono::Utc::now().timestamp_millis(),
        })
    };
    crate::ws::broadcaster::broadcast_to_room(registry, &room_state, joined_envelope);

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
        "timestamp": chrono::Utc::now().timestamp_millis(),
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
    /// 抢麦分布式锁（T-00014 #4）；None = 跳过释放
    pub mic_lock: Option<Arc<dyn MicLock>>,
}

// ─── do_leave_room ───────────────────────────────────────────────────────────

/// 核心离开逻辑（9 步），主动离开与断线共享此路径。
///
/// 任何静默返回点均视为非错误（用户不在房间是正常状态）。
pub async fn do_leave_room(connection_id: Uuid, user_id: Uuid, deps: &LeaveRoomDeps) {
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

    // 4.1 释放分布式麦位锁（best-effort：锁已过期时忽略错误）
    if let (Some(mic_index), Some(ref ml)) = (left_mic_index, &deps.mic_lock) {
        if let Err(e) = ml.release(room_id, mic_index).await {
            tracing::warn!(error=?e, %room_id, mic_index, "mic_lock release on disconnect failed (best-effort, ignored)");
        }
    }

    // 5. 先清除 room_id，使后续广播时自然排除离开者
    deps.registry.clear_room_id(connection_id);

    // 6. 统计：活跃房间（失败不阻断主流程）
    //    P1-4: 僅當房間人數變為 0 時才從 active_rooms 集合移除
    let remaining = room_state.member_count();
    deps.stats.user_leave_room(room_id, remaining).await.ok();

    // 7. 广播 UserLeft 给房间剩余成员（离开者已被步骤 5 排除）— 统一出口
    // PROTO-BINDING: doc/protocol/schemas/ws/UserLeft.schema.json (S→Room broadcast)
    let user_left_envelope = serde_json::json!({
        "type": "UserLeft",
        "payload": { "user_id": user_id.to_string() },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    crate::ws::broadcaster::broadcast_to_room(&deps.registry, &room_state, user_left_envelope);

    // 4'（被动下麦广播）：在 clear_room_id 之后广播 MicLeft，离开者已被排除
    if let Some(mic_index) = left_mic_index {
        broadcast_mic_left(&deps.registry, &room_state, mic_index, user_id, false);
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
///
// PROTO-BINDING: doc/protocol/schemas/ws/LeaveRoom.schema.json (C→S)
// PROTO-BINDING: doc/protocol/schemas/ws/LeaveRoomResult.schema.json (S→C result)
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
        "timestamp": chrono::Utc::now().timestamp_millis(),
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
        "timestamp": chrono::Utc::now().timestamp_millis(),
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
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}
