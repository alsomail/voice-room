//! 心跳检测模块
//!
//! 每 10 秒扫描所有连接的 last_heartbeat，超过 30 秒无心跳的连接将被移除。
//! `remove_expired` 暴露为纯函数以便单元测试直接调用，不依赖定时器。

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::registry::ConnectionRegistry;

/// 心跳超时阈值（30 秒无心跳则断开）
pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(30);
/// 检测间隔（每 10 秒扫描一次）
const CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// 移除所有心跳已超时的连接。
///
/// 使用 DashMap::retain 原子地扫描+删除，返回移除的连接数量。
pub fn remove_expired(registry: &ConnectionRegistry) -> usize {
    let now = Instant::now();
    let mut removed: usize = 0;

    registry.connections.retain(|connection_id, handle| {
        let elapsed = now.duration_since(
            *handle.last_heartbeat.read().expect("heartbeat lock poisoned"),
        );
        if elapsed > HEARTBEAT_TIMEOUT {
            tracing::info!(
                %connection_id,
                user_id = %handle.user_id,
                elapsed_secs = elapsed.as_secs(),
                "heartbeat expired, disconnecting"
            );
            removed += 1;
            false // retain=false → DashMap 移除此条目
        } else {
            true
        }
    });

    removed
}

/// 心跳检测后台 task：每 CHECK_INTERVAL 触发一次清理，支持优雅停机。
///
/// `shutdown` 为 watch channel 的接收端；发送端发送任意值或被 drop 时，task 退出。
pub async fn heartbeat_task(
    registry: Arc<ConnectionRegistry>,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let removed = remove_expired(&registry);
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

        // 最近一次心跳 = now（未超时）
        let handle = make_handle_with_heartbeat(Instant::now());
        registry.register(handle);
        assert_eq!(registry.count(), 1);

        let removed = remove_expired(&registry);

        assert_eq!(removed, 0, "fresh heartbeat connection should not be removed");
        assert_eq!(registry.count(), 1, "connection should still be in registry");
    }

    // H02: 超过 30 秒无心跳，连接被标记为过期并从 registry 移除
    #[tokio::test]
    async fn h02_expired_heartbeat_detected() {
        let registry = ConnectionRegistry::new();

        // 最近一次心跳 = 31 秒前（已超时）
        let stale = Instant::now() - Duration::from_secs(31);
        let handle = make_handle_with_heartbeat(stale);
        registry.register(handle);
        assert_eq!(registry.count(), 1);

        let removed = remove_expired(&registry);

        assert_eq!(removed, 1, "expired connection should be counted as removed");
        assert_eq!(registry.count(), 0, "expired connection should be removed from registry");
    }

    // H03: heartbeat_task 收到 shutdown 信号后退出
    #[tokio::test]
    async fn h03_heartbeat_task_stops_on_shutdown() {
        use std::time::Duration;

        let registry = Arc::new(ConnectionRegistry::new());
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let task = tokio::spawn(heartbeat_task(registry, shutdown_rx));

        // 发送 shutdown 信号
        shutdown_tx.send(true).expect("shutdown send should succeed");

        // task 应在短时间内退出，不会永久阻塞
        tokio::time::timeout(Duration::from_millis(200), task)
            .await
            .expect("heartbeat_task should stop within 200ms after shutdown signal")
            .expect("task should not panic");
    }
}
