//! T-00025 WS 广播工具
//!
//! 向房间内所有连接广播 `RoomInfoUpdated` 消息（S→C）

use serde::Serialize;
use uuid::Uuid;

use crate::ws::registry::ConnectionRegistry;

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

/// 向 `room_id` 所在房间的所有 WS 连接广播 `RoomInfoUpdated` 信令
///
/// - 房间内无连接时静默忽略
/// - `room_id` 解析失败时记录 warn 并返回
pub fn broadcast_room_info_updated(
    registry: &ConnectionRegistry,
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

    let msg = match serde_json::to_string(&BroadcastEnvelope {
        msg_type: "RoomInfoUpdated",
        payload,
        timestamp: chrono::Utc::now().timestamp(),
    }) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("broadcast_room_info_updated: serialize error: {e}");
            return;
        }
    };

    for (_, sender) in registry.get_connections_in_room(room_id) {
        let _ = sender.send(msg.clone());
    }
}

/// WS 消息外层包装（type / payload / timestamp）
#[derive(Serialize)]
struct BroadcastEnvelope<'a> {
    #[serde(rename = "type")]
    msg_type: &'a str,
    payload: &'a RoomInfoUpdatedPayload,
    timestamp: i64,
}

// ─── 单元测试（BR-01 ~ BR-04）────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
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

        broadcast_room_info_updated(&registry, &sample_payload(room_id));

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should not timeout")
            .expect("channel should be open");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "RoomInfoUpdated", "BR-01: type must be RoomInfoUpdated");
        assert_eq!(json["payload"]["room_id"], room_id.to_string());
        assert_eq!(json["payload"]["title"], "新标题");
        assert_eq!(json["payload"]["category"], "music");
        assert_eq!(json["payload"]["has_password"], false);
        assert!(json["timestamp"].is_number(), "timestamp must be present");
    }

    /// BR-02: 房间内无连接时，不 panic，不阻塞
    #[tokio::test]
    async fn br02_no_connections_in_room_no_panic() {
        let registry = ConnectionRegistry::new();
        let room_id = Uuid::new_v4();
        let payload = sample_payload(room_id);
        // Should not panic
        broadcast_room_info_updated(&registry, &payload);
    }

    /// BR-03: `has_password: true` 时 payload 中布尔值正确（PR25-12）
    #[tokio::test]
    async fn br03_has_password_field_correct() {
        let room_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let (registry, mut rx) = make_registry_with_room_conn(user_id, room_id);

        let payload = RoomInfoUpdatedPayload {
            room_id: room_id.to_string(),
            title: "锁房".to_string(),
            announcement: None,
            category: "chat".to_string(),
            cover_url: String::new(),
            has_password: true,
        };
        broadcast_room_info_updated(&registry, &payload);

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should not timeout")
            .expect("channel open");

        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            json["payload"]["has_password"],
            true,
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

        broadcast_room_info_updated(&registry, &sample_payload(room_id));

        let result =
            tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(
            result.is_err(),
            "BR-04: connection in different room should NOT receive message"
        );
    }
}
