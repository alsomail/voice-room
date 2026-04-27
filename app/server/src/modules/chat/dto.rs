//! 聊天历史 REST DTO（T-00043）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::repository::ChatHistoryRow;

/// 查询参数：`?limit=50&offset=0`
#[derive(Debug, Deserialize)]
pub struct MessagesQuery {
    /// 默认 50；上限 100；> 100 时静默截断为 100
    #[serde(default)]
    pub limit: Option<u32>,
    /// 默认 0
    #[serde(default)]
    pub offset: Option<u32>,
}

/// 默认 limit = 50
pub const DEFAULT_LIMIT: u32 = 50;
/// 上限 limit = 100
pub const MAX_LIMIT: u32 = 100;
/// offset 软上限：超过此值会被截断（防止超大 OFFSET 引发 PG O(N) 跳表扫描）。
/// Round 1 review Should-6：偏移 > 100_000 时强制截断，提示前端切换到游标分页。
pub const MAX_OFFSET: u32 = 100_000;

/// 单条历史消息（REST 序列化形态）
#[derive(Debug, Clone, Serialize)]
pub struct ChatMessageItem {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl From<ChatHistoryRow> for ChatMessageItem {
    fn from(r: ChatHistoryRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            nickname: r.nickname,
            avatar_url: r.avatar_url,
            content: r.content,
            created_at: r.created_at,
        }
    }
}

/// 历史消息分页响应
#[derive(Debug, Clone, Serialize)]
pub struct ChatMessagesResponse {
    pub items: Vec<ChatMessageItem>,
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
}

/// 规范化分页参数。
///
/// - `limit = None` → `DEFAULT_LIMIT`
/// - `limit = 0` → `DEFAULT_LIMIT`（防呆）
/// - `limit > MAX_LIMIT` → `MAX_LIMIT`（B-1）
/// - `offset = None` → 0
/// - `offset > MAX_OFFSET` → `MAX_OFFSET`（Round 1 Should-6 软上限）
pub fn normalize_pagination(q: &MessagesQuery) -> (u32, u32) {
    let mut limit = q.limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 {
        limit = DEFAULT_LIMIT;
    }
    if limit > MAX_LIMIT {
        limit = MAX_LIMIT;
    }
    let mut offset = q.offset.unwrap_or(0);
    if offset > MAX_OFFSET {
        offset = MAX_OFFSET;
    }
    (limit, offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B-1 单测：normalize_pagination 默认值与上限
    #[test]
    fn normalize_pagination_defaults_and_caps() {
        let q = MessagesQuery {
            limit: None,
            offset: None,
        };
        assert_eq!(normalize_pagination(&q), (DEFAULT_LIMIT, 0));

        let q = MessagesQuery {
            limit: Some(0),
            offset: None,
        };
        assert_eq!(normalize_pagination(&q), (DEFAULT_LIMIT, 0));

        // B-1 上限截断
        let q = MessagesQuery {
            limit: Some(999),
            offset: Some(0),
        };
        assert_eq!(normalize_pagination(&q), (MAX_LIMIT, 0));

        let q = MessagesQuery {
            limit: Some(100),
            offset: Some(50),
        };
        assert_eq!(normalize_pagination(&q), (100, 50));

        let q = MessagesQuery {
            limit: Some(20),
            offset: Some(40),
        };
        assert_eq!(normalize_pagination(&q), (20, 40));
    }

    /// Round 1 Should-6：offset 软上限测试
    #[test]
    fn normalize_pagination_offset_soft_cap() {
        // 上限内不变
        let q = MessagesQuery {
            limit: Some(50),
            offset: Some(MAX_OFFSET),
        };
        assert_eq!(normalize_pagination(&q), (50, MAX_OFFSET));

        // 超出上限被截断
        let q = MessagesQuery {
            limit: Some(50),
            offset: Some(MAX_OFFSET + 1),
        };
        assert_eq!(normalize_pagination(&q), (50, MAX_OFFSET));

        let q = MessagesQuery {
            limit: Some(50),
            offset: Some(u32::MAX),
        };
        assert_eq!(normalize_pagination(&q), (50, MAX_OFFSET));
    }
}
