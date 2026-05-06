//! BroadcastNoticeService — 系统公告广播
//!
//! PROTO-BINDING: doc/protocol/schemas/pubsub/BroadcastNotice.schema.json
//!
//! 通过 Redis Pub/Sub `admin:events` 频道向所有在线用户广播公告。
//!
//! # 职责
//! 1. 校验消息内容（schema: minLength=1）
//! 2. 构造 `AdminEvent::BroadcastNotice` strict enum（T-00105）
//! 3. 发布到 `admin:events` 频道（fire-and-forget）

use std::sync::Arc;

use uuid::Uuid;

use crate::common::error::AppError;
use crate::modules::event::publisher::{AdminEvent, BroadcastNoticePayload, EventPublisher};

// ─── BroadcastNoticeService ──────────────────────────────────────────────────

/// 广播通知服务。
///
/// 职责：校验 + 发布 BroadcastNotice 事件到 `admin:events` 频道。
pub struct BroadcastNoticeService {
    event_publisher: Arc<dyn EventPublisher>,
}

impl BroadcastNoticeService {
    pub fn new(event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self { event_publisher }
    }

    /// 发布全局广播公告。
    ///
    /// # 校验规则
    /// - `message` 不能为空（schema: minLength=1）
    ///
    /// # 行为
    /// - 发布成功：返回 `Ok(())`
    /// - 发布失败（Redis 错误）：记录 warn 日志，仍返回 `Ok(())`（fire-and-forget）
    pub async fn broadcast_notice(
        &self,
        operator_id: Uuid,
        message: String,
    ) -> Result<(), AppError> {
        // schema: minLength=1
        if message.trim().is_empty() {
            return Err(AppError::ValidationError(
                "broadcast message must not be empty (schema: minLength=1)".to_string(),
            ));
        }

        // T-00105: strict AdminEvent enum，消除 r#type 字符串拼写错误
        let event = AdminEvent::BroadcastNotice {
            payload: BroadcastNoticePayload { message },
            admin_id: operator_id,
            ts: chrono::Utc::now().timestamp_millis(),
        };

        if let Err(e) = self.event_publisher.publish("admin:events", event).await {
            tracing::warn!(error = %e, "failed to publish broadcast_notice event");
        }

        Ok(())
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::event::publisher::{ErrorEventPublisher, NoopEventPublisher};

    fn make_service() -> (BroadcastNoticeService, Arc<NoopEventPublisher>) {
        let publisher = Arc::new(NoopEventPublisher::default());
        let svc = BroadcastNoticeService::new(publisher.clone() as Arc<dyn EventPublisher>);
        (svc, publisher)
    }

    // BN-01: 正常消息发布成功，NoopPublisher 记录 1 次调用
    #[tokio::test]
    async fn bn01_broadcast_notice_publishes_event() {
        let (svc, publisher) = make_service();
        let operator_id = Uuid::new_v4();
        let message = "系统维护通知：今晚 22:00 停服 30 分钟".to_string();

        let result = svc
            .broadcast_notice(operator_id, message.clone())
            .await;

        assert!(result.is_ok(), "BN-01: broadcast_notice 应返回 Ok(())");

        let calls = publisher.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "BN-01: 应发布恰好 1 次事件");
        assert_eq!(calls[0].0, "admin:events", "BN-01: channel 应为 admin:events");

        match &calls[0].1 {
            AdminEvent::BroadcastNotice { payload, admin_id, .. } => {
                assert_eq!(payload.message, message, "BN-01: message 应与输入一致");
                assert_eq!(*admin_id, operator_id, "BN-01: admin_id 应为 operator_id");
            }
            other => panic!("BN-01: 期望 BroadcastNotice 变体，实际: {:?}", other),
        }
    }

    // BN-02: 空消息 → ValidationError（schema: minLength=1）
    #[tokio::test]
    async fn bn02_empty_message_returns_validation_error() {
        let (svc, _) = make_service();

        let result = svc.broadcast_notice(Uuid::new_v4(), String::new()).await;
        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "BN-02: 空 message 应返回 ValidationError"
        );
    }

    // BN-03: 纯空白消息 → ValidationError
    #[tokio::test]
    async fn bn03_whitespace_only_message_returns_validation_error() {
        let (svc, _) = make_service();

        let result = svc.broadcast_notice(Uuid::new_v4(), "   \t\n  ".to_string()).await;
        assert!(
            matches!(result, Err(AppError::ValidationError(_))),
            "BN-03: 纯空白 message 应返回 ValidationError"
        );
    }

    // BN-04: Redis 失败时仍返回 Ok（fire-and-forget）
    #[tokio::test]
    async fn bn04_redis_error_does_not_affect_result() {
        let publisher = Arc::new(ErrorEventPublisher);
        let svc = BroadcastNoticeService::new(publisher as Arc<dyn EventPublisher>);

        let result = svc
            .broadcast_notice(Uuid::new_v4(), "test notice".to_string())
            .await;

        assert!(
            result.is_ok(),
            "BN-04: Redis 发布失败不应影响 broadcast_notice 返回值（fire-and-forget）"
        );
    }

    // BN-05: Unicode 消息正确发布
    #[tokio::test]
    async fn bn05_unicode_message_publishes_correctly() {
        let (svc, publisher) = make_service();
        let msg = "🔴 紧急公告 — 系统将进行升级".to_string();

        svc.broadcast_notice(Uuid::new_v4(), msg.clone())
            .await
            .expect("BN-05: Unicode 消息应发布成功");

        let calls = publisher.calls.lock().unwrap();
        match &calls[0].1 {
            AdminEvent::BroadcastNotice { payload, .. } => {
                assert_eq!(payload.message, msg, "BN-05: Unicode 消息必须原样传递");
            }
            other => panic!("BN-05: 期望 BroadcastNotice，实际: {:?}", other),
        }
    }
}
