//! 单连接消息处理模块
//!
//! 提供纯函数 `handle_text_message`，解析 JSON 信令并返回响应。
//! 真实的读/写 task (`handle_socket`) 构建于此之上，但测试直接调用纯函数。
//!
//! 信令格式（对齐 protocol.md §6.3 + Ping.schema.json）:
//! ```json
//! {"type":"Ping","msg_id":"uuid","payload":{},"timestamp":1700000000000}
//! ```

use std::sync::{Arc, RwLock};
use std::time::Instant;

use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::registry::{ConnectionHandle, ConnectionRegistry};
use crate::core::analytics::writer::EventWriterPort;
use crate::modules::auth::service::AuthService;
use crate::modules::events::ws::{handle_report_event, ReportEventDeps};
use crate::modules::gift::send_gift::{handle_send_gift, SendGiftDeps, SendGiftServicePort};
use crate::modules::governance::force_mic::{
    handle_force_leave_mic, handle_force_take_mic, ForceLeaveMicDeps, ForceTakeMicDeps,
};
use crate::modules::governance::kick::{handle_kick, KickAuditDb, KickDeps, KickRedis};
use crate::modules::governance::mute::{handle_mute, handle_unmute, MuteDb, MuteDeps, MuteRedis};
use crate::modules::governance::transfer::{
    handle_transfer_admin, TransferAdminDeps, TransferAdminRepo,
};
use crate::modules::nobility::NobilityServicePort;
use crate::modules::room::service::RoomService;
use crate::room::handler::do_leave_room;
use crate::room::handler::{JoinRoomDeps, LeaveRoomDeps};
use crate::room::mic_lock::MicLock;
use crate::room::RoomManager;
use crate::stats::StatsPort;

// ─── 信令数据结构 ─────────────────────────────────────────────────────────────

/// 客户端下行消息（服务端接收）
///
/// `#[serde(deny_unknown_fields)]` — 拒绝含未知字段的消息，防止字段注入。
/// PROTO-BINDING: doc/protocol/schemas/ws/Ping.schema.json (JoinRoom/LeaveMic/…对应各自 schema)
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomingMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub msg_id: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub timestamp: Option<i64>,
}

/// 服务端上行消息（服务端发送）
#[derive(Debug, Serialize)]
pub struct OutgoingMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    pub timestamp: i64,
}

// ─── 纯函数：消息处理核心逻辑 ─────────────────────────────────────────────────

/// 解析一条文本信令，更新心跳（如有），返回待发送的响应 JSON 字符串。
///
/// - `"ping"` → 更新 last_heartbeat，回复 `"pong"`（msg_id 一致）
/// - 其他类型 → 记录警告，返回 `None`（不 panic）
pub fn handle_text_message(text: &str, last_heartbeat: &Arc<RwLock<Instant>>) -> Option<String> {
    let msg: IncomingMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "failed to parse incoming ws message");
            return None;
        }
    };

    match msg.msg_type.as_str() {
        "ping" => {
            // [DEPRECATED] 旧格式 ping，兼容期保留。将在下一主版本移除。
            // PROTO-BINDING: doc/protocol/schemas/ws/Ping.schema.json
            tracing::warn!(
                msg_type = "ping",
                "DEPRECATED: received lowercase 'ping', use 'Ping' (PascalCase). Will be removed in next major version."
            );
            // 更新心跳时间戳
            match last_heartbeat.write() {
                Ok(mut guard) => *guard = Instant::now(),
                Err(e) => tracing::error!("heartbeat lock poisoned: {e}"),
            }

            let pong = OutgoingMessage {
                msg_type: "pong".to_string(),
                msg_id: msg.msg_id,
                payload: None,
                timestamp: chrono::Utc::now().timestamp_millis(),
            };

            serde_json::to_string(&pong).ok()
        }
        "Ping" => {
            // 主格式：大写 Ping 信令
            // PROTO-BINDING: doc/protocol/schemas/ws/Ping.schema.json
            // PROTO-BINDING: doc/protocol/schemas/ws/Pong.schema.json (msg_id required)
            match last_heartbeat.write() {
                Ok(mut guard) => *guard = Instant::now(),
                Err(e) => tracing::error!("heartbeat lock poisoned: {e}"),
            }

            // 如果客户端未提供 msg_id，服务端生成 UUID 保证 Pong.msg_id 合规
            let pong_msg_id = msg.msg_id.unwrap_or_else(|| Uuid::new_v4().to_string());
            let pong = OutgoingMessage {
                msg_type: "Pong".to_string(),
                msg_id: Some(pong_msg_id),
                payload: None,
                timestamp: chrono::Utc::now().timestamp_millis(),
            };

            serde_json::to_string(&pong).ok()
        }
        unknown => {
            tracing::warn!(msg_type = unknown, "unknown ws message type received");
            None
        }
    }
}

/// 兼容期双发响应：对 Ping 信令在 dev 模式下同时返回新格式 Pong 和旧格式 pong。
///
/// - Release: 只返回新格式 `[Pong]`
/// - Debug:   同时返回 `[Pong, pong]`（兼容旧版客户端）
///
/// 用于 T-00103 PING-COMPAT-2 测试。
pub fn ping_pong_responses(msg_id: Option<String>) -> Vec<String> {
    let ts = chrono::Utc::now().timestamp_millis();
    // PROTO-BINDING: doc/protocol/schemas/ws/Pong.schema.json (msg_id required)
    // 如果客户端未提供 msg_id，服务端生成 UUID 保证 Pong 合规
    let msg_id_val = msg_id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let pong_new = OutgoingMessage {
        msg_type: "Pong".to_string(),
        msg_id: Some(msg_id_val.clone()),
        payload: None,
        timestamp: ts,
    };
    let pong_legacy = OutgoingMessage {
        msg_type: "pong".to_string(),
        msg_id: Some(msg_id_val),
        payload: None,
        timestamp: ts,
    };

    #[cfg(debug_assertions)]
    {
        vec![
            serde_json::to_string(&pong_new).unwrap_or_default(),
            serde_json::to_string(&pong_legacy).unwrap_or_default(),
        ]
    }
    #[cfg(not(debug_assertions))]
    {
        vec![serde_json::to_string(&pong_new).unwrap_or_default()]
    }
}

// ─── 连接生命周期 task ────────────────────────────────────────────────────────

/// handle_socket 所需的全部服务依赖，降低参数数量
pub struct SocketDeps {
    pub registry: Arc<ConnectionRegistry>,
    pub stats: Arc<dyn StatsPort>,
    pub room_manager: Arc<RoomManager>,
    pub room_service: Arc<RoomService>,
    pub auth_service: Arc<AuthService>,
    pub send_gift_service: Arc<dyn SendGiftServicePort>,
    pub event_writer: Arc<dyn EventWriterPort>,
    /// JWT 密钥（T-00026 room access token 验证用）
    pub jwt_secret: String,
    /// 踢人冷却 Redis（T-00028 KickUser 信令 + JoinRoom 前置检查）
    pub kick_redis: Arc<dyn KickRedis>,
    /// 踢人审计 DB（T-00028 KickUser 信令）
    pub kick_audit_db: Arc<dyn KickAuditDb>,
    /// 禁麦/禁言 Redis（T-00029 MuteUser/UnmuteUser 信令 + 前置拦截）
    pub mute_redis: Arc<dyn MuteRedis>,
    /// 禁麦/禁言审计 DB（T-00029 MuteUser 信令）
    pub mute_db: Arc<dyn MuteDb>,
    /// 抢麦分布式锁（T-00014 #4 / P2-12）
    pub mic_lock: Arc<dyn MicLock>,
    /// 管理员任命 DB（T-00030 TransferAdmin 信令）
    pub transfer_admin_repo: Arc<dyn TransferAdminRepo>,
    /// 聊天消息持久化（T-00043 SendMessage）
    pub chat_repo: Arc<dyn crate::modules::chat::ChatRepository>,
    /// 贵族服务（T-00069 UserJoined 广播携带 noble 字段）
    pub nobility_service: Arc<dyn NobilityServicePort>,
}

/// 在成功升级的 WebSocket 上运行完整的读/写生命周期。
///
/// 每次调用生成独立的 `connection_id`（与 user_id 解耦），
/// 注销时仅删除自己的条目，不影响同一用户的其他连接。
/// 参数数量超过 7 个：系统边界层函数，聚合全部 WS 服务依赖，抑制 Clippy 警告。
#[allow(clippy::too_many_arguments)]
pub async fn handle_socket(
    socket: WebSocket,
    user_id: Uuid,
    registry: Arc<ConnectionRegistry>,
    stats: Arc<dyn StatsPort>,
    room_manager: Arc<RoomManager>,
    room_service: Arc<RoomService>,
    auth_service: Arc<AuthService>,
    send_gift_service: Arc<dyn SendGiftServicePort>,
    event_writer: Arc<dyn EventWriterPort>,
    jwt_secret: String,
    kick_redis: Arc<dyn KickRedis>,
    kick_audit_db: Arc<dyn KickAuditDb>,
    mute_redis: Arc<dyn MuteRedis>,
    mute_db: Arc<dyn MuteDb>,
    mic_lock: Arc<dyn MicLock>,
    transfer_admin_repo: Arc<dyn TransferAdminRepo>,
    chat_repo: Arc<dyn crate::modules::chat::ChatRepository>,
    nobility_service: Arc<dyn NobilityServicePort>,
) {
    let connection_id = Uuid::new_v4(); // 每次連接生成唯一 ID，與 user_id 解耦
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let last_heartbeat = Arc::new(RwLock::new(Instant::now()));

    let handle = ConnectionHandle {
        connection_id,
        user_id,
        room_id: None,
        sender: tx,
        last_heartbeat: last_heartbeat.clone(),
    };
    registry.register(handle);

    // 用戶上線統計（HyperLogLog PFADD，失敗不影響主流程）
    stats.user_online(user_id).await.ok();

    let mut socket = socket;

    loop {
        tokio::select! {
            // 出站：從 mpsc channel 轉發到 WebSocket
            msg = rx.recv() => {
                match msg {
                    Some(text) => {
                        // T-00041：心跳超时通知帧 → 转发后立即发显式 Close(1000) 并断开
                        let close_frame = crate::ws::heartbeat::close_frame_for_message(&text);
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                        if let Some(frame) = close_frame {
                            tracing::warn!(
                                %user_id,
                                %connection_id,
                                "heartbeat timeout: sending Close(1000) and terminating connection"
                            );
                            // 尽力发送 Close 帧；忽略错误（对端可能已断）
                            let _ = socket.send(Message::Close(Some(frame))).await;
                            break;
                        }
                    }
                    None => break,
                }
            }
            // 入站：處理客戶端消息
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let text_str = text.as_str();
                        // P2-10: 表驱动 dispatch —— 将原 13 路 `else if msg_type == "..."` 链
                        // 改为 `match incoming.msg_type.as_str()` 单层 match，每条信令独立 arm，
                        // 嵌套层级从 7 降到 4，新增信令只需新增 arm（不再修改长链）。
                        let response_opt: Option<String> = if let Ok(incoming) =
                            serde_json::from_str::<IncomingMessage>(text_str)
                        {
                            match incoming.msg_type.as_str() {
                                "JoinRoom" => {
                                    let deps = JoinRoomDeps {
                                        room_manager: room_manager.clone(),
                                        room_service: room_service.clone(),
                                        auth_service: auth_service.clone(),
                                        registry: registry.clone(),
                                        stats: stats.clone(),
                                        jwt_secret: jwt_secret.clone(),
                                        kick_redis: Some(kick_redis.clone()),
                                        nobility_service: Some(nobility_service.clone()),
                                        global_broadcast: None,
                                    };
                                    Some(
                                        crate::room::handler::handle_join_room(
                                            incoming.payload,
                                            incoming.msg_id,
                                            connection_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "LeaveRoom" => {
                                    let deps = LeaveRoomDeps {
                                        room_manager: room_manager.clone(),
                                        registry: registry.clone(),
                                        stats: stats.clone(),
                                        mic_lock: Some(mic_lock.clone()),
                                    };
                                    Some(
                                        crate::room::handler::handle_leave_room(
                                            incoming.msg_id,
                                            connection_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "TakeMic" => {
                                    let deps = crate::room::handler::TakeMicDeps {
                                        room_manager: room_manager.clone(),
                                        registry: registry.clone(),
                                        mute_redis: Some(mute_redis.clone()),
                                        mic_lock: Some(mic_lock.clone()),
                                    };
                                    Some(
                                        crate::room::handler::handle_take_mic(
                                            incoming.payload,
                                            incoming.msg_id,
                                            connection_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "LeaveMic" => {
                                    let deps = crate::room::handler::LeaveMicDeps {
                                        room_manager: room_manager.clone(),
                                        registry: registry.clone(),
                                        mic_lock: Some(mic_lock.clone()),
                                    };
                                    Some(
                                        crate::room::handler::handle_leave_mic(
                                            incoming.msg_id,
                                            connection_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "SendMessage" => {
                                    let deps = crate::room::handler::SendMessageDeps {
                                        room_manager: room_manager.clone(),
                                        registry: registry.clone(),
                                        mute_redis: Some(mute_redis.clone()),
                                        chat_repo: Some(chat_repo.clone()),
                                    };
                                    Some(
                                        crate::room::handler::handle_send_message(
                                            incoming.payload,
                                            incoming.msg_id,
                                            connection_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "SendGift" => {
                                    let deps = SendGiftDeps {
                                        send_gift_service: send_gift_service.clone(),
                                        registry: registry.clone(),
                                    };
                                    Some(
                                        handle_send_gift(
                                            incoming.payload,
                                            incoming.msg_id,
                                            connection_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "ReportEvent" => {
                                    let deps = ReportEventDeps {
                                        event_writer: event_writer.clone(),
                                    };
                                    Some(
                                        handle_report_event(
                                            incoming.payload,
                                            incoming.msg_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "KickUser" => {
                                    // T-00028: KickUser 信令处理
                                    let deps = KickDeps {
                                        room_manager: room_manager.clone(),
                                        room_service: room_service.clone(),
                                        redis: kick_redis.clone(),
                                        audit_db: kick_audit_db.clone(),
                                        registry: registry.clone(),
                                    };
                                    Some(
                                        handle_kick(
                                            incoming.payload,
                                            incoming.msg_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "MuteUser" => {
                                    // T-00029: MuteUser 信令处理
                                    let deps = MuteDeps {
                                        room_manager: room_manager.clone(),
                                        room_service: room_service.clone(),
                                        mute_redis: mute_redis.clone(),
                                        mute_db: mute_db.clone(),
                                        registry: registry.clone(),
                                    };
                                    Some(
                                        handle_mute(
                                            incoming.payload,
                                            incoming.msg_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "UnmuteUser" => {
                                    // T-00029: UnmuteUser 信令处理
                                    let deps = MuteDeps {
                                        room_manager: room_manager.clone(),
                                        room_service: room_service.clone(),
                                        mute_redis: mute_redis.clone(),
                                        mute_db: mute_db.clone(),
                                        registry: registry.clone(),
                                    };
                                    Some(
                                        handle_unmute(
                                            incoming.payload,
                                            incoming.msg_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "TransferAdmin" => {
                                    // T-00030: TransferAdmin 信令处理
                                    let deps = TransferAdminDeps {
                                        room_manager: room_manager.clone(),
                                        room_service: room_service.clone(),
                                        room_repo: transfer_admin_repo.clone(),
                                        registry: registry.clone(),
                                    };
                                    Some(
                                        handle_transfer_admin(
                                            incoming.payload,
                                            incoming.msg_id,
                                            user_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "ForceTakeMic" => {
                                    // T-00030: ForceTakeMic 信令处理
                                    // room_id 来自 session context（不从 payload 读取，schema additionalProperties: false）
                                    let operator_room_id = registry.get_room_id(connection_id);
                                    let deps = ForceTakeMicDeps {
                                        room_manager: room_manager.clone(),
                                        room_service: room_service.clone(),
                                        mute_redis: mute_redis.clone(),
                                        registry: registry.clone(),
                                    };
                                    Some(
                                        handle_force_take_mic(
                                            incoming.payload,
                                            incoming.msg_id,
                                            user_id,
                                            operator_room_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                "ForceLeaveMic" => {
                                    // T-00030: ForceLeaveMic 信令处理
                                    // room_id 来自 session context（不从 payload 读取，schema additionalProperties: false）
                                    let operator_room_id = registry.get_room_id(connection_id);
                                    let deps = ForceLeaveMicDeps {
                                        room_manager: room_manager.clone(),
                                        room_service: room_service.clone(),
                                        registry: registry.clone(),
                                    };
                                    Some(
                                        handle_force_leave_mic(
                                            incoming.payload,
                                            incoming.msg_id,
                                            user_id,
                                            operator_room_id,
                                            &deps,
                                        )
                                        .await,
                                    )
                                }
                                // ── PING-COMPAT: TDS §一.3 — dev 双发，release 单发 ──────
                                // "Ping" / "ping" 由 ping_pong_responses 处理，接入真实链路
                                // 不走 handle_text_message fallback（后者只能单发）
                                "Ping" => {
                                    // 主格式：更新心跳，双发 Pong + pong（debug）/ 单发 Pong（release）
                                    match last_heartbeat.write() {
                                        Ok(mut guard) => *guard = Instant::now(),
                                        Err(e) => {
                                            tracing::error!("heartbeat lock poisoned: {e}")
                                        }
                                    }
                                    for resp in ping_pong_responses(incoming.msg_id.clone()) {
                                        if !registry.send_to(connection_id, &resp) {
                                            tracing::warn!(
                                                %connection_id,
                                                "failed to send Pong response"
                                            );
                                        }
                                    }
                                    None // responses already dispatched via registry
                                }
                                "ping" => {
                                    // [DEPRECATED] 旧格式 ping，兼容期保留，下一主版本移除
                                    tracing::warn!(
                                        msg_type = "ping",
                                        "DEPRECATED: received lowercase 'ping', \
                                         use 'Ping' (PascalCase). \
                                         Will be removed in next major version."
                                    );
                                    match last_heartbeat.write() {
                                        Ok(mut guard) => *guard = Instant::now(),
                                        Err(e) => {
                                            tracing::error!("heartbeat lock poisoned: {e}")
                                        }
                                    }
                                    for resp in ping_pong_responses(incoming.msg_id.clone()) {
                                        if !registry.send_to(connection_id, &resp) {
                                            tracing::warn!(
                                                %connection_id,
                                                "failed to send pong response"
                                            );
                                        }
                                    }
                                    None // responses already dispatched via registry
                                }
                                // 其他未知类型
                                _ => handle_text_message(text_str, &last_heartbeat),
                            }
                        } else {
                            // 解析失败也走纯函数路径（保留原有兼容行为）
                            handle_text_message(text_str, &last_heartbeat)
                        };

                        if let Some(response) = response_opt {
                            if !registry.send_to(connection_id, &response) {
                                tracing::warn!(
                                    %connection_id,
                                    "failed to send signal response"
                                );
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {} // Binary / Ping / Pong 幀
                }
            }
        }
    }

    // 断线时自动离开房间（必须在 stats.user_offline 和 registry.unregister 之前调用）
    let leave_deps = LeaveRoomDeps {
        room_manager: room_manager.clone(),
        registry: registry.clone(),
        stats: stats.clone(),
        mic_lock: Some(mic_lock.clone()),
    };
    do_leave_room(connection_id, user_id, &leave_deps).await;

    // 用戶下線統計（HLL append-only no-op，失敗不影響主流程）
    stats.user_offline(user_id).await.ok();

    // 僅注銷本連接的 connection_id，不影響同一用戶的其他連接
    registry.unregister(connection_id);
    tracing::info!(%user_id, %connection_id, "websocket connection closed");
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;

    fn fresh_heartbeat() -> Arc<RwLock<Instant>> {
        Arc::new(RwLock::new(Instant::now()))
    }

    // C01: ping 消息触发 pong 响应，msg_id 一致
    #[tokio::test]
    async fn c01_ping_triggers_pong_with_same_msg_id() {
        let hb = fresh_heartbeat();
        let ping_json = r#"{"type":"ping","msg_id":"test-msg-id-abc123"}"#;

        let response = handle_text_message(ping_json, &hb);

        assert!(response.is_some(), "ping should produce a pong response");

        let resp_str = response.unwrap();
        let resp: serde_json::Value =
            serde_json::from_str(&resp_str).expect("response must be valid JSON");

        assert_eq!(resp["type"], "pong", "response type should be pong");
        assert_eq!(
            resp["msg_id"], "test-msg-id-abc123",
            "pong msg_id must match ping msg_id"
        );
    }

    // C02: 未知消息类型不 panic，返回 None
    #[tokio::test]
    async fn c02_unknown_message_type_no_panic() {
        let hb = fresh_heartbeat();
        let unknown_json = r#"{"type":"some_future_event","msg_id":"xyz","payload":{"foo":1}}"#;

        // 如果 panic 则测试直接失败；正常情况返回 None
        let result = handle_text_message(unknown_json, &hb);

        assert!(
            result.is_none(),
            "unknown message type should return None, not panic"
        );
    }

    // C03: ping 消息更新 last_heartbeat
    #[tokio::test]
    async fn c03_ping_updates_last_heartbeat() {
        use std::time::Duration;

        let old_instant = Instant::now() - Duration::from_secs(20);
        let hb = Arc::new(RwLock::new(old_instant));

        let ping_json = r#"{"type":"ping","msg_id":"hb-test"}"#;
        let _ = handle_text_message(ping_json, &hb);

        let elapsed = Instant::now().duration_since(*hb.read().unwrap());
        assert!(
            elapsed < Duration::from_secs(1),
            "last_heartbeat should be updated to near-now after ping"
        );
    }

    // C04: 格式非法的 JSON 不 panic，返回 None
    #[tokio::test]
    async fn c04_malformed_json_no_panic() {
        let hb = fresh_heartbeat();
        let bad_json = r#"{not valid json"#;

        let result = handle_text_message(bad_json, &hb);
        assert!(
            result.is_none(),
            "malformed JSON should return None, not panic"
        );
    }
}
