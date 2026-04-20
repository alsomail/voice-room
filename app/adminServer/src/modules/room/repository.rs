use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::error::AppError;

use super::dto::{AdminRoomDetailRow, AdminRoomFilter, AdminRoomListRow};

// ─── Trait ──────────────────────────────────────────────────────────────────

/// rooms + users 联表查询抽象，隔离真实 DB 与测试 Fake。
#[async_trait]
pub trait AdminRoomRepository: Send + Sync {
    /// 按过滤条件统计未软删除的房间总数。
    async fn count_rooms(&self, filter: &AdminRoomFilter) -> Result<i64, AppError>;

    /// 按过滤条件分页查询房间列表（joined users），结果按 created_at DESC 排序。
    async fn find_rooms(
        &self,
        filter: &AdminRoomFilter,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<AdminRoomListRow>, AppError>;

    /// 按 id 查询房间详情（不过滤 status，仅过滤 deleted_at IS NULL）。
    async fn find_room_by_id_any_status(
        &self,
        room_id: Uuid,
    ) -> Result<Option<AdminRoomDetailRow>, AppError>;

    /// 将房间状态设置为 "closed"（仅过滤软删除，不做状态前置校验）。
    /// 房间不存在时静默忽略（Ok(())），由调用方负责前置状态检查。
    async fn set_room_closed(&self, room_id: Uuid) -> Result<(), AppError>;
}

// ─── Postgres 实现 ───────────────────────────────────────────────────────────

/// 基于 SQLx + PostgreSQL 的 AdminRoomRepository 生产实现。
pub struct PgAdminRoomRepository {
    pool: PgPool,
}

impl PgAdminRoomRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminRoomRepository for PgAdminRoomRepository {
    async fn count_rooms(&self, filter: &AdminRoomFilter) -> Result<i64, AppError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) \
             FROM rooms r \
             JOIN users u ON r.owner_id = u.id \
             WHERE r.deleted_at IS NULL \
               AND ($1::text IS NULL OR r.status = $1) \
               AND ($2::text IS NULL OR LOWER(r.title) LIKE '%' || LOWER($2) || '%')",
        )
        .bind(filter.status.as_deref())
        .bind(filter.keyword.as_deref())
        .fetch_one(&self.pool)
        .await?;
        Ok(count.0)
    }

    async fn find_rooms(
        &self,
        filter: &AdminRoomFilter,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<AdminRoomListRow>, AppError> {
        let rows = sqlx::query_as::<_, AdminRoomListRow>(
            "SELECT r.id, r.title, r.status, r.room_type, r.member_count, r.max_members, \
                    r.owner_id, u.nickname AS owner_nickname, u.avatar AS owner_avatar, \
                    r.created_at \
             FROM rooms r \
             JOIN users u ON r.owner_id = u.id \
             WHERE r.deleted_at IS NULL \
               AND ($1::text IS NULL OR r.status = $1) \
               AND ($2::text IS NULL OR LOWER(r.title) LIKE '%' || LOWER($2) || '%') \
             ORDER BY r.created_at DESC \
             LIMIT $3 OFFSET $4",
        )
        .bind(filter.status.as_deref())
        .bind(filter.keyword.as_deref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn find_room_by_id_any_status(
        &self,
        room_id: Uuid,
    ) -> Result<Option<AdminRoomDetailRow>, AppError> {
        let row = sqlx::query_as::<_, AdminRoomDetailRow>(
            "SELECT r.id, r.title, r.status, r.room_type, r.member_count, r.max_members, \
                    r.owner_id, u.nickname AS owner_nickname, u.avatar AS owner_avatar, \
                    r.created_at, r.updated_at \
             FROM rooms r \
             JOIN users u ON r.owner_id = u.id \
             WHERE r.id = $1 \
               AND r.deleted_at IS NULL",
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn set_room_closed(&self, room_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE rooms \
             SET status = 'closed', updated_at = NOW() \
             WHERE id = $1 \
               AND deleted_at IS NULL",
        )
        .bind(room_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// ─── Fake 实现（内存，用于单元 / 集成测试）────────────────────────────────────

/// 内部存储条目（含软删除标记，供测试用）。
struct FakeRoomEntry {
    row: AdminRoomListRow,
    deleted_at: Option<DateTime<Utc>>,
}

/// 内部存储条目（详情，含软删除标记）。
struct FakeDetailEntry {
    row: AdminRoomDetailRow,
    deleted_at: Option<DateTime<Utc>>,
}

/// 内存版 AdminRoomRepository，用于单元 / 集成测试。
#[derive(Default)]
pub struct FakeAdminRoomRepository {
    entries: Mutex<Vec<FakeRoomEntry>>,
    detail_entries: Mutex<HashMap<Uuid, FakeDetailEntry>>,
}

impl FakeAdminRoomRepository {
    /// 预置一条未删除的房间列表行（供 list 测试）。
    pub fn seed(&self, row: AdminRoomListRow) {
        self.entries
            .lock()
            .unwrap()
            .push(FakeRoomEntry { row, deleted_at: None });
    }

    /// 预置一条已软删除的房间列表行（用于 R-07 测试）。
    pub fn seed_deleted(&self, row: AdminRoomListRow) {
        self.entries.lock().unwrap().push(FakeRoomEntry {
            row,
            deleted_at: Some(Utc::now()),
        });
    }

    /// 预置一条未删除的房间详情行（供 detail 测试）。
    pub fn seed_detail(&self, row: AdminRoomDetailRow) {
        self.detail_entries
            .lock()
            .unwrap()
            .insert(row.id, FakeDetailEntry { row, deleted_at: None });
    }

    /// 预置一条已软删除的房间详情行（用于 DR-04 / D-04 测试）。
    pub fn seed_detail_deleted(&self, row: AdminRoomDetailRow) {
        self.detail_entries.lock().unwrap().insert(
            row.id,
            FakeDetailEntry {
                row,
                deleted_at: Some(Utc::now()),
            },
        );
    }
}

#[async_trait]
impl AdminRoomRepository for FakeAdminRoomRepository {
    async fn count_rooms(&self, filter: &AdminRoomFilter) -> Result<i64, AppError> {
        let guard = self.entries.lock().unwrap();
        let count = guard
            .iter()
            .filter(|e| e.deleted_at.is_none())
            .filter(|e| matches_filter(&e.row, filter))
            .count();
        Ok(count as i64)
    }

    async fn find_rooms(
        &self,
        filter: &AdminRoomFilter,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<AdminRoomListRow>, AppError> {
        let guard = self.entries.lock().unwrap();
        let mut results: Vec<AdminRoomListRow> = guard
            .iter()
            .filter(|e| e.deleted_at.is_none())
            .filter(|e| matches_filter(&e.row, filter))
            .map(|e| e.row.clone())
            .collect();

        // 按 created_at DESC 排序
        results.sort_by_key(|r| std::cmp::Reverse(r.created_at));

        // 分页切片
        let start = (offset as usize).min(results.len());
        let end = ((offset + limit) as usize).min(results.len());
        Ok(results[start..end].to_vec())
    }

    async fn find_room_by_id_any_status(
        &self,
        room_id: Uuid,
    ) -> Result<Option<AdminRoomDetailRow>, AppError> {
        let guard = self.detail_entries.lock().unwrap();
        let result = guard
            .get(&room_id)
            .filter(|e| e.deleted_at.is_none())
            .map(|e| e.row.clone());
        Ok(result)
    }

    async fn set_room_closed(&self, room_id: Uuid) -> Result<(), AppError> {
        let mut guard = self.detail_entries.lock().unwrap();
        if let Some(entry) = guard.get_mut(&room_id) {
            // 仅对未软删除的条目生效
            if entry.deleted_at.is_none() {
                entry.row.status = "closed".to_string();
            }
        }
        // 找不到时静默忽略（幂等设计，由 Service 层负责前置状态校验）
        Ok(())
    }
}

/// 辅助：检查行是否满足过滤条件。
fn matches_filter(row: &AdminRoomListRow, filter: &AdminRoomFilter) -> bool {
    if let Some(status) = &filter.status {
        if &row.status != status {
            return false;
        }
    }
    if let Some(keyword) = &filter.keyword {
        if !row.title.to_lowercase().contains(&keyword.to_lowercase()) {
            return false;
        }
    }
    true
}

// ─── Repository 单元测试（TDD T-10004 验收用例 R-01~R-07）────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use uuid::Uuid;

    // ── 测试辅助 ──────────────────────────────────────────────────────────────

    fn make_row(title: &str, status: &str, created_at_offset_secs: i64) -> AdminRoomListRow {
        AdminRoomListRow {
            id: Uuid::new_v4(),
            title: title.to_string(),
            status: status.to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            max_members: 50,
            owner_id: Uuid::new_v4(),
            owner_nickname: "TestOwner".to_string(),
            owner_avatar: Some("https://avatar.example.com/1.png".to_string()),
            created_at: Utc::now() + Duration::seconds(created_at_offset_secs),
        }
    }

    fn empty_filter() -> AdminRoomFilter {
        AdminRoomFilter::default()
    }

    fn status_filter(s: &str) -> AdminRoomFilter {
        AdminRoomFilter {
            status: Some(s.to_string()),
            keyword: None,
        }
    }

    fn keyword_filter(k: &str) -> AdminRoomFilter {
        AdminRoomFilter {
            status: None,
            keyword: Some(k.to_string()),
        }
    }

    // ── R-01: count_rooms(all) = 所有未软删除房间数 ──────────────────────────
    #[tokio::test]
    async fn r01_count_rooms_all_returns_non_deleted_total() {
        let repo = FakeAdminRoomRepository::default();
        repo.seed(make_row("Room A", "active", 0));
        repo.seed(make_row("Room B", "closed", -10));
        repo.seed(make_row("Room C", "active", -20));

        let count = repo.count_rooms(&empty_filter()).await.unwrap();
        assert_eq!(count, 3, "R-01: 3 个未删除房间，count 应为 3");
    }

    // ── R-02: count_rooms(active) = 仅 active 数 ─────────────────────────────
    #[tokio::test]
    async fn r02_count_rooms_active_only() {
        let repo = FakeAdminRoomRepository::default();
        repo.seed(make_row("Active 1", "active", 0));
        repo.seed(make_row("Active 2", "active", -10));
        repo.seed(make_row("Closed 1", "closed", -20));

        let count = repo.count_rooms(&status_filter("active")).await.unwrap();
        assert_eq!(count, 2, "R-02: 仅统计 active 状态");
    }

    // ── R-03: count_rooms(keyword="test") = title 含 test 的数量 ─────────────
    #[tokio::test]
    async fn r03_count_rooms_keyword_test_matches_title() {
        let repo = FakeAdminRoomRepository::default();
        repo.seed(make_row("test room alpha", "active", 0));
        repo.seed(make_row("Test Room Beta", "closed", -10)); // 大写 Test
        repo.seed(make_row("unrelated room", "active", -20));

        let count = repo.count_rooms(&keyword_filter("test")).await.unwrap();
        assert_eq!(count, 2, "R-03: 含 'test' 的 title 有 2 个（大小写不敏感）");
    }

    // ── R-04: find_rooms 结果按 created_at DESC 排序 ──────────────────────────
    #[tokio::test]
    async fn r04_find_rooms_sorted_by_created_at_desc() {
        let repo = FakeAdminRoomRepository::default();
        repo.seed(make_row("Old Room", "active", -100)); // 最早
        repo.seed(make_row("Newest Room", "active", 0)); // 最新
        repo.seed(make_row("Mid Room", "active", -50));  // 中间

        let rows = repo.find_rooms(&empty_filter(), 0, 10).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].title, "Newest Room", "R-04: 第 1 行应为最新房间");
        assert_eq!(rows[1].title, "Mid Room", "R-04: 第 2 行应为中间房间");
        assert_eq!(rows[2].title, "Old Room", "R-04: 第 3 行应为最旧房间");
    }

    // ── R-05: find_rooms 分页偏移正确 ────────────────────────────────────────
    #[tokio::test]
    async fn r05_find_rooms_pagination_offset_correct() {
        let repo = FakeAdminRoomRepository::default();
        // 按时间从新到旧：room_0 > room_1 > ... > room_4
        for i in 0..5i64 {
            repo.seed(make_row(&format!("room_{i}"), "active", -i * 10));
        }

        // 第一页（offset=0, limit=2）
        let page1 = repo.find_rooms(&empty_filter(), 0, 2).await.unwrap();
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].title, "room_0");
        assert_eq!(page1[1].title, "room_1");

        // 第二页（offset=2, limit=2）
        let page2 = repo.find_rooms(&empty_filter(), 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);
        assert_eq!(page2[0].title, "room_2");
        assert_eq!(page2[1].title, "room_3");
    }

    // ── R-06: keyword 过滤大小写不敏感 ───────────────────────────────────────
    #[tokio::test]
    async fn r06_keyword_filter_case_insensitive() {
        let repo = FakeAdminRoomRepository::default();
        repo.seed(make_row("MUSIC Room", "active", 0));
        repo.seed(make_row("music lounge", "active", -10));
        repo.seed(make_row("Gaming Zone", "active", -20));

        // 用小写 "music" 查询应匹配 "MUSIC Room" 和 "music lounge"
        let count = repo.count_rooms(&keyword_filter("music")).await.unwrap();
        assert_eq!(count, 2, "R-06: 大小写不敏感，'music' 应匹配 2 条");

        let rows = repo.find_rooms(&keyword_filter("music"), 0, 10).await.unwrap();
        assert_eq!(rows.len(), 2, "R-06: find_rooms 也应返回 2 条");
    }

    // ── R-07: 软删除房间不返回 ────────────────────────────────────────────────
    #[tokio::test]
    async fn r07_soft_deleted_rooms_not_returned() {
        let repo = FakeAdminRoomRepository::default();
        repo.seed(make_row("Visible Room", "active", 0));
        repo.seed_deleted(make_row("Deleted Room", "active", -10)); // 软删除

        let count = repo.count_rooms(&empty_filter()).await.unwrap();
        assert_eq!(count, 1, "R-07: count_rooms 不应统计已软删除的房间");

        let rows = repo.find_rooms(&empty_filter(), 0, 10).await.unwrap();
        assert_eq!(rows.len(), 1, "R-07: find_rooms 不应返回已软删除的房间");
        assert_eq!(rows[0].title, "Visible Room");
    }

    // ══════════════════════════════════════════════════════════════════════════
    // T-10005 新增 Repository 单元测试（DR-01~DR-04）
    // ══════════════════════════════════════════════════════════════════════════

    fn make_detail_row(title: &str, status: &str) -> super::super::dto::AdminRoomDetailRow {
        use super::super::dto::AdminRoomDetailRow;
        AdminRoomDetailRow {
            id: Uuid::new_v4(),
            title: title.to_string(),
            status: status.to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            max_members: 50,
            owner_id: Uuid::new_v4(),
            owner_nickname: "DetailOwner".to_string(),
            owner_avatar: Some("https://avatar.example.com/1.png".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ── DR-01: find_room_by_id_any_status 返回已存在的 active 房间 ────────────
    #[tokio::test]
    async fn dr01_find_room_by_id_returns_active_room() {
        let repo = FakeAdminRoomRepository::default();
        let row = make_detail_row("Active Room", "active");
        let id = row.id;
        repo.seed_detail(row);

        let result = repo.find_room_by_id_any_status(id).await.unwrap();
        assert!(result.is_some(), "DR-01: active 房间应能查到");
        assert_eq!(result.unwrap().title, "Active Room");
    }

    // ── DR-02: find_room_by_id_any_status 返回 closed 房间（不过滤状态）────────
    #[tokio::test]
    async fn dr02_find_room_by_id_returns_closed_room() {
        let repo = FakeAdminRoomRepository::default();
        let row = make_detail_row("Closed Room", "closed");
        let id = row.id;
        repo.seed_detail(row);

        let result = repo.find_room_by_id_any_status(id).await.unwrap();
        assert!(result.is_some(), "DR-02: closed 房间也应能查到（不过滤 status）");
        assert_eq!(result.unwrap().status, "closed");
    }

    // ── DR-03: find_room_by_id_any_status 房间不存在时返回 None ─────────────────
    #[tokio::test]
    async fn dr03_find_room_by_id_returns_none_when_not_found() {
        let repo = FakeAdminRoomRepository::default();
        let result = repo
            .find_room_by_id_any_status(Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none(), "DR-03: 不存在的 id 应返回 None");
    }

    // ── DR-04: find_room_by_id_any_status 软删除房间返回 None ──────────────────
    #[tokio::test]
    async fn dr04_find_room_by_id_returns_none_for_deleted_room() {
        let repo = FakeAdminRoomRepository::default();
        let row = make_detail_row("Deleted Room", "active");
        let id = row.id;
        repo.seed_detail_deleted(row);

        let result = repo.find_room_by_id_any_status(id).await.unwrap();
        assert!(result.is_none(), "DR-04: 已软删除的房间应返回 None");
    }

    // ══════════════════════════════════════════════════════════════════════════
    // T-10006 新增 Repository 单元测试（FCR-01~FCR-03）
    // ══════════════════════════════════════════════════════════════════════════

    // ── FCR-01: set_room_closed(active_id) → Ok(())，再 find → status=="closed" ──
    #[tokio::test]
    async fn fcr01_set_room_closed_active_room_becomes_closed() {
        let repo = FakeAdminRoomRepository::default();
        let row = make_detail_row("Active Room", "active");
        let id = row.id;
        repo.seed_detail(row);

        let result = repo.set_room_closed(id).await;
        assert!(result.is_ok(), "FCR-01: set_room_closed 应返回 Ok(())");

        let found = repo.find_room_by_id_any_status(id).await.unwrap();
        assert!(found.is_some(), "FCR-01: 关闭后仍能查到房间");
        assert_eq!(
            found.unwrap().status,
            "closed",
            "FCR-01: 关闭后 status 应变为 closed"
        );
    }

    // ── FCR-02: set_room_closed(closed_id) → Ok(())（幂等）─────────────────────
    #[tokio::test]
    async fn fcr02_set_room_closed_already_closed_is_idempotent() {
        let repo = FakeAdminRoomRepository::default();
        let row = make_detail_row("Closed Room", "closed");
        let id = row.id;
        repo.seed_detail(row);

        // 对已关闭房间再次关闭，应静默 Ok(())
        let result = repo.set_room_closed(id).await;
        assert!(result.is_ok(), "FCR-02: 对已 closed 房间调用 set_room_closed 应返回 Ok(())");

        let found = repo.find_room_by_id_any_status(id).await.unwrap();
        assert_eq!(
            found.unwrap().status,
            "closed",
            "FCR-02: 幂等操作后 status 仍为 closed"
        );
    }

    // ── FCR-03: set_room_closed(nonexistent_id) → Ok(())（静默忽略）──────────────
    #[tokio::test]
    async fn fcr03_set_room_closed_nonexistent_id_is_silent_ok() {
        let repo = FakeAdminRoomRepository::default();
        let nonexistent_id = Uuid::new_v4();

        let result = repo.set_room_closed(nonexistent_id).await;
        assert!(
            result.is_ok(),
            "FCR-03: 对不存在的 room_id 调用 set_room_closed 应返回 Ok(())"
        );
    }
}
