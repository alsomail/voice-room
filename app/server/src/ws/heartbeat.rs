//! 心跳检测模块
//!
//! 每 5 秒扫描所有连接的 last_heartbeat，超过 30 秒无心跳的连接将被服务端主动剔除
//! 并通过显式 close-frame（code=1000, reason="Heartbeat timeout"）关闭 WebSocket。
//! `remove_expired*` 暴露为纯函数以便单元测试直接调用，不依赖定时器。

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ws::CloseFrame;

use super::registry::ConnectionRegistry;

/// 心跳超时阈值默认值（30 秒无心跳则断开）。
/// 与 `doc/protocol/websocket_signals.md §6.2` 一致。
pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(30);
/// 心跳检测扫描间隔默认值（每 5 秒一次）。T-00041：从 10s 缩短到 5s
/// 以保证 35s 静默场景内能够及时触发剔除（最坏 30+5=35s）。
pub const CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// 心跳超时时给客户端发送的显式关闭信令（P2-17：将"超时"语义提升为可观测事件）。
pub const HEARTBEAT_TIMEOUT_MESSAGE: &str =
    r#"{"type":"connection_closed","reason":"heartbeat_timeout"}"#;

/// T-00041：心跳超时主动关闭时使用的 WebSocket Close 帧 reason 文本。
pub const HEARTBEAT_TIMEOUT_CLOSE_REASON: &str = "Heartbeat timeout";
/// T-00041：心跳超时主动关闭时使用的 WebSocket Close 帧状态码（RFC 6455 §7.4 — 1000 Normal Closure）。
pub const HEARTBEAT_TIMEOUT_CLOSE_CODE: u16 = 1000;

/// 心跳检测可注入配置（默认值与 `HEARTBEAT_TIMEOUT` / `CHECK_INTERVAL` 保持一致，向后兼容）。
#[derive(Clone, Copy, Debug)]
pub struct HeartbeatConfig {
    pub timeout: Duration,
    pub check_interval: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            timeout: HEARTBEAT_TIMEOUT,
            check_interval: CHECK_INTERVAL,
        }
    }
}

/// T-00041：将 mpsc 下行文本帧映射为可发的 WebSocket Close 帧。
///
/// 当且仅当文本为 `HEARTBEAT_TIMEOUT_MESSAGE` 时返回
/// `Some(CloseFrame{1000, "Heartbeat timeout"})`，用于 connection 主循环在转发完
/// 该文本后再发送显式 Close 帧并终止连接。其他文本返回 `None`（保持原行为）。
pub fn close_frame_for_message(text: &str) -> Option<CloseFrame> {
    if text == HEARTBEAT_TIMEOUT_MESSAGE {
        Some(CloseFrame {
            code: HEARTBEAT_TIMEOUT_CLOSE_CODE,
            reason: HEARTBEAT_TIMEOUT_CLOSE_REASON.into(),
        })
    } else {
        None
    }
}

/// 移除所有心跳已超时的连接（使用默认 30s 阈值，保持向后兼容）。
pub fn remove_expired(registry: &ConnectionRegistry) -> usize {
    remove_expired_with_timeout(registry, HEARTBEAT_TIMEOUT)
}

/// 移除所有 `now - last_heartbeat > timeout` 的连接，返回剔除条目数。
///
/// 使用 DashMap::retain 原子地扫描+删除。
/// P2-17：移除前显式向客户端 sender 发送 `HEARTBEAT_TIMEOUT_MESSAGE`。
/// T-00041：tracing::warn 输出 user_id，便于线上问题定位。
pub fn remove_expired_with_timeout(registry: &ConnectionRegistry, timeout: Duration) -> usize {
    let now = Instant::now();
    let mut removed: usize = 0;

    registry.connections.retain(|connection_id, handle| {
        let elapsed = now.duration_since(
            *handle
                .last_heartbeat
                .read()
                .expect("heartbeat lock poisoned"),
        );
        if elapsed > timeout {
            tracing::warn!(
                %connection_id,
                user_id = %handle.user_id,
                elapsed_secs = elapsed.as_secs(),
                timeout_secs = timeout.as_secs(),
                "heartbeat timeout, server actively closing connection"
            );
            let _ = handle.sender.send(HEARTBEAT_TIMEOUT_MESSAGE.to_string());
            removed += 1;
            false
        } else {
            true
        }
    });

    removed
}

/// 心跳检测后台 task：每 `CHECK_INTERVAL` 触发一次清理，支持优雅停机。
pub async fn heartbeat_task(
    registry: Arc<ConnectionRegistry>,
    shutdown: tokio::sync::watch::Receiver<bool>,
) {
    heartbeat_task_with_config(registry, shutdown, HeartbeatConfig::default()).await
}

/// 可注入配置版本的心跳后台 task（T-00041）。默认等价于历史行为；测试可注入更短超时或自定义间隔。
pub async fn heartbeat_task_with_config(
    registry: Arc<ConnectionRegistry>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    cfg: HeartbeatConfig,
) {
    let mut interval = tokio::time::interval(cfg.check_interval);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let removed = remove_expired_with_timeout(&registry, cfg.timeout);
                if removed > 0 {
                    tracing::info!(count = removed, "heartbeat: removed expired connections");
                }
            }
            _ = shutdown.changed() => {
                tracing::info!("heartbeat_task: shutdown signal received, stopping");
                break;
            }
        }
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::registry::ConnectionHandle;
    use std::sync::{Arc, RwLock};
    use tokio::sync::mpsc;
    use uuid::Uuid;

    fn make_handle_with_heartbeat(last_beat: Instant) -> ConnectionHandle {
        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(last_beat)),
        }
    }

    // H01: 心跳时间更新后（刚刚），检测器不会断开连接
    #[tokio::test]
    async fn h01_updated_heartbeat_not_expired() {
        let registry = ConnectionRegistry::new();
        let handle = make_handle_with_heartbeat(Instant::now());
        registry.register(handle);
        assert_eq!(registry.count(), 1);

        let removed = remove_expired(&registry);
        assert_eq!(removed, 0);
        assert_eq!(registry.count(), 1);
    }

    // H02: 超过 30 秒无心跳，连接被标记为过期并从 registry 移除
    #[tokio::test]
    async fn h02_expired_heartbeat_detected() {
        let registry = ConnectionRegistry::new();
        let stale = Instant::now() - Duration::from_secs(31);
        let handle = make_handle_with_heartbeat(stale);
        registry.register(handle);
        assert_eq!(registry.count(), 1);

        let removed = remove_expired(&registry);
        assert_eq!(removed, 1);
        assert_eq!(registry.count(), 0);
    }

    // H03: heartbeat_task 收到 shutdown 信号后退出
    #[tokio::test]
    async fn h03_heartbeat_task_stops_on_shutdown() {
        let registry = Arc::new(ConnectionRegistry::new());
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(heartbeat_task(registry, shutdown_rx));
        shutdown_tx.send(true).expect("shutdown send should succeed");
        tokio::time::timeout(Duration::from_millis(200), task)
            .await
            .expect("heartbeat_task should stop within 200ms")
            .expect("task should not panic");
    }

    // H04 (P2-17): 心跳超时时显式向客户端发送 connection_closed/heartbeat_timeout 帧
    #[tokio::test]
    async fn h04_expired_connection_receives_explicit_shutdown_frame() {
        let registry = ConnectionRegistry::new();
        let stale = Instant::now() - Duration::from_secs(31);
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let handle = ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(stale)),
        };
        registry.register(handle);

        let removed = remove_expired(&registry);
        assert_eq!(removed, 1);

        let msg = rx.try_recv().expect("P2-17: 必须显式发送 connection_closed 帧");
        let json: serde_json::Value =
            serde_json::from_str(&msg).expect("timeout 帧必须是合法 JSON");
        assert_eq!(json["type"], "connection_closed");
        assert_eq!(json["reason"], "heartbeat_timeout");
    }

    // T-00041 — close-frame 映射：HEARTBEAT_TIMEOUT_MESSAGE → Close(1000, "Heartbeat timeout")
    #[test]
    fn t41_close_frame_for_heartbeat_timeout_message() {
        let frame = close_frame_for_message(HEARTBEAT_TIMEOUT_MESSAGE)
            .expect("heartbeat timeout text 必须映射为 Close 帧");
        assert_eq!(frame.code, 1000, "Close code 必须为 1000 (Normal Closure)");
        assert_eq!(
            frame.reason.as_ref() as &str,
            "Heartbeat timeout",
            "reason 必须为 'Heartbeat timeout'"
        );
    }

    #[test]
    fn t41_close_frame_for_unrelated_message_is_none() {
        assert!(close_frame_for_message(r#"{"type":"chat","msg":"hi"}"#).is_none());
        assert!(close_frame_for_message("").is_none());
    }

    // T-00041 U-2：客户端静默 35s — 通过将 last_heartbeat 回拨到 35s 前模拟，
    // heartbeat_task 在真实 select! / interval 路径中触发剔除并发送显式 close 帧。
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn t41_u2_silent_35s_triggers_active_close_with_code_1000() {
        let registry = Arc::new(ConnectionRegistry::new());

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let stale = Instant::now() - Duration::from_secs(35);
        registry.register(ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(stale)),
        });

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(heartbeat_task_with_config(
            registry.clone(),
            shutdown_rx,
            HeartbeatConfig::default(),
        ));

        // 真实 select! 路径：等 task 启动 → 推 5s 触发首个非零 tick → 等清理完成
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;

        assert_eq!(
            registry.count(),
            0,
            "U-2: 静默 35s 后连接必须从 registry 移除"
        );

        let msg = rx
            .try_recv()
            .expect("U-2: 必须收到 connection_closed/heartbeat_timeout 帧");
        let frame =
            close_frame_for_message(&msg).expect("U-2: timeout 帧必须映射为 Close 帧");
        assert_eq!(frame.code, 1000, "U-2: close code 必须为 1000");
        assert_eq!(
            frame.reason.as_ref() as &str,
            "Heartbeat timeout",
            "U-2: reason 必须为 'Heartbeat timeout'"
        );

        let _ = shutdown_tx.send(true);
        let _ = task.await;
    }

    // T-00041 U-3：边界值 — 静默 29s 不断开；静默 31s 必须断开。
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn t41_u3_boundary_29s_alive_31s_dead() {
        let registry = Arc::new(ConnectionRegistry::new());

        let (tx_a, _rx_a) = mpsc::unbounded_channel::<String>();
        let alive_id = Uuid::new_v4();
        registry.register(ConnectionHandle {
            connection_id: alive_id,
            user_id: Uuid::new_v4(),
            room_id: None,
            sender: tx_a,
            last_heartbeat: Arc::new(RwLock::new(Instant::now() - Duration::from_secs(29))),
        });

        let (tx_d, _rx_d) = mpsc::unbounded_channel::<String>();
        let dead_id = Uuid::new_v4();
        registry.register(ConnectionHandle {
            connection_id: dead_id,
            user_id: Uuid::new_v4(),
            room_id: None,
            sender: tx_d,
            last_heartbeat: Arc::new(RwLock::new(Instant::now() - Duration::from_secs(31))),
        });
        assert_eq!(registry.count(), 2);

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(heartbeat_task_with_config(
            registry.clone(),
            shutdown_rx,
            HeartbeatConfig::default(),
        ));

        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;

        assert_eq!(
            registry.count(),
            1,
            "U-3: 仅静默 31s 的连接被剔除，静默 29s 的连接保持"
        );
        assert!(
            registry.connections.contains_key(&alive_id),
            "U-3 下界: 29s 连接必须保留"
        );
        assert!(
            !registry.connections.contains_key(&dead_id),
            "U-3 上界: 31s 连接必须被剔除"
        );

        let _ = shutdown_tx.send(true);
        let _ = task.await;
    }

    // T-00041 U-1：客户端每 15s 刷新一次 last_heartbeat，连续 3 分钟连接不断开。
    // 真实 interval/select! 路径：tokio time 驱动 tick；每 15s 把 last_heartbeat 重置为 real now。
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn t41_u1_keepalive_every_15s_for_3min_no_drop() {
        let registry = Arc::new(ConnectionRegistry::new());

        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        let last_hb = Arc::new(RwLock::new(Instant::now()));
        registry.register(ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            room_id: None,
            sender: tx,
            last_heartbeat: last_hb.clone(),
        });

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(heartbeat_task_with_config(
            registry.clone(),
            shutdown_rx,
            HeartbeatConfig::default(),
        ));
        tokio::task::yield_now().await;

        // 模拟 12 轮 × 15s = 180s（3 分钟）保活
        for _ in 0..12 {
            tokio::time::sleep(Duration::from_secs(15)).await;
            *last_hb.write().expect("hb lock") = Instant::now();
            tokio::task::yield_now().await;
        }

        assert_eq!(
            registry.count(),
            1,
            "U-1: 客户端每 15s 保活 3 分钟，连接不应被剔除"
        );

        let _ = shutdown_tx.send(true);
        let _ = task.await;
    }

    // T-00041 U-5：10 连接并发隔离 — 5 个保活、5 个静默；只有静默的 5 个被剔除。
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn t41_u5_concurrent_isolation_only_silent_dropped() {
        let registry = Arc::new(ConnectionRegistry::new());

        let mut alive_ids = Vec::new();
        for _ in 0..5 {
            let (tx, _rx) = mpsc::unbounded_channel::<String>();
            let id = Uuid::new_v4();
            registry.register(ConnectionHandle {
                connection_id: id,
                user_id: Uuid::new_v4(),
                room_id: None,
                sender: tx,
                last_heartbeat: Arc::new(RwLock::new(Instant::now())),
            });
            alive_ids.push(id);
        }

        let mut dead_ids = Vec::new();
        for _ in 0..5 {
            let (tx, _rx) = mpsc::unbounded_channel::<String>();
            let id = Uuid::new_v4();
            registry.register(ConnectionHandle {
                connection_id: id,
                user_id: Uuid::new_v4(),
                room_id: None,
                sender: tx,
                last_heartbeat: Arc::new(RwLock::new(Instant::now() - Duration::from_secs(35))),
            });
            dead_ids.push(id);
        }
        assert_eq!(registry.count(), 10);

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(heartbeat_task_with_config(
            registry.clone(),
            shutdown_rx,
            HeartbeatConfig::default(),
        ));
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;

        assert_eq!(
            registry.count(),
            5,
            "U-5: 仅静默的 5 个连接应被剔除，保活的 5 个保留"
        );
        for id in &alive_ids {
            assert!(
                registry.connections.contains_key(id),
                "U-5: 保活连接 {id} 必须保留"
            );
        }
        for id in &dead_ids {
            assert!(
                !registry.connections.contains_key(id),
                "U-5: 静默连接 {id} 必须被剔除"
            );
        }

        let _ = shutdown_tx.send(true);
        let _ = task.await;
    }
}
