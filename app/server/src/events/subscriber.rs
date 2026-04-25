//! Redis Pub/Sub 订阅 task
//!
//! 订阅 `admin:events` 频道，解析 `AdminEvent` JSON 并分发到 `handle_admin_event`。
//!
//! 特性：
//! - 自动重连（Redis 断线后 2s 重试）
//! - 优雅停机：通过 `watch::Receiver<bool>` 信号退出
//! - 每个消息在独立 `tokio::spawn` 中处理，异常不影响订阅循环
//! - P2-16：Redis URL 解析失败返回 `Result::Err` 而非 panic，由 bootstrap 决策
//! - P3-19：payload 解析失败显式 warn 日志，避免静默丢消息

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::watch;

use crate::events::admin_event::AdminEvent;
use crate::events::handler::handle_admin_event;
use crate::ws::ConnectionRegistry;

/// 启动管理事件订阅 task。
///
/// # 参数
/// - `redis_url`：Redis 连接字符串，如 `"redis://127.0.0.1:6379"`
/// - `registry`：全局连接注册表
/// - `shutdown`：`watch::Receiver<bool>` — 收到任意变更即退出
///
/// # 返回
/// - `Ok(())`：收到 shutdown 信号正常退出
/// - `Err(anyhow::Error)`：Redis URL 解析失败（启动期错误，由调用方决策 fail-fast）
///
/// # 行为
/// - 连接失败/订阅失败：等待 2s 后重试（不返回 Err，仅日志）
/// - 收到消息：在新 task 中调用 `handle_admin_event`
/// - 收到 shutdown 信号：立即返回 `Ok(())`
pub async fn start_admin_event_subscriber(
    redis_url: String,
    registry: Arc<ConnectionRegistry>,
    mut shutdown: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    // P2-16：URL 无效不再 panic，统一向上返回错误
    let client = redis::Client::open(redis_url.as_str())
        .map_err(|e| anyhow::anyhow!("invalid Redis URL for admin events subscriber: {e}"))?;

    loop {
        match client.get_async_pubsub().await {
            Ok(mut pubsub) => {
                if let Err(e) = pubsub.subscribe("admin:events").await {
                    tracing::error!("Failed to subscribe to admin:events: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }

                tracing::info!("Subscribed to Redis admin:events channel");

                let mut stream = pubsub.on_message();
                loop {
                    tokio::select! {
                        Some(msg) = stream.next() => {
                            // P3-19: payload 解析失败显式 warn，避免静默丢消息
                            let payload: String = match msg.get_payload::<String>() {
                                Ok(s) => s,
                                Err(e) => {
                                    tracing::warn!(
                                        error = ?e,
                                        channel = ?msg.get_channel_name(),
                                        "admin event payload not valid UTF-8 string, dropped"
                                    );
                                    continue;
                                }
                            };
                            let registry_clone = registry.clone();
                            tokio::spawn(async move {
                                match serde_json::from_str::<AdminEvent>(&payload) {
                                    Ok(event) => {
                                        handle_admin_event(event, registry_clone).await;
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            error = ?e,
                                            raw = %payload,
                                            "Failed to parse admin event from Redis"
                                        );
                                    }
                                }
                            });
                        }
                        _ = shutdown.changed() => {
                            tracing::info!("Admin event subscriber shutting down");
                            return Ok(());
                        }
                        else => {
                            tracing::warn!("Redis pubsub stream ended unexpectedly, reconnecting");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Redis connection failed: {:?}", e);
                // 在重试 sleep 期间也尊重 shutdown 信号，避免最长 2s 的退出延迟
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                    _ = shutdown.changed() => {
                        tracing::info!("Admin event subscriber shutting down (during reconnect backoff)");
                        return Ok(());
                    }
                }
            }
        }
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // SUB-01 (P2-16): 非法 Redis URL 返回 Err，不 panic
    #[tokio::test]
    async fn sub01_invalid_redis_url_returns_err_not_panic() {
        let registry = Arc::new(ConnectionRegistry::new());
        let (_tx, rx) = watch::channel(false);

        // "://" 缺少 scheme，redis::Client::open 必返回 Err
        let result =
            start_admin_event_subscriber("not-a-valid-url".to_string(), registry, rx).await;

        assert!(
            result.is_err(),
            "P2-16: 非法 Redis URL 必须返回 Err 由 bootstrap 决策，禁止 panic"
        );
    }

    // SUB-02 (P2-16): shutdown 信号下正常返回 Ok，即使在 connect 失败重试 backoff 期间
    #[tokio::test]
    async fn sub02_shutdown_signal_returns_ok_during_reconnect_backoff() {
        let registry = Arc::new(ConnectionRegistry::new());
        let (tx, rx) = watch::channel(false);

        // 端口 1 通常无 Redis → 进入 connect 失败重试分支
        let task = tokio::spawn(start_admin_event_subscriber(
            "redis://127.0.0.1:1".to_string(),
            registry,
            rx,
        ));

        // 让 task 进入 connect 失败 backoff
        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(true).expect("shutdown send should succeed");

        let outer = tokio::time::timeout(Duration::from_millis(500), task)
            .await
            .expect("subscriber should exit within 500ms after shutdown");
        outer.expect("task should not panic")
            .expect("shutdown 路径必须返回 Ok(())");
    }
}
