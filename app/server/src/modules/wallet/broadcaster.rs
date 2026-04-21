//! BalanceBroadcaster — 监听 BalanceEvent 并广播 BalanceUpdated WS 信令
//!
//! ## 设计
//! - `BalanceEvent` 由 `WalletService::apply_delta` 在事务提交后发送
//! - `BalanceBroadcaster::run(rx)` 作为常驻 tokio task 运行
//! - `broadcast_event(&event)` 纯同步方法，可直接在单元测试中调用
//! - 同一用户多端在线时，遍历 `registry.get_by_user_id` 全部推送

use std::sync::Arc;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::ws::ConnectionRegistry;

// ─── BalanceEvent ─────────────────────────────────────────────────────────────

/// 余额变化事件，由 `WalletService::apply_delta` 在事务提交后发送。
#[derive(Debug, Clone)]
pub struct BalanceEvent {
    /// 余额变化的用户
    pub user_id: Uuid,
    /// 变化后余额
    pub balance_after: i64,
    /// 本次变化量（正数=充值，负数=扣款）
    pub delta: i64,
    /// 变化原因（对应 WalletTxnType 的 snake_case 字符串，或自定义描述）
    pub reason: String,
    /// 关联业务 ID（礼物记录 ID、admin_log_id 等，可选）
    pub ref_id: Option<Uuid>,
}

// ─── BalanceBroadcaster ───────────────────────────────────────────────────────

/// 余额广播器
///
/// 从 `mpsc::Receiver<BalanceEvent>` 接收事件，并通过 `ConnectionRegistry`
/// 向该用户所有在线 WS 连接推送 `BalanceUpdated` 信令。
pub struct BalanceBroadcaster {
    registry: Arc<ConnectionRegistry>,
}

impl BalanceBroadcaster {
    /// 创建广播器，注入 ConnectionRegistry 共享引用
    pub fn new(registry: Arc<ConnectionRegistry>) -> Self {
        Self { registry }
    }

    /// 向目标用户所有在线 WS 连接推送 `BalanceUpdated` 信令。
    ///
    /// 格式（对齐 TDS T-00018 §新增 WS 信令）：
    /// ```json
    /// {
    ///   "type": "BalanceUpdated",
    ///   "payload": {
    ///     "diamond_balance": 4800,
    ///     "delta": -520,
    ///     "reason": "gift_send",
    ///     "ref_id": "uuid|null"
    ///   },
    ///   "timestamp": 1720000000000
    /// }
    /// ```
    pub fn broadcast_event(&self, event: &BalanceEvent) {
        let msg = serde_json::json!({
            "type": "BalanceUpdated",
            "payload": {
                "diamond_balance": event.balance_after,
                "delta": event.delta,
                "reason": event.reason,
                "ref_id": event.ref_id.map(|u| u.to_string()),
            },
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });

        let msg_str = match serde_json::to_string(&msg) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("failed to serialize BalanceUpdated: {}", e);
                return;
            }
        };

        let connections = self.registry.get_by_user_id(event.user_id);
        for (_, sender) in connections {
            let _ = sender.send(msg_str.clone());
        }
    }

    /// 常驻循环：持续从 rx 接收 BalanceEvent 并广播。
    ///
    /// 通常通过 `tokio::spawn(broadcaster.run(rx))` 启动。
    /// 当 sender 端（WalletService）被 drop 时，rx 返回 None，task 自然退出。
    pub async fn run(self, mut rx: mpsc::Receiver<BalanceEvent>) {
        while let Some(event) = rx.recv().await {
            self.broadcast_event(&event);
        }
        tracing::info!("BalanceBroadcaster: channel closed, task exiting");
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, RwLock};
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc as tokio_mpsc;
    use uuid::Uuid;

    use crate::ws::registry::{ConnectionHandle, ConnectionRegistry};

    fn make_handle(user_id: Uuid) -> (ConnectionHandle, tokio_mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = tokio_mpsc::unbounded_channel::<String>();
        let handle = ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id,
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        };
        (handle, rx)
    }

    // BR01: broadcast_event 向目标用户发送 BalanceUpdated
    #[tokio::test]
    async fn br01_broadcast_event_sends_balance_updated() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();
        let (handle, mut rx) = make_handle(user_id);
        registry.register(handle);

        let broadcaster = BalanceBroadcaster::new(registry);
        broadcaster.broadcast_event(&BalanceEvent {
            user_id,
            balance_after: 1000,
            delta: 500,
            reason: "recharge".to_string(),
            ref_id: None,
        });

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let val: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(val["type"], "BalanceUpdated");
        assert_eq!(val["payload"]["diamond_balance"], 1000);
        assert_eq!(val["payload"]["delta"], 500);
        assert_eq!(val["payload"]["reason"], "recharge");
    }

    // BR02: broadcast_event 多连接同一用户全部收到
    #[tokio::test]
    async fn br02_broadcast_event_sends_to_all_user_connections() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();

        let (h1, mut rx1) = make_handle(user_id);
        let (h2, mut rx2) = make_handle(user_id);
        registry.register(h1);
        registry.register(h2);

        let broadcaster = BalanceBroadcaster::new(registry);
        broadcaster.broadcast_event(&BalanceEvent {
            user_id,
            balance_after: 200,
            delta: 200,
            reason: "admin_adjust".to_string(),
            ref_id: None,
        });

        let msg1 = tokio::time::timeout(Duration::from_millis(100), rx1.recv())
            .await
            .unwrap()
            .unwrap();
        let msg2 = tokio::time::timeout(Duration::from_millis(100), rx2.recv())
            .await
            .unwrap()
            .unwrap();

        let v1: serde_json::Value = serde_json::from_str(&msg1).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&msg2).unwrap();
        assert_eq!(v1["type"], "BalanceUpdated");
        assert_eq!(v2["type"], "BalanceUpdated");
    }

    // BR03: broadcast_event 用户不在线时不 panic
    #[tokio::test]
    async fn br03_broadcast_event_offline_user_no_panic() {
        let registry = Arc::new(ConnectionRegistry::new());
        let broadcaster = BalanceBroadcaster::new(registry);

        // 不 panic
        broadcaster.broadcast_event(&BalanceEvent {
            user_id: Uuid::new_v4(),
            balance_after: 0,
            delta: 0,
            reason: "gift_send".to_string(),
            ref_id: None,
        });
    }

    // BR04: run() 通过 mpsc channel 接收并广播
    #[tokio::test]
    async fn br04_run_processes_events_from_channel() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();
        let (handle, mut ws_rx) = make_handle(user_id);
        registry.register(handle);

        let (tx, rx) = mpsc::channel::<BalanceEvent>(10);
        let broadcaster = BalanceBroadcaster::new(registry);
        tokio::spawn(broadcaster.run(rx));

        tx.send(BalanceEvent {
            user_id,
            balance_after: 300,
            delta: 300,
            reason: "recharge".to_string(),
            ref_id: None,
        })
        .await
        .unwrap();

        let msg = tokio::time::timeout(Duration::from_millis(200), ws_rx.recv())
            .await
            .unwrap()
            .unwrap();
        let val: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(val["type"], "BalanceUpdated");
        assert_eq!(val["payload"]["diamond_balance"], 300);
    }

    // BR05: ref_id 字段正确序列化为字符串
    #[tokio::test]
    async fn br05_ref_id_serialized_as_string() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();
        let ref_id = Uuid::new_v4();
        let (handle, mut rx) = make_handle(user_id);
        registry.register(handle);

        let broadcaster = BalanceBroadcaster::new(registry);
        broadcaster.broadcast_event(&BalanceEvent {
            user_id,
            balance_after: 100,
            delta: 100,
            reason: "recharge".to_string(),
            ref_id: Some(ref_id),
        });

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let val: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(
            val["payload"]["ref_id"].as_str().unwrap(),
            ref_id.to_string()
        );
    }
}
