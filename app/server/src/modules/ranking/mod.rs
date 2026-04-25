//! Ranking 模块 — T-00021 魅力/财富榜单 API
//!
//! ## 功能概述
//! - `GET /api/v1/ranking?type=charm|wealth&period=day|week&limit=50`（需 JWT）
//! - 从 Redis ZSet 读取 Top N（ZREVRANGE + ZSCORE）
//! - 批量查询 PG 补充 nickname/avatar 用户信息
//! - 返回当前用户排名（未入榜时 rank=null）
//! - Top3 附带 gold/silver/bronze medal 字段
//!
//! ## 模块结构
//! - `service`  — `RankingServicePort` trait + `RankingService` 实现 + `FakeRankingService` 测试替身
//! - `handler`  — HTTP handler `get_ranking`
//! - `scheduler`— 每日/每周归档任务 + 补偿执行（幂等）
//! - `routes`   — 路由注册

pub mod handler;
pub mod scheduler;
pub mod service;

pub use routes::ranking_routes;
#[cfg(any(test, feature = "test-utils"))]
pub use service::FakeRankingService;
pub use service::RankingServicePort;

mod routes {
    use super::handler::get_ranking;
    use crate::bootstrap::AppState;
    use axum::{routing::get, Router};

    pub fn ranking_routes() -> Router<AppState> {
        Router::new().route("/api/v1/ranking", get(get_ranking))
    }
}

// ─── 共享数据类型 ──────────────────────────────────────────────────────────────

/// 榜单类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RankingType {
    Charm,
    Wealth,
}

impl RankingType {
    /// 转为 Redis key 片段
    pub fn as_key_segment(&self) -> &'static str {
        match self {
            RankingType::Charm => "charm",
            RankingType::Wealth => "wealth",
        }
    }
}

/// 榜单周期
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Day,
    Week,
}

impl Period {
    pub fn as_key_segment(&self) -> &'static str {
        match self {
            Period::Day => "day",
            Period::Week => "week",
        }
    }
}

/// 单个榜单条目
#[derive(Debug, Clone, serde::Serialize)]
pub struct RankingItem {
    pub rank: u32,
    pub user_id: uuid::Uuid,
    pub nickname: String,
    pub avatar: Option<String>,
    pub score: i64,
    /// Top3 金牌标识："gold" | "silver" | "bronze" | null
    pub medal: Option<String>,
}

/// 当前用户的榜单位置
#[derive(Debug, Clone, serde::Serialize)]
pub struct MeInfo {
    /// 未入榜时为 null
    pub rank: Option<u32>,
    /// 未入榜时为 0
    pub score: i64,
}

/// `top()` / `top_by_key()` 返回值（用于 API 响应 + 单元测试）
#[derive(Debug, Clone, serde::Serialize)]
pub struct RankingResult {
    #[serde(rename = "type")]
    pub ty: String,
    pub period: String,
    /// Riyadh 本地日期标识（YYYY-MM-DD 或 YYYY-WW）
    pub period_key: String,
    pub items: Vec<RankingItem>,
    pub me: MeInfo,
}

use crate::common::time::riyadh;

/// 构造 Redis 日榜 key（Riyadh 日期）
pub fn day_key(ty: RankingType) -> String {
    format!(
        "ranking:{}:day:{}",
        ty.as_key_segment(),
        riyadh::today_riyadh_str()
    )
}

/// 构造 Redis 周榜 key（Riyadh 年+周）
pub fn week_key(ty: RankingType) -> String {
    format!(
        "ranking:{}:week:{}",
        ty.as_key_segment(),
        riyadh::week_riyadh_str()
    )
}

/// 根据 type + period 计算当前 Redis key
pub fn current_key(ty: RankingType, period: Period) -> String {
    match period {
        Period::Day => day_key(ty),
        Period::Week => week_key(ty),
    }
}

/// 当前 period_key（用于 API 响应，告知客户端当前所属周期标识）
pub fn current_period_key(period: Period) -> String {
    match period {
        Period::Day => riyadh::today_riyadh_str(),
        Period::Week => riyadh::week_riyadh_str(),
    }
}

/// 为 rank（1-based）分配 medal
pub fn assign_medal(rank: u32) -> Option<String> {
    match rank {
        1 => Some("gold".to_string()),
        2 => Some("silver".to_string()),
        3 => Some("bronze".to_string()),
        _ => None,
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_key_format() {
        let k = day_key(RankingType::Charm);
        assert!(k.starts_with("ranking:charm:day:"), "charm day key prefix");
        let date = k.strip_prefix("ranking:charm:day:").unwrap();
        assert_eq!(date.len(), 10, "date part should be YYYY-MM-DD");
    }

    #[test]
    fn week_key_format() {
        let k = week_key(RankingType::Wealth);
        assert!(
            k.starts_with("ranking:wealth:week:"),
            "wealth week key prefix"
        );
    }

    #[test]
    fn current_key_dispatch() {
        let day = current_key(RankingType::Charm, Period::Day);
        assert!(day.contains(":day:"), "day period key should contain :day:");
        let week = current_key(RankingType::Wealth, Period::Week);
        assert!(
            week.contains(":week:"),
            "week period key should contain :week:"
        );
    }

    #[test]
    fn medal_assignment() {
        assert_eq!(assign_medal(1).as_deref(), Some("gold"));
        assert_eq!(assign_medal(2).as_deref(), Some("silver"));
        assert_eq!(assign_medal(3).as_deref(), Some("bronze"));
        assert_eq!(assign_medal(4), None);
        assert_eq!(assign_medal(100), None);
    }

    #[test]
    fn ranking_type_key_segment() {
        assert_eq!(RankingType::Charm.as_key_segment(), "charm");
        assert_eq!(RankingType::Wealth.as_key_segment(), "wealth");
    }

    #[test]
    fn period_key_segment() {
        assert_eq!(Period::Day.as_key_segment(), "day");
        assert_eq!(Period::Week.as_key_segment(), "week");
    }

    // 缺陷 #3 — current_period_key 必须基于 Riyadh 时区
    #[test]
    fn current_period_key_uses_riyadh() {
        let day_pk = current_period_key(Period::Day);
        let week_pk = current_period_key(Period::Week);
        assert_eq!(day_pk, crate::common::time::riyadh::today_riyadh_str());
        assert_eq!(week_pk, crate::common::time::riyadh::week_riyadh_str());
    }
}
