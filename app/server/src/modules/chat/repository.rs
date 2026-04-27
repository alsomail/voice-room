//! ChatRepository — 聊天消息 CRUD 抽象（T-00043）
//!
//! - 生产实现：`RealChatRepository`（基于 sqlx PgPool）
//! - 测试实现：`FakeChatRepository`（内存 Mutex<Vec>，仅在 `test` / `test-utils` 下编译）
//!
//! 提供：
//! - `insert_message(room_id, user_id, content) -> Uuid`
//! - `list_messages(room_id, limit, offset) -> (rows, total)`，LEFT JOIN users
//! - `count_messages(room_id) -> i64`（B-3 并发断言）

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::common::error::AppError;

/// 聊天历史一行（含 user 关联字段，由 LEFT JOIN 提供，user 删除后字段为 None）
#[derive(Debug, Clone, Serialize)]
pub struct ChatHistoryRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// 聊天消息存储抽象
#[async_trait]
pub trait ChatRepository: Send + Sync {
    /// 插入一条聊天消息，返回 DB 生成的 `id`（UUID v4）。
    ///
    /// 失败：DB 错误（外键、CHECK 等）→ `AppError::DatabaseError`。
    async fn insert_message(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        content: &str,
    ) -> Result<Uuid, AppError>;

    /// 分页查询房间历史消息，按 `created_at DESC`。
    ///
    /// 返回 `(items, total)`：
    /// - `items.len() <= limit`
    /// - `total` 为该房间消息总数（不受 limit/offset 影响）
    async fn list_messages(
        &self,
        room_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<ChatHistoryRow>, i64), AppError>;

    /// 单纯计数（B-3 并发写测试使用）
    async fn count_messages(&self, room_id: Uuid) -> Result<i64, AppError>;
}

/// Blanket：`Arc<T: ChatRepository>` 透传
#[async_trait]
impl<T: ChatRepository + ?Sized> ChatRepository for Arc<T> {
    async fn insert_message(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        content: &str,
    ) -> Result<Uuid, AppError> {
        (**self).insert_message(room_id, user_id, content).await
    }
    async fn list_messages(
        &self,
        room_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<ChatHistoryRow>, i64), AppError> {
        (**self).list_messages(room_id, limit, offset).await
    }
    async fn count_messages(&self, room_id: Uuid) -> Result<i64, AppError> {
        (**self).count_messages(room_id).await
    }
}

// ─── RealChatRepository（sqlx 实现） ─────────────────────────────────────────

/// 生产实现：基于 sqlx PgPool。
pub struct RealChatRepository {
    pool: sqlx::PgPool,
}

impl RealChatRepository {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChatRepository for RealChatRepository {
    async fn insert_message(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        content: &str,
    ) -> Result<Uuid, AppError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO chat_messages (room_id, user_id, content) \
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(room_id)
        .bind(user_id)
        .bind(content)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(id)
    }

    async fn list_messages(
        &self,
        room_id: Uuid,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<ChatHistoryRow>, i64), AppError> {
        // Round 1 Should-5：合并 SELECT + COUNT(*) → 单条 SQL，COUNT(*) OVER() 在 LIMIT/OFFSET 之前求值。
        // 排序键 `created_at DESC, id DESC` —— Round 1 Should-2：id 仅作为 deterministic tiebreak
        // 避免同毫秒并发写返回顺序抖动；不主张其语义反映插入序（UUID v4 随机）。
        type Row = (
            Uuid,
            Option<Uuid>,
            Option<String>,
            Option<String>,
            String,
            DateTime<Utc>,
            i64,
        );
        let rows: Vec<Row> = sqlx::query_as(
            r#"SELECT cm.id,
                      cm.user_id,
                      u.nickname,
                      u.avatar_url,
                      cm.content,
                      cm.created_at,
                      COUNT(*) OVER() AS total_count
               FROM chat_messages cm
               LEFT JOIN users u ON u.id = cm.user_id
               WHERE cm.room_id = $1
               ORDER BY cm.created_at DESC, cm.id DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(room_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 当 LIMIT/OFFSET 把整个窗口剪掉（如 B-2: offset 超过 total），
        // COUNT(*) OVER() 也无法返回值 → 退回单条 COUNT 兜底。
        // 仅 offset > 0 且 rows 为空时才补一次（避免 happy-path 多一次往返）。
        let total: i64 = if let Some(first) = rows.first() {
            first.6
        } else if offset == 0 {
            0
        } else {
            sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM chat_messages WHERE room_id = $1")
                .bind(room_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?
        };

        let items = rows
            .into_iter()
            .map(
                |(id, user_id, nickname, avatar_url, content, created_at, _total)| {
                    ChatHistoryRow {
                        id,
                        user_id,
                        nickname,
                        avatar_url,
                        content,
                        created_at,
                    }
                },
            )
            .collect();

        Ok((items, total))
    }

    async fn count_messages(&self, room_id: Uuid) -> Result<i64, AppError> {
        sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM chat_messages WHERE room_id = $1")
            .bind(room_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))
    }
}

// ─── FakeChatRepository（内存实现，仅 test/test-utils 编译） ─────────────────

#[cfg(any(test, feature = "test-utils"))]
mod fake_impl {
    use super::*;
    use std::sync::RwLock;

    /// 内存条目（与 DB schema 对齐）
    #[derive(Clone)]
    struct Row {
        id: Uuid,
        room_id: Uuid,
        user_id: Uuid,
        content: String,
        created_at: DateTime<Utc>,
        seq: u64, // 单调递增序列；Round 1 Should-2 — Fake 路径以此模拟 (created_at, id) 的 deterministic tiebreak
    }

    /// 测试用聊天 Repo，内存 RwLock<Vec>。
    /// Round 1 Should-3：写入 / 读取使用 RwLock，配合 `tokio::test(flavor = "multi_thread")`
    /// 至少能在多线程 runtime 上验证"无 panic / 无序号丢失"。真实并发 DB 写入由
    /// `b3_concurrent_db_inserts`（`#[ignore]`）覆盖。
    #[derive(Default)]
    pub struct FakeChatRepository {
        rows: RwLock<Vec<Row>>,
        users: RwLock<std::collections::HashMap<Uuid, (String, Option<String>)>>,
        seq: std::sync::atomic::AtomicU64,
    }

    impl FakeChatRepository {
        pub fn new() -> Self {
            Self::default()
        }

        /// 注入用户档案（用于 nickname/avatar_url JOIN）
        pub fn seed_user(&self, user_id: Uuid, nickname: &str, avatar_url: Option<&str>) {
            self.users.write().unwrap().insert(
                user_id,
                (nickname.to_string(), avatar_url.map(|s| s.to_string())),
            );
        }
    }

    #[async_trait]
    impl ChatRepository for FakeChatRepository {
        async fn insert_message(
            &self,
            room_id: Uuid,
            user_id: Uuid,
            content: &str,
        ) -> Result<Uuid, AppError> {
            let len = content.chars().count();
            if len == 0 || len > 500 {
                return Err(AppError::DatabaseError(format!(
                    "chat_messages content length out of range: {len}"
                )));
            }
            let id = Uuid::new_v4();
            let seq = self
                .seq
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.rows.write().unwrap().push(Row {
                id,
                room_id,
                user_id,
                content: content.to_string(),
                created_at: Utc::now(),
                seq,
            });
            Ok(id)
        }

        async fn list_messages(
            &self,
            room_id: Uuid,
            limit: u32,
            offset: u32,
        ) -> Result<(Vec<ChatHistoryRow>, i64), AppError> {
            let rows = self.rows.read().unwrap();
            let users = self.users.read().unwrap();
            let mut filtered: Vec<&Row> = rows.iter().filter(|r| r.room_id == room_id).collect();
            // DESC by seq（等价 created_at DESC, id DESC 的 deterministic 投影）
            filtered.sort_by(|a, b| b.seq.cmp(&a.seq));
            let total = filtered.len() as i64;
            let items: Vec<ChatHistoryRow> = filtered
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .map(|r| {
                    let (nickname, avatar_url) = users
                        .get(&r.user_id)
                        .map(|(n, a)| (Some(n.clone()), a.clone()))
                        .unwrap_or((None, None));
                    ChatHistoryRow {
                        id: r.id,
                        user_id: Some(r.user_id),
                        nickname,
                        avatar_url,
                        content: r.content.clone(),
                        created_at: r.created_at,
                    }
                })
                .collect();
            Ok((items, total))
        }

        async fn count_messages(&self, room_id: Uuid) -> Result<i64, AppError> {
            Ok(self
                .rows
                .read()
                .unwrap()
                .iter()
                .filter(|r| r.room_id == room_id)
                .count() as i64)
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub use fake_impl::FakeChatRepository;
