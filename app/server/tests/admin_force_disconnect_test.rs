//! T-00042 集成测试：Admin 强制断连广播事件
//!
//! 测试覆盖：
//! - U-1: user_banned → UserBanned 通知 + close 4003
//! - U-2: room_closed → RoomClosed 广播 + close 1000
//! - U-3: 多连接场景全部断开

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use uuid::Uuid;

use voice_room_server::events::admin_event::{AdminEvent, BanUserPayload, CloseRoomPayload};
use voice_room_server::events::handler::handle_admin_event;
use voice_room_server::ws::registry::{ConnectionHandle, ConnectionRegistry};
use std::sync::RwLock;
use std::time::Instant;

// ─── 测试辅助函数 ─────────────────────────────────────────────────────────────

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

// ─── U-1: user_banned → UserBanned 通知 + close 4003 ────────────────────────

/// U-1: 封禁用户事件 → 发送 UserBanned 通知 + 发送 Close 指令（code=4003）
#[tokio::test]
async fn u01_ban_user_sends_notification_and_close_frame() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user_id = Uuid::new_v4();

    let (handle, mut rx) = make_handle(user_id, None);
    registry.register(handle);

    let event = AdminEvent::BanUser {
        payload: BanUserPayload { user_id },
        admin_id: Uuid::new_v4(),
        ts: 1700000000,
    };

    handle_admin_event(event, registry.clone()).await;

    // 1. 第一条消息：UserBanned 通知
    let msg1 = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("should receive UserBanned notification")
        .expect("channel should not be closed");

    let json1: serde_json::Value = serde_json::from_str(&msg1).expect("valid JSON");
    assert_eq!(
        json1["type"], "ban_user",
        "U-1: 第一条消息应为 UserBanned 通知"
    );

    // 2. 第二条消息：Close 指令（用于触发 WebSocket Close frame）
    let msg2 = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("should receive close instruction")
        .expect("channel should not be closed");

    let json2: serde_json::Value = serde_json::from_str(&msg2).expect("valid JSON");
    assert_eq!(
        json2["type"], "connection_close",
        "U-1: 第二条消息应为 connection_close 指令"
    );
    assert_eq!(
        json2["code"], 4003,
        "U-1: close code 应为 4003（Account banned）"
    );

    // 3. 连接应被注销
    assert_eq!(
        registry.count(),
        0,
        "U-1: 连接应在发送 close 指令后被注销"
    );
}

// ─── U-2: room_closed → RoomClosed 广播 + close 1000 ────────────────────────

/// U-2: 房间关闭事件 → 广播 RoomClosed + 发送 Close 指令（code=1000）
#[tokio::test]
async fn u02_close_room_broadcasts_and_sends_close_frame() {
    let registry = Arc::new(ConnectionRegistry::new());
    let room_id = Uuid::new_v4();

    let (h1, mut rx1) = make_handle(Uuid::new_v4(), Some(room_id));
    let (h2, mut rx2) = make_handle(Uuid::new_v4(), Some(room_id));
    registry.register(h1);
    registry.register(h2);

    let event = AdminEvent::CloseRoom {
        payload: CloseRoomPayload { room_id },
        admin_id: Uuid::new_v4(),
        ts: 1700000001,
    };

    handle_admin_event(event, registry.clone()).await;

    // 房间成员 1：应收到 RoomClosed 通知 + close 指令
    let msg1_notif = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
        .await
        .expect("rx1 should receive RoomClosed")
        .expect("channel not closed");
    let json1: serde_json::Value = serde_json::from_str(&msg1_notif).unwrap();
    assert_eq!(json1["type"], "close_room", "U-2: 应收到 RoomClosed 通知");

    let msg1_close = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
        .await
        .expect("rx1 should receive close instruction")
        .expect("channel not closed");
    let close1: serde_json::Value = serde_json::from_str(&msg1_close).unwrap();
    assert_eq!(close1["type"], "connection_close", "U-2: 应收到 close 指令");
    assert_eq!(close1["code"], 1000, "U-2: close code 应为 1000");

    // 房间成员 2：同样应收到两条消息
    let msg2_notif = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
        .await
        .expect("rx2 should receive RoomClosed")
        .expect("channel not closed");
    let json2: serde_json::Value = serde_json::from_str(&msg2_notif).unwrap();
    assert_eq!(json2["type"], "close_room");

    let msg2_close = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
        .await
        .expect("rx2 should receive close instruction")
        .expect("channel not closed");
    let close2: serde_json::Value = serde_json::from_str(&msg2_close).unwrap();
    assert_eq!(close2["type"], "connection_close");
    assert_eq!(close2["code"], 1000);

    // 连接应全部注销
    assert_eq!(registry.count(), 0, "U-2: 所有房间成员连接应被注销");
}

// ─── U-3: 多连接场景全部断开 ─────────────────────────────────────────────────

/// U-3: 用户有 3 个连接（多设备），封禁后全部断开
#[tokio::test]
async fn u03_ban_user_multiple_connections_all_disconnected() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user_id = Uuid::new_v4();

    // 同一用户，3 个连接
    let (h1, mut rx1) = make_handle(user_id, None);
    let (h2, mut rx2) = make_handle(user_id, Some(Uuid::new_v4()));
    let (h3, mut rx3) = make_handle(user_id, None);
    registry.register(h1);
    registry.register(h2);
    registry.register(h3);

    assert_eq!(registry.count(), 3, "U-3: 注册 3 个连接");

    let event = AdminEvent::BanUser {
        payload: BanUserPayload { user_id },
        admin_id: Uuid::new_v4(),
        ts: 1700000000,
    };

    handle_admin_event(event, registry.clone()).await;

    // 所有连接都应收到通知 + close 指令
    for rx in [&mut rx1, &mut rx2, &mut rx3] {
        let notif = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should receive notification")
            .expect("channel not closed");
        let json: serde_json::Value = serde_json::from_str(&notif).unwrap();
        assert_eq!(json["type"], "ban_user", "U-3: 每个连接都应收到 ban 通知");

        let close_msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should receive close instruction")
            .expect("channel not closed");
        let close_json: serde_json::Value = serde_json::from_str(&close_msg).unwrap();
        assert_eq!(
            close_json["type"], "connection_close",
            "U-3: 每个连接都应收到 close 指令"
        );
        assert_eq!(close_json["code"], 4003);
    }

    assert_eq!(registry.count(), 0, "U-3: 所有 3 个连接都应被注销");
}
