//! 跨服务契约测试 — Admin Server `balance_updated` ↔ App Server `BalanceBroadcaster`
//!
//! 缺陷 #1 P0 防回归：Admin 端 publish 的 `payload` 必须能被 App 端 `handle_redis_payload`
//! 直接消费并广播 WS。本测试通过共享结构体 `BalanceUpdatedEvent`
//! 完整模拟 "Admin 序列化 → Redis JSON → App 反序列化 → WS 推送" 全链路，
//! 不依赖真实 Redis（PubSub 流被替换为直接调用 `handle_redis_payload`）。

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use uuid::Uuid;
use voice_room_shared::events::BalanceUpdatedEvent;

use voice_room_server::modules::wallet::broadcaster::BalanceBroadcaster;
use voice_room_server::ws::registry::{ConnectionHandle, ConnectionRegistry};

fn make_handle(user_id: Uuid) -> (ConnectionHandle, mpsc::UnboundedReceiver<String>) {
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    (
        ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id,
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        },
        rx,
    )
}

/// 模拟 Admin Server `WalletService::adjust_balance` 构造的事件 envelope。
///
/// 字段顺序与 `app/adminServer/src/modules/wallet/service.rs` 保持一致。
fn admin_publish_envelope(payload: &BalanceUpdatedEvent, admin_id: Uuid, ts: i64) -> String {
    serde_json::json!({
        "type":     "balance_updated",
        "payload":  serde_json::to_value(payload).unwrap(),
        "admin_id": admin_id.to_string(),
        "ts":       ts,
    })
    .to_string()
}

// CSCT-01: Admin 序列化的 payload → App `handle_redis_payload` → WS 收到 BalanceUpdated
#[tokio::test]
async fn csct01_admin_publish_to_app_ws_full_loop() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user_id = Uuid::new_v4();
    let ref_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let (handle, mut rx) = make_handle(user_id);
    registry.register(handle);

    let broadcaster = BalanceBroadcaster::new(registry);

    // Admin 端构造（与 service.rs 同源）
    let payload = BalanceUpdatedEvent {
        user_id,
        balance_after: 4800,
        delta: -520,
        reason: "admin_adjust".to_string(),
        ref_id: Some(ref_id),
    };
    let envelope = admin_publish_envelope(&payload, admin_id, 1_720_000_000);

    // App 端解析
    broadcaster.handle_redis_payload(&envelope);

    let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("CSCT-01: 必须收到 WS 消息（契约对齐）")
        .expect("CSCT-01: WS channel 未关闭");
    let val: serde_json::Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(val["type"], "BalanceUpdated");
    assert_eq!(val["payload"]["diamond_balance"], 4800);
    assert_eq!(val["payload"]["delta"], -520);
    assert_eq!(val["payload"]["reason"], "admin_adjust");
    assert_eq!(
        val["payload"]["ref_id"].as_str().unwrap(),
        ref_id.to_string()
    );
}

// CSCT-02: Admin 端 ref_id=None 时，App 端仍能解析（防 #[serde(default)] 缺失回归）
#[tokio::test]
async fn csct02_admin_payload_without_ref_id_still_parses() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user_id = Uuid::new_v4();
    let (handle, mut rx) = make_handle(user_id);
    registry.register(handle);

    let broadcaster = BalanceBroadcaster::new(registry);

    let payload = BalanceUpdatedEvent {
        user_id,
        balance_after: 1000,
        delta: 500,
        reason: "recharge".to_string(),
        ref_id: None,
    };
    let envelope = admin_publish_envelope(&payload, Uuid::new_v4(), 1_720_000_000);

    broadcaster.handle_redis_payload(&envelope);

    let msg = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("CSCT-02: 必须收到 WS 消息")
        .unwrap();
    let val: serde_json::Value = serde_json::from_str(&msg).unwrap();
    assert_eq!(val["payload"]["diamond_balance"], 1000);
    // ref_id null
    assert!(val["payload"]["ref_id"].is_null());
}

// CSCT-03: 老契约（`new_balance` 字段）必须解析失败、不推 WS（fail-fast，避免静默错配）
#[tokio::test]
async fn csct03_legacy_new_balance_payload_is_rejected() {
    let registry = Arc::new(ConnectionRegistry::new());
    let user_id = Uuid::new_v4();
    let (handle, mut rx) = make_handle(user_id);
    registry.register(handle);

    let broadcaster = BalanceBroadcaster::new(registry);

    // 模拟修复前 Admin 发出的旧 payload（含 new_balance 而非 balance_after）
    let legacy = serde_json::json!({
        "type": "balance_updated",
        "payload": {
            "user_id":     user_id.to_string(),
            "new_balance": 1500_i64,
            "delta":       500_i64,
            "reason":      "admin_adjust",
        }
    })
    .to_string();

    broadcaster.handle_redis_payload(&legacy);

    // App 端解析失败 → 不应有 WS 推送
    let no_msg = tokio::time::timeout(Duration::from_millis(80), rx.recv()).await;
    assert!(
        no_msg.is_err(),
        "CSCT-03: 旧契约 payload 必须解析失败（fail-fast），不可静默推送错误数据"
    );
}
