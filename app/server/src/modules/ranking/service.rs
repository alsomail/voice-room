//! RankingService — 榜单查询核心逻辑
//!
//! ## 数据流
//! ```text
//! top_by_key(key, limit, viewer)
//!   ├── ZREVRANGE key 0 (limit-1) WITHSCORES → [(member, score)]
//!   ├── batch SELECT users WHERE id IN (user_ids) → HashMap<Uuid, (nickname, avatar)>
//!   ├── ZREVRANK key viewer_id → Option<u64> (0-based)
//!   ├── ZSCORE key viewer_id   → Option<f64>
//!   └── 组装 RankingResult { items, me }
//! ```
//!
//! ## 接口说明
//! - `RankingServicePort` trait：HTTP handler 依赖的抽象，支持 FakeRankingService 测试替身
//! - `RankingService`：真实实现（PgPool + redis::Client）
//! - `FakeRankingService`：`#[cfg(any(test, feature="test-utils"))]` 内存替身

use std::collections::HashMap;

use async_trait::async_trait;
use redis::AsyncCommands;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::common::error::AppError;

use super::{
    assign_medal, current_key, current_period_key, MeInfo, Period, RankingItem, RankingResult,
    RankingType,
};

// ─── RankingServicePort trait ─────────────────────────────────────────────────

/// HTTP handler 依赖的榜单服务抽象。
#[async_trait]
pub trait RankingServicePort: Send + Sync {
    /// 查询当前 type+period 的 Top N 榜单，并附带 viewer 的实时排名。
    ///
    /// - `ty`：榜单类型（charm/wealth）
    /// - `period`：周期（day/week）
    /// - `limit`：返回条数（1-100）
    /// - `viewer`：当前用户 UUID（用于查询自身排名）
    async fn top(
        &self,
        ty: RankingType,
        period: Period,
        limit: usize,
        viewer: Option<Uuid>,
    ) -> Result<RankingResult, AppError>;
}

// ─── RankingService ───────────────────────────────────────────────────────────

/// 榜单服务真实实现（PgPool + Redis）
pub struct RankingService {
    pool: PgPool,
    redis_client: redis::Client,
}

impl RankingService {
    /// 创建 RankingService。
    ///
    /// Redis 连接采用 lazy 模式（首次查询时建立），`new()` 不 panic。
    pub fn new(pool: PgPool, redis_url: String) -> Self {
        let redis_client = redis::Client::open(redis_url).unwrap_or_else(|e| {
            tracing::warn!("RankingService: failed to open redis client: {e}");
            redis::Client::open("redis://127.0.0.1:6379").expect("fallback redis client")
        });
        Self { pool, redis_client }
    }

    /// 按指定 Redis key 查询榜单（供集成测试直接传入测试 key）。
    ///
    /// 这是核心实现函数，`top()` 会计算出 key 后委托给此函数。
    pub async fn top_by_key(
        &self,
        key: &str,
        limit: usize,
        viewer: Option<Uuid>,
    ) -> Result<RankingResult, AppError> {
        let mut conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        // ① ZREVRANGE key 0 (limit-1) WITHSCORES
        let raw: Vec<(String, f64)> = redis::cmd("ZREVRANGE")
            .arg(key)
            .arg(0i64)
            .arg((limit as i64) - 1)
            .arg("WITHSCORES")
            .query_async(&mut conn)
            .await
            .unwrap_or_default();

        // ② 收集 user_id 列表
        let user_ids: Vec<Uuid> = raw
            .iter()
            .filter_map(|(s, _)| Uuid::parse_str(s).ok())
            .collect();

        // ③ 批量查询用户信息（nickname + avatar）
        let user_info: HashMap<Uuid, (String, Option<String>)> = if user_ids.is_empty() {
            HashMap::new()
        } else {
            self.batch_get_user_info(&user_ids).await?
        };

        // ④ 构建 items
        let items: Vec<RankingItem> = raw
            .iter()
            .enumerate()
            .map(|(idx, (member_str, score))| {
                let rank = (idx + 1) as u32;
                let uid = Uuid::parse_str(member_str).unwrap_or_else(|_| Uuid::nil());
                let (nickname, avatar) = user_info
                    .get(&uid)
                    .cloned()
                    .unwrap_or_else(|| (uid.to_string(), None));
                RankingItem {
                    rank,
                    user_id: uid,
                    nickname,
                    avatar,
                    score: *score as i64,
                    medal: assign_medal(rank),
                }
            })
            .collect();

        // ⑤ 查询 viewer 排名
        let me = if let Some(viewer_id) = viewer {
            let viewer_str = viewer_id.to_string();

            // ZREVRANK 返回 0-based rank；未在 ZSet 中时返回 nil
            let zrevrank: Option<u64> = conn.zrevrank(key, &viewer_str).await.unwrap_or(None);

            let zscore: Option<f64> = conn.zscore(key, &viewer_str).await.unwrap_or(None);

            MeInfo {
                rank: zrevrank.map(|r| (r + 1) as u32),
                score: zscore.map(|s| s as i64).unwrap_or(0),
            }
        } else {
            MeInfo {
                rank: None,
                score: 0,
            }
        };

        Ok(RankingResult {
            ty: "charm".to_string(),   // placeholder; top() 会覆盖
            period: "day".to_string(), // placeholder; top() 会覆盖
            period_key: String::new(), // placeholder; top() 会覆盖
            items,
            me,
        })
    }

    /// 批量查询用户 nickname + avatar
    async fn batch_get_user_info(
        &self,
        user_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, (String, Option<String>)>, AppError> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }

        // 使用 ANY($1) 批量查询（无需动态拼 IN 子句）
        let rows = sqlx::query(
            "SELECT id, nickname, avatar FROM users WHERE id = ANY($1) AND deleted_at IS NULL",
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut map = HashMap::with_capacity(rows.len());
        for row in rows {
            let id: Uuid = row.get("id");
            let nickname: String = row.get("nickname");
            let avatar: Option<String> = row.get("avatar");
            map.insert(id, (nickname, avatar));
        }
        Ok(map)
    }
}

#[async_trait]
impl RankingServicePort for RankingService {
    async fn top(
        &self,
        ty: RankingType,
        period: Period,
        limit: usize,
        viewer: Option<Uuid>,
    ) -> Result<RankingResult, AppError> {
        let key = current_key(ty, period);
        let mut result = self.top_by_key(&key, limit, viewer).await?;
        // 填入正确的 type / period / period_key
        result.ty = ty.as_key_segment().to_string();
        result.period = period.as_key_segment().to_string();
        result.period_key = current_period_key(period);
        Ok(result)
    }
}

// ─── FakeRankingService ───────────────────────────────────────────────────────

/// 测试替身：返回空榜单，用于不需要真实 Redis/PG 的 HTTP 参数校验测试。
#[cfg(any(test, feature = "test-utils"))]
pub struct FakeRankingService;

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl RankingServicePort for FakeRankingService {
    async fn top(
        &self,
        ty: RankingType,
        period: Period,
        _limit: usize,
        _viewer: Option<Uuid>,
    ) -> Result<RankingResult, AppError> {
        Ok(RankingResult {
            ty: ty.as_key_segment().to_string(),
            period: period.as_key_segment().to_string(),
            period_key: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            items: vec![],
            me: MeInfo {
                rank: None,
                score: 0,
            },
        })
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    // RS-01: FakeRankingService 返回空 items
    #[tokio::test]
    async fn fake_service_returns_empty() {
        let svc = FakeRankingService;
        let result = svc
            .top(RankingType::Charm, Period::Day, 50, None)
            .await
            .unwrap();
        assert!(result.items.is_empty());
        assert_eq!(result.ty, "charm");
        assert_eq!(result.period, "day");
    }

    // RS-02: FakeRankingService me.rank=null, me.score=0
    #[tokio::test]
    async fn fake_service_me_is_null() {
        let svc = FakeRankingService;
        let result = svc
            .top(RankingType::Wealth, Period::Week, 10, Some(Uuid::new_v4()))
            .await
            .unwrap();
        assert!(result.me.rank.is_none());
        assert_eq!(result.me.score, 0);
    }
}
