//! Gift Ranking — Redis ZSet 榜单封装
//!
//! ## 键设计（Asia/Riyadh 时区，但 MVP 暂用 UTC 简化）
//! - 日榜：`ranking:charm:day:{YYYY-MM-DD}`  / `ranking:wealth:day:{YYYY-MM-DD}`
//! - 周榜：`ranking:charm:week:{YYYY-WW}`    / `ranking:wealth:week:{YYYY-WW}`
//! - TTL：日榜 48h，周榜 10d
//!
//! 供 `GiftSendService` 在送礼事务提交后调用，更新接收者魅力榜和发送者财富榜。

use redis::{aio::MultiplexedConnection, AsyncCommands};
use uuid::Uuid;

const DAY_TTL_SECS: u64 = 172_800; // 48h
const WEEK_TTL_SECS: u64 = 864_000; // 10d

/// 获取当前日榜 key（UTC 日期，格式 YYYY-MM-DD）
pub fn charm_day_key() -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    format!("ranking:charm:day:{}", today)
}

/// 获取当前周榜 key（UTC 年+周，格式 YYYY-WW）
pub fn charm_week_key() -> String {
    let now = chrono::Utc::now();
    let week = now.format("%Y-%W").to_string();
    format!("ranking:charm:week:{}", week)
}

/// 获取当前财富日榜 key
pub fn wealth_day_key() -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    format!("ranking:wealth:day:{}", today)
}

/// 获取当前财富周榜 key
pub fn wealth_week_key() -> String {
    let now = chrono::Utc::now();
    let week = now.format("%Y-%W").to_string();
    format!("ranking:wealth:week:{}", week)
}

/// 更新四个 ZSet（魅力日榜/周榜、财富日榜/周榜）
///
/// - `receiver_id`：接收者 UUID（魅力榜）
/// - `sender_id`：发送者 UUID（财富榜）
/// - `total`：本次礼物总价（ZINCRBY 增量）
///
/// Redis 错误不影响主流程（记录 warn 日志后返回 Ok）。
pub async fn update_rankings(
    conn: &mut MultiplexedConnection,
    receiver_id: Uuid,
    sender_id: Uuid,
    total: i64,
) {
    let recv_str = receiver_id.to_string();
    let sender_str = sender_id.to_string();
    let score = total as f64;

    // 魅力日榜
    let charm_day = charm_day_key();
    if let Err(e) = conn.zadd::<_, _, _, ()>(&charm_day, &recv_str, score).await {
        tracing::warn!("ranking: charm day zadd failed: {}", e);
    } else {
        let _ = increment_zscore(conn, &charm_day, &recv_str, score).await;
    }
    let _: Result<(), _> = conn.expire(&charm_day, DAY_TTL_SECS as i64).await;

    // 魅力周榜
    let charm_week = charm_week_key();
    let _ = increment_zscore(conn, &charm_week, &recv_str, score).await;
    let _: Result<(), _> = conn.expire(&charm_week, WEEK_TTL_SECS as i64).await;

    // 财富日榜
    let wealth_day = wealth_day_key();
    let _ = increment_zscore(conn, &wealth_day, &sender_str, score).await;
    let _: Result<(), _> = conn.expire(&wealth_day, DAY_TTL_SECS as i64).await;

    // 财富周榜
    let wealth_week = wealth_week_key();
    let _ = increment_zscore(conn, &wealth_week, &sender_str, score).await;
    let _: Result<(), _> = conn.expire(&wealth_week, WEEK_TTL_SECS as i64).await;
}

/// ZINCRBY wrapper：若成员不存在则从 0 开始累加
async fn increment_zscore(
    conn: &mut MultiplexedConnection,
    key: &str,
    member: &str,
    increment: f64,
) -> redis::RedisResult<f64> {
    conn.zincr(key, member, increment).await
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // RK01: charm_day_key 格式正确（YYYY-MM-DD）
    #[test]
    fn rk01_charm_day_key_format() {
        let key = charm_day_key();
        assert!(key.starts_with("ranking:charm:day:"), "RK01: key should start with correct prefix");
        let date_part = key.strip_prefix("ranking:charm:day:").unwrap();
        assert_eq!(date_part.len(), 10, "RK01: date part should be YYYY-MM-DD (10 chars)");
        assert!(date_part.contains('-'), "RK01: date part should contain dashes");
    }

    // RK02: charm_week_key 格式正确（YYYY-WW）
    #[test]
    fn rk02_charm_week_key_format() {
        let key = charm_week_key();
        assert!(key.starts_with("ranking:charm:week:"), "RK02: key should start with correct prefix");
        let week_part = key.strip_prefix("ranking:charm:week:").unwrap();
        assert!(!week_part.is_empty(), "RK02: week part should not be empty");
    }

    // RK03: wealth_day_key 格式正确
    #[test]
    fn rk03_wealth_day_key_format() {
        let key = wealth_day_key();
        assert!(key.starts_with("ranking:wealth:day:"), "RK03: key should start with correct prefix");
    }

    // RK04: wealth_week_key 格式正确
    #[test]
    fn rk04_wealth_week_key_format() {
        let key = wealth_week_key();
        assert!(key.starts_with("ranking:wealth:week:"), "RK04: key should start with correct prefix");
    }

    // RK05: 同日期两次调用返回相同 key
    #[test]
    fn rk05_same_day_same_key() {
        let key1 = charm_day_key();
        let key2 = charm_day_key();
        assert_eq!(key1, key2, "RK05: same day should produce same key");
    }
}
