//! T-00025 WS 广播工具
//!
//! - `broadcast_room_info_updated`：T-00025 房间信息更新（房间内所有连接，走统一出口）
//! - `broadcast_to_room`：P1-6 统一房间广播出口（所有信令统一走此函数 → 写入 `recent_broadcasts`
//!   环缓冲 → `last_msg_id` 重连续传基础设施）。返回服务端分配的 envelope-level `msg_id`。
//! - `build_outbound_envelope` / `build_outbound_result`：模块 8 R1 P1-7 引入的统一出站
//!   envelope 构造器，**保证每条出站消息**（包含点对点 `*Result` 与 `UserKicked`）携带
//!   `msg_id`（UUID v4）+ `timestamp`，前端可用于 ACK / 去重 / 续传基础设施。

use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::room::manager::RoomManager;
use crate::room::state::RoomState;
use crate::ws::registry::ConnectionRegistry;
use crate::ws::schema_guard;

/// 统一房间广播出口（P1-6）。
///
/// 行为：
/// 1. 服务端分配 envelope-level `msg_id`（UUID v4），写入 envelope 顶层；
/// 2. JSON 序列化整个 envelope，调用 `room_state.recent_broadcasts.push(msg_id, json)`
///    供后续重连 `last_msg_id` 续传查询；
/// 3. 调用 `registry.get_connections_in_room(room_id)` 向所有连接发送同一份 JSON。
///
/// `envelope` 必须是一个 JSON object（含 `type` / `payload` / `timestamp`）；如非 object，
/// 函数会记录 warn 并直接返回空字符串（不写缓冲、不发送）。
///
/// 返回服务端分配的 `msg_id`，调用方一般可丢弃；测试用例可断言其格式为 UUID。
pub fn broadcast_to_room(
    registry: &ConnectionRegistry,
    room_state: &RoomState,
    envelope: serde_json::Value,
) -> String {
    broadcast_to_room_inner(registry, room_state.room_id, Some(room_state), envelope)
}

/// 当房间状态不存在于内存（如治理操作发生时房间从未 JoinRoom 注册过）时使用的降级出口。
///
/// 与 [`broadcast_to_room`] 行为一致，但不写入 recent_broadcasts 环缓冲（即此条消息
/// 无法被 `last_msg_id` 续传）。仅用于 governance / 测试场景的兜底广播。
pub fn broadcast_to_room_no_state(
    registry: &ConnectionRegistry,
    room_id: Uuid,
    envelope: serde_json::Value,
) -> String {
    broadcast_to_room_inner(registry, room_id, None, envelope)
}

fn broadcast_to_room_inner(
    registry: &ConnectionRegistry,
    room_id: Uuid,
    room_state: Option<&RoomState>,
    mut envelope: serde_json::Value,
) -> String {
    let Some(obj) = envelope.as_object_mut() else {
        tracing::warn!("broadcast_to_room: envelope is not a JSON object, skip");
        return String::new();
    };

    let msg_id = Uuid::new_v4().to_string();
    obj.insert(
        "msg_id".to_string(),
        serde_json::Value::String(msg_id.clone()),
    );

    let json = match serde_json::to_string(&envelope) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("broadcast_to_room: serialize error: {e}");
            return String::new();
        }
    };

    if let Some(rs) = room_state {
        rs.recent_broadcasts.push(msg_id.clone(), json.clone());
    }

    let connections = registry.get_connections_in_room(room_id);
    let total = connections.len();
    tracing::info!(room_id=%room_id, total_connections=total, "broadcast: starting");

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;
    let mut stale_ids: Vec<Uuid> = Vec::new();

    for (conn_id, sender) in connections {
        match sender.send(json.clone()) {
            Ok(_) => {
                tracing::debug!(connection_id=%conn_id, room_id=%room_id, "broadcast: sent");
                ok_count += 1;
            }
            Err(_e) => {
                tracing::warn!(
                    connection_id=%conn_id,
                    room_id=%room_id,
                    "broadcast: receiver dropped, removing stale connection"
                );
                stale_ids.push(conn_id);
                fail_count += 1;
            }
        }
    }

    for conn_id in stale_ids {
        registry.unregister(conn_id);
    }

    tracing::info!(
        room_id=%room_id,
        sent=ok_count,
        failed=fail_count,
        "broadcast: done"
    );

    msg_id
}

/// 向 `room_id` 所在房间的所有 WS 连接广播 `RoomInfoUpdated` 信令（走统一出口）
///
/// - 房间内无连接时静默忽略
/// - `room_id` 解析失败时记录 warn 并返回
/// - 模块 8 R1 P1-5 修复：原实现绕过 [`broadcast_to_room`] 自行构造，缺失 `msg_id` 与
///   `recent_broadcasts` 入栈，违反 §6.7 重连补发契约。本版本统一走
///   [`broadcast_to_room`]（房间在内存中）或 [`broadcast_to_room_no_state`]（兜底），
///   保证 envelope 携带 `msg_id` 并写入续传缓冲。
pub fn broadcast_room_info_updated(
    registry: &ConnectionRegistry,
    room_manager: &RoomManager,
    payload: &RoomInfoUpdatedPayload,
) {
    let room_id = match Uuid::parse_str(&payload.room_id) {
        Ok(id) => id,
        Err(_) => {
            tracing::warn!(
                "broadcast_room_info_updated: invalid room_id={}",
                payload.room_id
            );
            return;
        }
    };

    let envelope = json!({
        "type": "RoomInfoUpdated",
        "payload": payload,
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });

    if let Some(rs) = room_manager.get_room(room_id) {
        broadcast_to_room(registry, &rs, envelope);
    } else {
        // 房间状态尚未注册（patch_room 立刻在房间生命周期内被调用而无任何 JoinRoom）
        // 时降级广播，不写 recent_broadcasts。
        broadcast_to_room_no_state(registry, room_id, envelope);
    }
}

/// 构造**点对点 / 单播**出站 envelope（事件类，C→S 无对应 inbound msg_id 的服务推送）。
///
/// - 模块 8 R1 P1-7 引入：保证每条出站消息**总是**携带 `msg_id`（UUID v4 服务端分配）+
///   `timestamp`，前端可基于 `msg_id` 做去重 / `processed_msg_ids` 集合工作。
/// - 不走 `recent_broadcasts`（点对点不参与 §6.7.4 续传）。
/// - 返回 `(json_string, msg_id)`：调用方一般使用 `json_string` 直接发送，`msg_id` 可用于
///   日志或测试断言。
pub fn build_outbound_envelope(type_str: &str, payload: Value) -> (String, String) {
    let msg_id = Uuid::new_v4().to_string();
    let envelope = json!({
        "type": type_str,
        "msg_id": msg_id,
        "payload": payload,
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    schema_guard::guard_outbound_envelope(&envelope);
    (
        serde_json::to_string(&envelope).unwrap_or_else(|_| String::new()),
        msg_id,
    )
}

/// 构造 **C↔S Result 类**出站 envelope（§6.3 通用 Result 格式）。
///
/// `{ type, msg_id, code, payload, timestamp }`
///
/// - `inbound_msg_id`：来自客户端请求的 msg_id（用作 ACK 关联）。若 `None` 则由服务端
///   分配 UUID v4，保证字段**永远存在**。
/// - `code`：业务错误码；`0` 表示成功。
/// - `payload`：业务字段（错误时通常包含 `message`）；`None` → 空对象 `{}`。
/// - 返回 JSON 字符串。
///
/// 模块 8 R1 P2-8 修复：错误体由顶层平铺 `code/message/timestamp` 改为
/// `payload: { message }` 包裹，与既有 `MicResult / SendGiftResult` payload 风格统一。
pub fn build_outbound_result(
    type_str: &str,
    inbound_msg_id: Option<String>,
    code: i64,
    payload: Option<Value>,
) -> String {
    let msg_id = inbound_msg_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let payload = payload.unwrap_or_else(|| json!({}));
    let envelope = json!({
        "type": type_str,
        "msg_id": msg_id,
        "code": code,
        "payload": payload,
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    schema_guard::guard_outbound_envelope(&envelope);
    serde_json::to_string(&envelope).unwrap_or_default()
}

/// `RoomInfoUpdated` WS 消息 payload
#[derive(Debug, Clone, Serialize)]
pub struct RoomInfoUpdatedPayload {
    pub room_id: String,
    pub title: String,
    pub announcement: Option<String>,
    pub category: String,
    pub cover_url: String,
    pub has_password: bool,
}

// ─── 单元测试（BR-01 ~ BR-04 + BR-05/06 R1 P1-5/P1-7）────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::manager::RoomManager;
    use std::{
        sync::{Arc, RwLock},
        time::{Duration, Instant},
    };
    use tokio::sync::mpsc;

    /// 辅助：向 registry 注册一个已加入 `room_id` 的连接，返回 receiver
    fn make_registry_with_room_conn(
        user_id: Uuid,
        room_id: Uuid,
    ) -> (Arc<ConnectionRegistry>, mpsc::UnboundedReceiver<String>) {
        use crate::ws::registry::ConnectionHandle;
        let registry = Arc::new(ConnectionRegistry::new());
        let (tx, rx) = mpsc::unbounded_channel();
        let conn_id = Uuid::new_v4();
        registry.register(ConnectionHandle {
            connection_id: conn_id,
            user_id,
            room_id: Some(room_id),
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });
        (registry, rx)
    }

    fn sample_payload(room_id: Uuid) -> RoomInfoUpdatedPayload {
        RoomInfoUpdatedPayload {
            room_id: room_id.to_string(),
            title: "新标题".to_string(),
            announcement: Some("欢迎来到新世界".to_string()),
            category: "music".to_string(),
            cover_url: "/assets/covers/night.png".to_string(),
            has_password: false,
        }
    }

    /// BR-01: 房间内连接收到 RoomInfoUpdated 消息，类型和字段正确
    #[tokio::test]
    async fn br01_connection_in_room_receives_message() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (registry, mut rx) = make_registry_with_room_conn(user_id, room_id);
        let manager = RoomManager::new();

        broadcast_room_info_updated(&registry, &manager, &sample_payload(room_id));

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should not timeout")
            .expect("channel should be open");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            json["type"], "RoomInfoUpdated",
            "BR-01: type must be RoomInfoUpdated"
        );
        assert_eq!(json["payload"]["room_id"], room_id.to_string());
        assert_eq!(json["payload"]["title"], "新标题");
        assert_eq!(json["payload"]["category"], "music");
        assert_eq!(json["payload"]["has_password"], false);
        assert!(json["timestamp"].is_number(), "timestamp must be present");
        assert!(
            json["msg_id"].is_string(),
            "BR-01 R1 P1-5: msg_id must be injected by unified outbound exit"
        );
    }

    /// BR-02: 房间内无连接时，不 panic，不阻塞
    #[tokio::test]
    async fn br02_no_connections_in_room_no_panic() {
        let registry = ConnectionRegistry::new();
        let manager = RoomManager::new();
        let room_id = Uuid::new_v4();
        let payload = sample_payload(room_id);
        // Should not panic
        broadcast_room_info_updated(&registry, &manager, &payload);
    }

    /// BR-03: `has_password: true` 时 payload 中布尔值正确（PR25-12）
    #[tokio::test]
    async fn br03_has_password_field_correct() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (registry, mut rx) = make_registry_with_room_conn(user_id, room_id);
        let manager = RoomManager::new();

        let payload = RoomInfoUpdatedPayload {
            room_id: room_id.to_string(),
            title: "锁房".to_string(),
            announcement: None,
            category: "chat".to_string(),
            cover_url: String::new(),
            has_password: true,
        };
        broadcast_room_info_updated(&registry, &manager, &payload);

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should not timeout")
            .expect("channel open");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            json["payload"]["has_password"], true,
            "PR25-12: has_password must be true for password rooms"
        );
    }

    /// BR-04: 其他房间的连接不收到消息
    #[tokio::test]
    async fn br04_connection_not_in_room_does_not_receive() {
        use crate::ws::registry::ConnectionHandle;
        let room_id = Uuid::new_v4();
        let other_room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let registry = Arc::new(ConnectionRegistry::new());
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id,
            room_id: Some(other_room_id), // different room
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        let manager = RoomManager::new();
        broadcast_room_info_updated(&registry, &manager, &sample_payload(room_id));

        let result = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(
            result.is_err(),
            "BR-04: connection in different room should NOT receive message"
        );
    }

    /// BR-05 (R1 P1-7): build_outbound_envelope 注入 msg_id (UUID v4) + timestamp
    #[test]
    fn br05_build_outbound_envelope_injects_msg_id() {
        let (json_str, msg_id) =
            build_outbound_envelope("UserKicked", json!({ "reason": "spam" }));
        assert!(Uuid::parse_str(&msg_id).is_ok(), "msg_id must be UUID v4");
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["type"], "UserKicked");
        assert_eq!(v["msg_id"], msg_id);
        assert_eq!(v["payload"]["reason"], "spam");
        assert!(v["timestamp"].is_number());
    }

    /// BR-06 (R1 P2-8): build_outbound_result payload 包裹 message，code 顶层
    #[test]
    fn br06_build_outbound_result_payload_wraps_message() {
        let json_str = build_outbound_result(
            "KickUserResult",
            Some("client-msg-1".to_string()),
            40301,
            Some(json!({ "message": "permission denied" })),
        );
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v["type"], "KickUserResult");
        assert_eq!(v["msg_id"], "client-msg-1");
        assert_eq!(v["code"], 40301);
        assert_eq!(v["payload"]["message"], "permission denied");
        assert!(v["timestamp"].is_number());
        // 顶层不再平铺 message（§6.3 对齐）
        assert!(
            v.get("message").is_none() || v["message"].is_null(),
            "P2-8: message must NOT be at envelope top level"
        );
    }

    /// BR-07 (R1 P1-7): inbound msg_id None 时服务端自动分配 UUID v4
    #[test]
    fn br07_build_outbound_result_auto_assigns_msg_id_when_none() {
        let json_str = build_outbound_result("MuteUserResult", None, 0, None);
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let msg_id = v["msg_id"].as_str().expect("msg_id must always exist");
        assert!(
            Uuid::parse_str(msg_id).is_ok(),
            "msg_id must be valid UUID when inbound is None"
        );
    }

    /// BR-08: 正常连接广播成功，connection 仍在 registry
    #[tokio::test]
    async fn br08_broadcast_success_connection_stays_in_registry() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (registry, mut rx) = make_registry_with_room_conn(user_id, room_id);

        let envelope = serde_json::json!({
            "type": "RoomMessage",
            "payload": { "content": "hello" },
            "timestamp": 1234567890i64,
        });

        let msg_id = broadcast_to_room_inner(&*registry, room_id, None, envelope);
        assert!(!msg_id.is_empty(), "BR-08: msg_id must not be empty");

        let msg = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv(),
        )
        .await
        .expect("should not timeout")
        .expect("channel should be open");
        let v: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(v["type"], "RoomMessage");

        let conns = registry.get_connections_in_room(room_id);
        assert_eq!(conns.len(), 1, "BR-08: connection must still be in registry after success");
    }

    /// BR-09: receiver drop 后广播，stale connection 被自动清理
    #[tokio::test]
    async fn br09_stale_connection_removed_after_broadcast_failure() {
        use crate::ws::registry::ConnectionHandle;

        let room_id = Uuid::new_v4();
        let registry = Arc::new(ConnectionRegistry::new());
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let conn_id = Uuid::new_v4();
        registry.register(ConnectionHandle {
            connection_id: conn_id,
            user_id: Uuid::new_v4(),
            room_id: Some(room_id),
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        drop(rx);

        let envelope = serde_json::json!({
            "type": "RoomMessage",
            "payload": { "content": "hello" },
            "timestamp": 1234567890i64,
        });

        broadcast_to_room_inner(&*registry, room_id, None, envelope);

        let conns = registry.get_connections_in_room(room_id);
        assert_eq!(
            conns.len(),
            0,
            "BR-09: stale connection must be removed from registry after broadcast failure"
        );
    }

    /// BR-10: 2 正常 + 1 stale 混合广播
    #[tokio::test]
    async fn br10_mixed_connections_stale_removed_others_receive() {
        use crate::ws::registry::ConnectionHandle;

        let room_id = Uuid::new_v4();
        let registry = Arc::new(ConnectionRegistry::new());

        let (tx1, mut rx1) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            room_id: Some(room_id),
            sender: tx1,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        let (tx2, mut rx2) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            room_id: Some(room_id),
            sender: tx2,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        let (tx3, rx3) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            room_id: Some(room_id),
            sender: tx3,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });
        drop(rx3);

        let envelope = serde_json::json!({
            "type": "RoomMessage",
            "payload": { "content": "broadcast" },
            "timestamp": 1234567890i64,
        });

        broadcast_to_room_inner(&*registry, room_id, None, envelope);

        let msg1 = tokio::time::timeout(std::time::Duration::from_millis(100), rx1.recv())
            .await
            .expect("rx1 should not timeout")
            .expect("rx1 channel should be open");
        let msg2 = tokio::time::timeout(std::time::Duration::from_millis(100), rx2.recv())
            .await
            .expect("rx2 should not timeout")
            .expect("rx2 channel should be open");

        let v1: serde_json::Value = serde_json::from_str(&msg1).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&msg2).unwrap();
        assert_eq!(v1["type"], "RoomMessage", "BR-10: conn1 should receive RoomMessage");
        assert_eq!(v2["type"], "RoomMessage", "BR-10: conn2 should receive RoomMessage");

        let remaining = registry.get_connections_in_room(room_id);
        assert_eq!(
            remaining.len(),
            2,
            "BR-10: only 2 healthy connections should remain in registry"
        );
    }
}
