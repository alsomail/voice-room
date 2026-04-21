//! BalanceBroadcaster — 监听 BalanceEvent 并广播 BalanceUpdated WS 信令
//!
//! ## 设计
//! - `BalanceEvent` 由 `WalletService::notify_balance_updated` 在事务提交后发送
//! - `BalanceBroadcaster::run(rx)` 作为常驻 tokio task 运行，消费本进程 mpsc channel
//! - `handle_redis_payload(json)` 解析 Redis admin:events 中的 balance_updated 事件并广播
//! - `run_with_redis(rx, redis_url, shutdown)` 同时监听本进程 channel 和 Redis PubSub 两个事件源
//! - 同一用户多端在线时，遍历 `registry.get_by_user_id` 全部推送

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::ws::ConnectionRegistry;

// ─── BalanceEvent ─────────────────────────────────────────────────────────────

/// 余额变化事件，由 `WalletService::notify_balance_updated` 在事务提交后发送。
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

// ─── Redis balance_updated 事件反序列化结构 ───────────────────────────────────

/// Redis admin:events 频道中 balance_updated 事件的 payload
#[derive(Debug, Deserialize)]
struct BalanceUpdatedRedisPayload {
    user_id: Uuid,
    balance_after: i64,
    delta: i64,
    reason: String,
    ref_id: Option<Uuid>,
}

// ─── BalanceBroadcaster ───────────────────────────────────────────────────────

/// 余额广播器
///
/// ## 两个事件源
/// 1. 本进程 `mpsc::Receiver<BalanceEvent>`（同进程余额变更，如 SendGift）
/// 2. Redis PubSub `admin:events` 中 `type=balance_updated`（Admin 跨进程调整）
///
/// 收到事件后通过 `ConnectionRegistry` 向该用户所有在线 WS 连接推送 `BalanceUpdated` 信令。
#[derive(Clone)]
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
    /// 格式（对齐 TDS T-00018 §新增 WS 信令 + WS 通用格式 §6.3）：
    /// ```json
    /// {
    ///   "type": "BalanceUpdated",
    ///   "msg_id": "uuid",
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
        let connections = self.registry.get_by_user_id(event.user_id);
        for (_, sender) in connections {
            // MEDIUM-2: 每条 WS 消息独立生成 msg_id，符合 §6.3 通用格式要求
            let msg = serde_json::json!({
                "type": "BalanceUpdated",
                "msg_id": Uuid::new_v4().to_string(),
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
                    continue;
                }
            };

            // MEDIUM-1: send 失败时记录 warn 日志（unbounded channel 实际上不会满）
            if let Err(e) = sender.send(msg_str) {
                tracing::warn!(
                    user_id = %event.user_id,
                    "Failed to send BalanceUpdated to WS connection, connection may have closed: {}",
                    e
                );
            }
        }
    }

    /// 解析 Redis admin:events 频道中的 balance_updated 事件并广播 WS 信令。
    ///
    /// 此方法是 HIGH-2 的核心：将 Redis PubSub 消息转换为 WS 推送。
    ///
    /// # 处理逻辑
    /// - 非 `balance_updated` 类型的事件静默忽略（其他 AdminEvent 由 events/subscriber.rs 处理）
    /// - JSON 解析失败记录 error 日志，不 panic
    /// - payload 字段缺失记录 error 日志，不 panic
    pub fn handle_redis_payload(&self, json: &str) {
        let val = match serde_json::from_str::<serde_json::Value>(json) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    raw = %json,
                    "BalanceBroadcaster: failed to parse Redis event JSON: {}",
                    e
                );
                return;
            }
        };

        // 只处理 balance_updated 类型
        match val.get("type").and_then(|v| v.as_str()) {
            Some("balance_updated") => {}
            _ => return, // 其他类型静默忽略
        }

        let payload_val = match val.get("payload") {
            Some(p) => p,
            None => {
                tracing::error!(
                    raw = %json,
                    "BalanceBroadcaster: balance_updated event missing 'payload' field"
                );
                return;
            }
        };

        let payload = match serde_json::from_value::<BalanceUpdatedRedisPayload>(payload_val.clone()) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(
                    raw = %json,
                    "BalanceBroadcaster: failed to parse balance_updated payload: {}",
                    e
                );
                return;
            }
        };

        self.broadcast_event(&BalanceEvent {
            user_id: payload.user_id,
            balance_after: payload.balance_after,
            delta: payload.delta,
            reason: payload.reason,
            ref_id: payload.ref_id,
        });
    }

    /// 常驻循环：持续从 rx 接收 BalanceEvent 并广播（仅本进程 mpsc channel）。
    ///
    /// 通常通过 `tokio::spawn(broadcaster.run(rx))` 启动。
    /// 当 sender 端（WalletService）被 drop 时，rx 返回 None，task 自然退出。
    pub async fn run(self, mut rx: mpsc::Receiver<BalanceEvent>) {
        while let Some(event) = rx.recv().await {
            self.broadcast_event(&event);
        }
        tracing::info!("BalanceBroadcaster: channel closed, task exiting");
    }

    /// 常驻循环：同时监听本进程 mpsc channel 和 Redis PubSub 两个事件源。
    ///
    /// # 参数
    /// - `rx`：本进程余额事件 channel（来自 WalletService::notify_balance_updated）
    /// - `redis_url`：Redis 连接字符串
    /// - `shutdown`：优雅停机信号（watch::Receiver<bool>，收到变更即退出）
    ///
    /// # 行为
    /// - Redis 连接失败：等待 2s 后重试
    /// - Redis 断线重连：自动重试
    /// - 收到停机信号：立即退出
    pub async fn run_with_redis(
        self,
        mut rx: mpsc::Receiver<BalanceEvent>,
        redis_url: String,
        mut shutdown: watch::Receiver<bool>,
    ) {
        let client = match redis::Client::open(redis_url.as_str()) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("BalanceBroadcaster: invalid Redis URL '{}': {:?}", redis_url, e);
                // fallback: 仅运行 mpsc 模式
                while let Some(event) = rx.recv().await {
                    self.broadcast_event(&event);
                }
                return;
            }
        };

        loop {
            match client.get_async_pubsub().await {
                Ok(mut pubsub) => {
                    if let Err(e) = pubsub.subscribe("admin:events").await {
                        tracing::error!(
                            "BalanceBroadcaster: failed to subscribe to admin:events: {:?}",
                            e
                        );
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }

                    tracing::info!("BalanceBroadcaster: subscribed to Redis admin:events channel");
                    let mut stream = pubsub.on_message();

                    loop {
                        tokio::select! {
                            msg = stream.next() => {
                                match msg {
                                    Some(msg) => {
                                        let payload: String = msg.get_payload().unwrap_or_default();
                                        self.handle_redis_payload(&payload);
                                    }
                                    None => {
                                        tracing::warn!(
                                            "BalanceBroadcaster: Redis pubsub stream ended, reconnecting"
                                        );
                                        break;
                                    }
                                }
                            }
                            event = rx.recv() => {
                                match event {
                                    Some(event) => self.broadcast_event(&event),
                                    None => {
                                        tracing::info!(
                                            "BalanceBroadcaster: mpsc channel closed, continuing Redis-only mode"
                                        );
                                        // mpsc 关闭后继续监听 Redis
                                    }
                                }
                            }
                            _ = shutdown.changed() => {
                                tracing::info!("BalanceBroadcaster: shutdown signal received, exiting");
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "BalanceBroadcaster: Redis connection failed: {:?}, retrying in 2s",
                        e
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
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

    // BR01: broadcast_event 向目标用户发送 BalanceUpdated，含 msg_id
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
        // MEDIUM-2: 必须包含 msg_id 字段且为合法 UUID
        let msg_id = val["msg_id"].as_str().expect("msg_id must be present");
        Uuid::parse_str(msg_id).expect("msg_id must be a valid UUID");
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
        // 两条消息的 msg_id 应各自不同（每次广播独立生成）
        let id1 = v1["msg_id"].as_str().unwrap();
        let id2 = v2["msg_id"].as_str().unwrap();
        assert_ne!(id1, id2, "Each connection should get a unique msg_id");
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

    // BR06: handle_redis_payload 解析合法 balance_updated JSON 并推送 WS
    // 这是 HIGH-2 的核心测试：验证 Redis → WS 完整路径（无需真实 Redis）
    #[tokio::test]
    async fn br06_handle_redis_payload_balance_updated() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();
        let ref_id = Uuid::new_v4();
        let (handle, mut rx) = make_handle(user_id);
        registry.register(handle);

        let broadcaster = BalanceBroadcaster::new(registry);

        // 构造 Redis admin:events 频道中 balance_updated 事件的 JSON
        let json = serde_json::json!({
            "type": "balance_updated",
            "payload": {
                "user_id": user_id.to_string(),
                "balance_after": 4800_i64,
                "delta": -520_i64,
                "reason": "admin_adjust",
                "ref_id": ref_id.to_string(),
            }
        })
        .to_string();

        broadcaster.handle_redis_payload(&json);

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Should receive WS message after Redis balance_updated event")
            .expect("WS channel should not be closed");

        let val: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(val["type"], "BalanceUpdated");
        assert_eq!(val["payload"]["diamond_balance"], 4800);
        assert_eq!(val["payload"]["delta"], -520);
        assert_eq!(val["payload"]["reason"], "admin_adjust");
        assert_eq!(
            val["payload"]["ref_id"].as_str().unwrap(),
            ref_id.to_string()
        );
        // msg_id 必须存在
        let msg_id = val["msg_id"].as_str().expect("msg_id must be present in BalanceUpdated from Redis");
        Uuid::parse_str(msg_id).expect("msg_id must be valid UUID");
    }

    // BR07: handle_redis_payload 忽略非 balance_updated 类型（不 panic，不推送）
    #[tokio::test]
    async fn br07_handle_redis_payload_ignores_other_event_types() {
        let registry = Arc::new(ConnectionRegistry::new());
        let user_id = Uuid::new_v4();
        let (handle, mut rx) = make_handle(user_id);
        registry.register(handle);

        let broadcaster = BalanceBroadcaster::new(registry);

        // 发送一个 ban_user 事件，不应触发 WS 推送
        let json = serde_json::json!({
            "type": "ban_user",
            "payload": { "user_id": user_id.to_string() },
            "admin_id": Uuid::new_v4().to_string(),
            "ts": 1700000000_i64
        })
        .to_string();

        broadcaster.handle_redis_payload(&json);

        // 不应收到任何 WS 消息
        let no_msg = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(no_msg.is_err(), "Non-balance_updated event must not trigger WS push");
    }

    // BR08: handle_redis_payload 处理格式错误的 JSON（不 panic）
    #[tokio::test]
    async fn br08_handle_redis_payload_invalid_json_no_panic() {
        let registry = Arc::new(ConnectionRegistry::new());
        let broadcaster = BalanceBroadcaster::new(registry);

        // 格式错误的 JSON 不应 panic
        broadcaster.handle_redis_payload("not valid json at all");
        broadcaster.handle_redis_payload(r#"{"type":"balance_updated","payload":{}}"#);
        // 缺少必填字段（user_id 缺失）
        broadcaster.handle_redis_payload(r#"{"type":"balance_updated","payload":{"balance_after":100}}"#);
    }
}
