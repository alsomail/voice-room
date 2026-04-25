// ─── 错误类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum EventPublishError {
    #[error("Redis error: {0}")]
    RedisError(String),
    #[error("Serialize error: {0}")]
    SerializeError(String),
}

// ─── 事件结构体 ─────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct AdminEvent {
    pub r#type: String,
    pub payload: serde_json::Value,
    pub admin_id: String,
    pub ts: i64,
}

// ─── trait ──────────────────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, channel: &str, event: AdminEvent) -> Result<(), EventPublishError>;
}

// ─── 生产实现 ────────────────────────────────────────────────────────────────

pub struct RedisEventPublisher {
    client: redis::Client,
}

impl RedisEventPublisher {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl EventPublisher for RedisEventPublisher {
    async fn publish(&self, channel: &str, event: AdminEvent) -> Result<(), EventPublishError> {
        let payload = serde_json::to_string(&event)
            .map_err(|e| EventPublishError::SerializeError(e.to_string()))?;
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| EventPublishError::RedisError(e.to_string()))?;
        redis::cmd("PUBLISH")
            .arg(channel)
            .arg(&payload)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| EventPublishError::RedisError(e.to_string()))
    }
}

// ─── 测试用 Noop 实现 ────────────────────────────────────────────────────────

/// 测试专用：始终成功，记录调用历史供断言
pub struct NoopEventPublisher {
    pub calls: std::sync::Mutex<Vec<(String, AdminEvent)>>,
}

impl Default for NoopEventPublisher {
    fn default() -> Self {
        Self {
            calls: std::sync::Mutex::new(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(&self, channel: &str, event: AdminEvent) -> Result<(), EventPublishError> {
        self.calls
            .lock()
            .unwrap()
            .push((channel.to_string(), event));
        Ok(())
    }
}

/// 测试专用：始终失败，用于验证 fire-and-forget 不影响主业务
pub struct ErrorEventPublisher;

#[async_trait::async_trait]
impl EventPublisher for ErrorEventPublisher {
    async fn publish(&self, _channel: &str, _event: AdminEvent) -> Result<(), EventPublishError> {
        Err(EventPublishError::RedisError(
            "mock connection refused".to_string(),
        ))
    }
}

// ─── 单元测试（EP-01~03）────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(event_type: &str) -> AdminEvent {
        AdminEvent {
            r#type: event_type.to_string(),
            payload: serde_json::json!({ "user_id": "abc-123" }),
            admin_id: "admin-1".to_string(),
            ts: 1713312000,
        }
    }

    // ── EP-01: NoopEventPublisher::publish → Ok(())，calls 长度 +1 ────────────
    #[tokio::test]
    async fn ep01_noop_publish_returns_ok_and_calls_increases() {
        let publisher = NoopEventPublisher::default();
        let event = make_event("ban_user");

        let result = publisher.publish("admin:events", event).await;

        assert!(
            result.is_ok(),
            "EP-01: NoopEventPublisher.publish 应返回 Ok(())"
        );
        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "EP-01: calls 长度应为 1");
        assert_eq!(
            calls[0].0, "admin:events",
            "EP-01: channel 应为 admin:events"
        );
        assert_eq!(
            calls[0].1.r#type, "ban_user",
            "EP-01: event.type 应为 ban_user"
        );
    }

    // ── EP-02: ErrorEventPublisher::publish → Err(RedisError) ─────────────────
    #[tokio::test]
    async fn ep02_error_publisher_returns_err_redis_error() {
        let publisher = ErrorEventPublisher;
        let event = make_event("ban_user");

        let result = publisher.publish("admin:events", event).await;

        assert!(
            matches!(result, Err(EventPublishError::RedisError(_))),
            "EP-02: ErrorEventPublisher.publish 应返回 Err(RedisError)"
        );
    }

    // ── EP-03: AdminEvent 序列化后 JSON 包含 type/payload/admin_id/ts 四个字段 ──
    #[tokio::test]
    async fn ep03_admin_event_serializes_with_four_top_level_fields() {
        let event = AdminEvent {
            r#type: "ban_user".to_string(),
            payload: serde_json::json!({ "user_id": "550e8400" }),
            admin_id: "admin-xyz".to_string(),
            ts: 1713312000,
        };

        let json_str = serde_json::to_string(&event).expect("序列化不应失败");
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(
            value["type"].as_str().is_some(),
            "EP-03: JSON 应含 type 字段"
        );
        assert_eq!(value["type"].as_str().unwrap(), "ban_user");
        assert!(
            value["payload"].is_object(),
            "EP-03: JSON 应含 payload 字段"
        );
        assert_eq!(value["admin_id"].as_str().unwrap(), "admin-xyz");
        assert_eq!(value["ts"].as_i64().unwrap(), 1713312000);
    }
}
