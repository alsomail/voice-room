//! 管理事件处理器
//!
//! `handle_admin_event` 根据 `AdminEvent` 类型分发到对应处理逻辑：
//! - `BanUser`         → 发送封禁通知 → 断开连接
//! - `CloseRoom`       → 向房间所有成员广播关闭消息 → 批量断开
//! - `BroadcastNotice` → 向所有在线连接广播公告
//!
//! 所有发送操作使用 `let _ = ...` 忽略 channel 关闭错误，保证不 panic。

use std::sync::Arc;

use crate::events::admin_event::AdminEvent;
use crate::ws::ConnectionRegistry;

// ─── 通知 JSON 构造函数 ───────────────────────────────────────────────────────

/// 封禁通知消息 JSON
fn ban_notification_json() -> String {
    r#"{"type":"ban_user","message":"You have been banned from this platform."}"#.to_string()
}

/// 房间关闭通知消息 JSON
fn room_closed_json() -> String {
    r#"{"type":"close_room","message":"This room has been closed by an administrator."}"#
        .to_string()
}

/// 系统公告消息 JSON
fn notice_json(message: String) -> String {
    serde_json::json!({"type": "broadcast_notice", "message": message}).to_string()
}

// ─── 事件分发入口 ─────────────────────────────────────────────────────────────

/// 处理一个 AdminEvent，对 registry 执行相应操作。
///
/// 所有操作均为尽力投递（best-effort）：
/// - channel 已关闭：静默跳过（不 panic，不影响主服务）
/// - 用户/房间不在线：静默忽略
pub async fn handle_admin_event(event: AdminEvent, registry: Arc<ConnectionRegistry>) {
    match event {
        AdminEvent::BanUser { payload, .. } => {
            let conns = registry.get_by_user_id(payload.user_id);
            for (conn_id, sender) in conns {
                let _ = sender.send(ban_notification_json());
                registry.unregister(conn_id);
            }
        }
        AdminEvent::CloseRoom { payload, .. } => {
            let conns = registry.get_connections_in_room(payload.room_id);
            for (_, sender) in &conns {
                let _ = sender.send(room_closed_json());
            }
            for (conn_id, _) in conns {
                registry.unregister(conn_id);
            }
        }
        AdminEvent::BroadcastNotice { payload, .. } => {
            registry.broadcast_to_all(&notice_json(payload.message));
        }
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};
    use std::time::{Duration, Instant};

    use tokio::sync::mpsc;
    use uuid::Uuid;

    use crate::events::admin_event::{
        AdminEvent, BanUserPayload, BroadcastNoticePayload, CloseRoomPayload,
    };
    use crate::ws::registry::{ConnectionHandle, ConnectionRegistry};

    // ─── 测试辅助函数 ─────────────────────────────────────────────────────────

    /// 创建一个测试用 ConnectionHandle，返回 (handle, receiver)
    fn make_handle(
        user_id: Uuid,
        room_id: Option<Uuid>,
    ) -> (ConnectionHandle, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let handle = ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id,
            room_id,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        };
        (handle, rx)
    }

    /// 创建 ban_user 事件
    fn ban_user_event(user_id: Uuid) -> AdminEvent {
        AdminEvent::BanUser {
            payload: BanUserPayload { user_id },
            admin_id: Uuid::new_v4(),
            ts: 1700000000,
        }
    }

    /// 创建 close_room 事件
    fn close_room_event(room_id: Uuid) -> AdminEvent {
        AdminEvent::CloseRoom {
            payload: CloseRoomPayload { room_id },
            admin_id: Uuid::new_v4(),
            ts: 1700000001,
        }
    }

    /// 创建 broadcast_notice 事件
    fn broadcast_notice_event(message: &str) -> AdminEvent {
        AdminEvent::BroadcastNotice {
            payload: BroadcastNoticePayload {
                message: message.to_string(),
            },
            admin_id: Uuid::new_v4(),
            ts: 1700000002,
        }
    }

    // ─── E01: ban_user 发送封禁消息 ───────────────────────────────────────────

    /// E01: ban_user 事件 → 找到用户连接 → 发送封禁消息
    #[tokio::test]
    async fn e01_ban_user_sends_notification_to_connection() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();

        let (handle, mut rx) = make_handle(user_id, None);
        registry.register(handle);

        let event = ban_user_event(user_id);
        super::handle_admin_event(event, registry.clone()).await;

        // 接收端必须收到封禁通知
        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should not time out waiting for ban notification")
            .expect("channel should not be closed before receiving ban notification");

        let json: serde_json::Value =
            serde_json::from_str(&msg).expect("ban notification should be valid JSON");
        assert_eq!(
            json["type"], "ban_user",
            "ban notification must have type=ban_user"
        );
    }

    // ─── E02: ban_user 发送后注销连接 ────────────────────────────────────────

    /// E02: ban_user 事件 → 发送消息后注销连接
    #[tokio::test]
    async fn e02_ban_user_disconnects_connection() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();

        let (handle, _rx) = make_handle(user_id, None);
        let conn_id = handle.connection_id;
        registry.register(handle);

        assert_eq!(registry.count(), 1, "should have 1 connection before ban");

        let event = ban_user_event(user_id);
        super::handle_admin_event(event, registry.clone()).await;

        assert_eq!(
            registry.count(),
            0,
            "connection should be unregistered after ban_user event"
        );
        assert!(
            registry.get(conn_id).is_none(),
            "specific connection_id should no longer exist in registry"
        );
    }

    // ─── E03: ban_user 用户不在线无 panic ────────────────────────────────────

    /// E03: ban_user 事件 → 用户不在线时无 panic
    #[tokio::test]
    async fn e03_ban_user_offline_user_no_panic() {
        let registry = Arc::new(ConnectionRegistry::new());
        let non_existent_user = Uuid::new_v4();

        // registry 为空，用户不在线
        let event = ban_user_event(non_existent_user);

        // 不应 panic，正常完成
        super::handle_admin_event(event, registry.clone()).await;

        assert_eq!(registry.count(), 0, "empty registry should remain empty");
    }

    // ─── E04: close_room 广播关闭消息给所有成员 ──────────────────────────────

    /// E04: close_room 事件 → 广播关闭消息给房间内所有连接
    #[tokio::test]
    async fn e04_close_room_broadcasts_to_room_members() {
        let registry = Arc::new(ConnectionRegistry::new());
        let room_id = Uuid::new_v4();

        // 注册 2 个房间内的用户 + 1 个不在房间的用户
        let (h1, mut rx1) = make_handle(Uuid::new_v4(), Some(room_id));
        let (h2, mut rx2) = make_handle(Uuid::new_v4(), Some(room_id));
        let (h_other, mut rx_other) = make_handle(Uuid::new_v4(), None); // 不在房间
        registry.register(h1);
        registry.register(h2);
        registry.register(h_other);

        let event = close_room_event(room_id);
        super::handle_admin_event(event, registry.clone()).await;

        // 房间内两个连接都必须收到关闭消息
        let msg1 = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
            .await
            .expect("rx1 should not time out")
            .expect("rx1 channel should not be closed");
        let msg2 = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
            .await
            .expect("rx2 should not time out")
            .expect("rx2 channel should not be closed");

        let json1: serde_json::Value =
            serde_json::from_str(&msg1).expect("msg1 should be valid JSON");
        let json2: serde_json::Value =
            serde_json::from_str(&msg2).expect("msg2 should be valid JSON");

        assert_eq!(
            json1["type"], "close_room",
            "room member 1 should receive close_room"
        );
        assert_eq!(
            json2["type"], "close_room",
            "room member 2 should receive close_room"
        );

        // 不在房间的连接不应收到消息
        let no_msg = tokio::time::timeout(Duration::from_millis(20), rx_other.recv()).await;
        assert!(
            no_msg.is_err(),
            "connection not in room should NOT receive close_room message"
        );
    }

    // ─── E05: close_room 断开所有房间成员连接 ────────────────────────────────

    /// E05: close_room 事件 → 广播后断开所有房间成员连接
    #[tokio::test]
    async fn e05_close_room_disconnects_all_members() {
        let registry = Arc::new(ConnectionRegistry::new());
        let room_id = Uuid::new_v4();

        let (h1, _rx1) = make_handle(Uuid::new_v4(), Some(room_id));
        let (h2, _rx2) = make_handle(Uuid::new_v4(), Some(room_id));
        let conn1 = h1.connection_id;
        let conn2 = h2.connection_id;
        registry.register(h1);
        registry.register(h2);

        assert_eq!(registry.count(), 2);

        let event = close_room_event(room_id);
        super::handle_admin_event(event, registry.clone()).await;

        assert_eq!(
            registry.count(),
            0,
            "all room members should be unregistered after close_room"
        );
        assert!(
            registry.get(conn1).is_none(),
            "conn1 should be removed from registry"
        );
        assert!(
            registry.get(conn2).is_none(),
            "conn2 should be removed from registry"
        );
    }

    // ─── E06: broadcast_notice 向所有在线用户推送 ────────────────────────────

    /// E06: broadcast_notice 事件 → 向所有在线用户推送
    #[tokio::test]
    async fn e06_broadcast_notice_sends_to_all_connections() {
        let registry = Arc::new(ConnectionRegistry::new());

        let (h1, mut rx1) = make_handle(Uuid::new_v4(), None);
        let (h2, mut rx2) = make_handle(Uuid::new_v4(), Some(Uuid::new_v4()));
        registry.register(h1);
        registry.register(h2);

        let notice_msg = "Happy New Year from admin!";
        let event = broadcast_notice_event(notice_msg);
        super::handle_admin_event(event, registry.clone()).await;

        // 所有连接都应收到公告
        let msg1 = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
            .await
            .expect("rx1 should not time out")
            .expect("rx1 channel closed");
        let msg2 = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
            .await
            .expect("rx2 should not time out")
            .expect("rx2 channel closed");

        let json1: serde_json::Value = serde_json::from_str(&msg1).unwrap();
        let json2: serde_json::Value = serde_json::from_str(&msg2).unwrap();

        assert_eq!(json1["type"], "broadcast_notice");
        assert_eq!(json1["message"], notice_msg);
        assert_eq!(json2["type"], "broadcast_notice");
        assert_eq!(json2["message"], notice_msg);
    }

    // ─── E07: 事件处理失败不影响主服务 ──────────────────────────────────────

    /// E07: 事件处理失败不影响主服务（channel 已关闭时不 panic）
    ///
    /// 模拟场景：receiver 端已被 drop（连接已断开），但 sender 仍在 registry 中。
    /// handle_admin_event 必须能静默跳过，不 panic。
    #[tokio::test]
    async fn e07_event_handling_failure_does_not_crash() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();

        let (handle, rx) = make_handle(user_id, None);
        registry.register(handle);

        // 模拟 receiver 端被 drop（连接已断开）
        drop(rx);

        // ban_user：发送到已关闭 channel 时必须静默处理，不 panic
        let event = ban_user_event(user_id);
        // 若此处 panic，测试框架会捕获并报告失败
        super::handle_admin_event(event, registry.clone()).await;

        // 运行到这里说明没有 panic —— 验证注销也已执行
        assert_eq!(
            registry.count(),
            0,
            "connection should still be unregistered even when channel was already closed"
        );
    }
}
