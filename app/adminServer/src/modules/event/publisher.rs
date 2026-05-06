// PROTO-BINDING: doc/protocol/schemas/pubsub/BanUser.schema.json
// PROTO-BINDING: doc/protocol/schemas/pubsub/UnbanUser.schema.json
// PROTO-BINDING: doc/protocol/schemas/pubsub/CloseRoom.schema.json
// PROTO-BINDING: doc/protocol/schemas/pubsub/BroadcastNotice.schema.json

// ─── 共享事件类型（来自 voice_room_shared crate）────────────────────────────────
//
// AdminEvent 的权威定义在 `app/shared/src/admin_event.rs`。
// 此处仅重导出，确保整个 adminServer 使用同一份 strict enum，
// 消除 schema-less 字符串拼写错误风险。
pub use voice_room_shared::admin_event::{
    AdminEvent, BanUserPayload, BroadcastNoticePayload, CloseRoomPayload, UnbanUserPayload,
};

// ─── 错误类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum EventPublishError {
    #[error("Redis error: {0}")]
    RedisError(String),
    #[error("Serialize error: {0}")]
    SerializeError(String),
}

// ─── 非治理用途的通用 Raw 事件结构体 ───────────────────────────────────────────
//
// 用于 gift（gift_cache_invalidate）和 wallet（balance_updated）等事件，
// 这些事件不属于 admin governance 协议范围，不使用 strict AdminEvent enum。
#[derive(Debug, serde::Serialize)]
pub struct RawEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
    pub admin_id: String,
    pub ts: i64,
}

// ─── EventPublisher trait ─────────────────────────────────────────────────────
//
// 统一的发布器接口，包含两类方法：
//  - `publish`     : admin governance 事件（strict AdminEvent enum）
//                    → user/service.rs, room/service.rs, notice_service.rs
//  - `publish_raw` : 非治理用途（gift cache invalidate / balance_updated 等）
//                    → gift/service.rs, wallet/service.rs
//
// 使用单一 trait 的好处：bootstrap/mod.rs 的 DI 接线保持不变，
// Arc<dyn EventPublisher> 作为单一字段注入各 service。

#[async_trait::async_trait]
pub trait EventPublisher: Send + Sync {
    /// 发布 admin governance 事件（严格类型）
    async fn publish(
        &self,
        channel: &str,
        event: AdminEvent,
    ) -> Result<(), EventPublishError>;

    /// 发布非治理用途的 raw 事件（gift_cache_invalidate / balance_updated 等）
    async fn publish_raw(
        &self,
        channel: &str,
        event: RawEvent,
    ) -> Result<(), EventPublishError>;
}

// ─── 生产实现 ────────────────────────────────────────────────────────────────

pub struct RedisEventPublisher {
    client: redis::Client,
}

impl RedisEventPublisher {
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    async fn publish_json<T: serde::Serialize>(
        &self,
        channel: &str,
        value: &T,
    ) -> Result<(), EventPublishError> {
        let payload = serde_json::to_string(value)
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

#[async_trait::async_trait]
impl EventPublisher for RedisEventPublisher {
    async fn publish(
        &self,
        channel: &str,
        event: AdminEvent,
    ) -> Result<(), EventPublishError> {
        self.publish_json(channel, &event).await
    }

    async fn publish_raw(
        &self,
        channel: &str,
        event: RawEvent,
    ) -> Result<(), EventPublishError> {
        self.publish_json(channel, &event).await
    }
}

// ─── 测试用 Noop 实现 ────────────────────────────────────────────────────────

/// 测试专用：始终成功，分别记录 governance 和 raw 调用历史供断言
pub struct NoopEventPublisher {
    /// governance 事件调用记录（AdminEvent）
    pub calls: std::sync::Mutex<Vec<(String, AdminEvent)>>,
    /// 非治理 raw 事件调用记录（RawEvent）
    pub raw_calls: std::sync::Mutex<Vec<(String, RawEvent)>>,
}

impl Default for NoopEventPublisher {
    fn default() -> Self {
        Self {
            calls: std::sync::Mutex::new(vec![]),
            raw_calls: std::sync::Mutex::new(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl EventPublisher for NoopEventPublisher {
    async fn publish(
        &self,
        channel: &str,
        event: AdminEvent,
    ) -> Result<(), EventPublishError> {
        self.calls
            .lock()
            .unwrap()
            .push((channel.to_string(), event));
        Ok(())
    }

    async fn publish_raw(
        &self,
        channel: &str,
        event: RawEvent,
    ) -> Result<(), EventPublishError> {
        self.raw_calls
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
    async fn publish(
        &self,
        _channel: &str,
        _event: AdminEvent,
    ) -> Result<(), EventPublishError> {
        Err(EventPublishError::RedisError(
            "mock connection refused".to_string(),
        ))
    }

    async fn publish_raw(
        &self,
        _channel: &str,
        _event: RawEvent,
    ) -> Result<(), EventPublishError> {
        Err(EventPublishError::RedisError(
            "mock connection refused".to_string(),
        ))
    }
}

// ─── 单元测试（EP-01~05）────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_admin_event() -> AdminEvent {
        AdminEvent::BanUser {
            payload: BanUserPayload {
                user_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            },
            admin_id: Uuid::parse_str("00000000-0000-0000-0000-000000000099").unwrap(),
            ts: 1_700_000_000,
        }
    }

    // ── EP-01: NoopEventPublisher::publish → Ok(())，calls 长度 +1 ────────────
    #[tokio::test]
    async fn ep01_noop_publish_returns_ok_and_calls_increases() {
        let publisher = NoopEventPublisher::default();
        let event = make_admin_event();

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
        assert!(
            matches!(&calls[0].1, AdminEvent::BanUser { .. }),
            "EP-01: 应为 BanUser 变体"
        );
    }

    // ── EP-02: ErrorEventPublisher::publish → Err(RedisError) ─────────────────
    #[tokio::test]
    async fn ep02_error_publisher_returns_err_redis_error() {
        let publisher = ErrorEventPublisher;
        let event = make_admin_event();

        let result = publisher.publish("admin:events", event).await;

        assert!(
            matches!(result, Err(EventPublishError::RedisError(_))),
            "EP-02: ErrorEventPublisher.publish 应返回 Err(RedisError)"
        );
    }

    // ── EP-03: AdminEvent 序列化后 JSON 包含 type/payload/admin_id/ts 四个字段 ──
    #[tokio::test]
    async fn ep03_admin_event_serializes_with_four_top_level_fields() {
        let event = make_admin_event();

        let json_str = serde_json::to_string(&event).expect("序列化不应失败");
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let top_keys: std::collections::BTreeSet<&str> =
            value.as_object().unwrap().keys().map(|s| s.as_str()).collect();
        let expected: std::collections::BTreeSet<&str> =
            ["type", "payload", "admin_id", "ts"].iter().cloned().collect();
        assert_eq!(top_keys, expected, "EP-03: 顶层字段必须严格匹配 schema");
        assert_eq!(value["type"].as_str(), Some("ban_user"));
    }

    // ── EP-04: 多次 publish 累积到 calls ─────────────────────────────────────
    #[tokio::test]
    async fn ep04_multiple_publishes_accumulate_in_calls() {
        let publisher = NoopEventPublisher::default();

        for _ in 0..4 {
            publisher
                .publish("admin:events", make_admin_event())
                .await
                .unwrap();
        }

        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 4, "EP-04: 应有 4 次发布调用");
    }

    // ── EP-05: RawEvent 通过 publish_raw 记录在 raw_calls，不混入 governance calls
    #[tokio::test]
    async fn ep05_raw_event_publishes_to_raw_calls_not_governance_calls() {
        let publisher = NoopEventPublisher::default();
        let raw = RawEvent {
            event_type: "gift_cache_invalidate".to_string(),
            payload: serde_json::json!({}),
            admin_id: "system".to_string(),
            ts: 12345,
        };

        publisher
            .publish_raw("admin:events", raw)
            .await
            .expect("EP-05: publish_raw 应返回 Ok");

        assert_eq!(
            publisher.raw_calls.lock().unwrap().len(),
            1,
            "EP-05: raw_calls 应为 1"
        );
        assert_eq!(
            publisher.calls.lock().unwrap().len(),
            0,
            "EP-05: governance calls 应为 0"
        );
        assert_eq!(
            publisher.raw_calls.lock().unwrap()[0].1.event_type,
            "gift_cache_invalidate"
        );
    }
}
