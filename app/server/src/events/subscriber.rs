//! Redis Pub/Sub 订阅 task
//!
//! 订阅 `admin:events` 频道，解析 `AdminEvent` JSON 并分发到 `handle_admin_event`。
//!
//! 特性：
//! - 自动重连（Redis 断线后 2s 重试）
//! - 优雅停机：通过 `watch::Receiver<bool>` 信号退出
//! - 每个消息在独立 `tokio::spawn` 中处理，异常不影响订阅循环

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
/// # 行为
/// - 连接失败/订阅失败：等待 2s 后重试
/// - 收到消息：在新 task 中调用 `handle_admin_event`
/// - 收到 shutdown 信号：立即返回
pub async fn start_admin_event_subscriber(
    redis_url: String,
    registry: Arc<ConnectionRegistry>,
    mut shutdown: watch::Receiver<bool>,
) {
    let client = redis::Client::open(redis_url).expect("Redis URL invalid");

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
                            let payload: String = msg.get_payload().unwrap_or_default();
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
                            return;
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
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}
