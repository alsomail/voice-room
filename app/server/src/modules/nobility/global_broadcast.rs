//! 全服广播端口 — NobleEntranceGlobal (T-00069 §10.4.6)
//!
//! 抽象 `GlobalBroadcastPort` trait，允许测试时注入 `FakeGlobalBroadcast`，
//! 生产时可替换为 Redis Pub/Sub 实现。
//!
//! 数据流：
//!  JoinRoom handler (level >= 5)
//!    → GlobalBroadcastPort::try_publish_noble_entrance(...)
//!      → INSERT noble_global_broadcast_log ON CONFLICT DO NOTHING（频控）
//!      → PUBLISH noble:global <NobleEntranceGlobal payload>
//!  其他 server 实例 SUBSCRIBE noble:global → 推送给所有在线 WsConnection

use async_trait::async_trait;
use std::sync::Mutex;
use uuid::Uuid;

// ─── Port trait ───────────────────────────────────────────────────────────────

/// 全服广播端口：供 JoinRoom handler 注入（T-00069 §10.4.6）
///
/// 实现应包含：
/// 1. 今日频控（INSERT INTO noble_global_broadcast_log ON CONFLICT DO NOTHING）
/// 2. Redis PUBLISH noble:global NobleEntranceGlobal payload
///
/// `None` 时 handler 跳过（无 Redis / 测试环境），不影响历史测试。
#[async_trait]
pub trait GlobalBroadcastPort: Send + Sync {
    /// 尝试发布 LV5+ 贵族进场全服通知（§10.4.6）
    ///
    /// 返回 `true` 表示今日首次（已发布）；
    /// 返回 `false` 表示今日已发（频控命中，跳过）。
    async fn try_publish_noble_entrance(
        &self,
        user_id: Uuid,
        tier_id: &str,
        level: i16,
        nickname: &str,
    ) -> bool;
}

// ─── Fake 实现 ────────────────────────────────────────────────────────────────

/// 内存测试替身：记录所有已发布的进场广播（用于断言）
pub struct FakeGlobalBroadcast {
    pub published: Mutex<Vec<(Uuid, String, i16, String)>>,
}

impl FakeGlobalBroadcast {
    pub fn new() -> Self {
        Self {
            published: Mutex::new(vec![]),
        }
    }

    /// 返回已记录的广播数量
    pub fn published_count(&self) -> usize {
        self.published.lock().unwrap().len()
    }

    /// 返回所有已记录的广播（克隆）
    pub fn all_published(&self) -> Vec<(Uuid, String, i16, String)> {
        self.published.lock().unwrap().clone()
    }
}

impl Default for FakeGlobalBroadcast {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GlobalBroadcastPort for FakeGlobalBroadcast {
    async fn try_publish_noble_entrance(
        &self,
        user_id: Uuid,
        tier_id: &str,
        level: i16,
        nickname: &str,
    ) -> bool {
        self.published.lock().unwrap().push((
            user_id,
            tier_id.to_string(),
            level,
            nickname.to_string(),
        ));
        true // Fake 始终返回 true（成功发布，无频控）
    }
}

// ─── 辅助纯函数 ───────────────────────────────────────────────────────────────

/// 判断该 level 是否触发全服广播（LV5+ duke/king；§10.4.6）
///
/// 注意：此函数语义为「是否触发 NobleEntranceGlobal」，不应用于其他判断。
pub fn can_trigger_global_broadcast_for_level(level: i16) -> bool {
    level >= 5
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // GB-01: FakeGlobalBroadcast 记录发布事件
    #[tokio::test]
    async fn gb01_fake_records_publish() {
        let fake = FakeGlobalBroadcast::new();
        let user_id = Uuid::new_v4();
        let published = fake
            .try_publish_noble_entrance(user_id, "duke", 5, "Ali")
            .await;
        assert!(published, "GB-01: fake should return true for first publish");
        assert_eq!(fake.published_count(), 1, "GB-01: should record 1 publish");

        let records = fake.all_published();
        assert_eq!(records[0].0, user_id, "GB-01: user_id should match");
        assert_eq!(records[0].1, "duke", "GB-01: tier_id should match");
        assert_eq!(records[0].2, 5, "GB-01: level should match");
        assert_eq!(records[0].3, "Ali", "GB-01: nickname should match");
    }

    // GB-02: FakeGlobalBroadcast 满足 Send + Sync + dyn 约束
    #[test]
    fn gb02_fake_is_send_sync() {
        let _: Arc<dyn GlobalBroadcastPort> = Arc::new(FakeGlobalBroadcast::new());
    }

    // GB-03: can_trigger_global_broadcast_for_level — LV5+ 触发，LV4- 不触发
    #[test]
    fn gb03_can_trigger_by_level() {
        // LV5+ (duke/king) → 触发
        assert!(can_trigger_global_broadcast_for_level(5));
        assert!(can_trigger_global_broadcast_for_level(6));
        // LV4- (earl 及以下) → 不触发
        assert!(!can_trigger_global_broadcast_for_level(4));
        assert!(!can_trigger_global_broadcast_for_level(3));
        assert!(!can_trigger_global_broadcast_for_level(1));
    }

    // GB-04: FakeGlobalBroadcast 多次发布累计
    #[tokio::test]
    async fn gb04_fake_accumulates_publishes() {
        let fake = FakeGlobalBroadcast::new();
        let uid1 = Uuid::new_v4();
        let uid2 = Uuid::new_v4();
        fake.try_publish_noble_entrance(uid1, "duke", 5, "UserA").await;
        fake.try_publish_noble_entrance(uid2, "king", 6, "UserB").await;
        assert_eq!(fake.published_count(), 2, "GB-04: should record 2 publishes");
    }
}
