use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::error::AppError;

use super::dto::{AdminUserFilter, AdminUserListRow};

// ─── Trait ──────────────────────────────────────────────────────────────────

/// users 表查询抽象，隔离真实 DB 与测试 Fake。
#[async_trait]
pub trait AdminUserRepository: Send + Sync {
    /// 按过滤条件统计未软删除的用户总数。
    async fn count_users(&self, filter: &AdminUserFilter) -> Result<i64, AppError>;

    /// 按过滤条件分页查询用户列表，结果按 created_at DESC 排序。
    async fn find_users(
        &self,
        filter: &AdminUserFilter,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<AdminUserListRow>, AppError>;

    /// 按 user_id 精确查询单个未软删除用户，不存在或已软删除返回 None。
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<AdminUserListRow>, AppError>;

    /// 更新用户封禁状态。
    /// 返回 true 表示找到并更新成功，false 表示用户不存在（0 affected rows）。
    async fn update_ban_status(&self, id: Uuid, is_banned: bool) -> Result<bool, AppError>;
}

// ─── Postgres 实现 ───────────────────────────────────────────────────────────

/// 基于 SQLx + PostgreSQL 的 AdminUserRepository 生产实现。
pub struct PgAdminUserRepository {
    pool: PgPool,
}

impl PgAdminUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminUserRepository for PgAdminUserRepository {
    async fn count_users(&self, filter: &AdminUserFilter) -> Result<i64, AppError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) \
             FROM users \
             WHERE deleted_at IS NULL \
               AND ($1::text IS NULL OR phone = $1) \
               AND ($2::uuid IS NULL OR id = $2) \
               AND ($3::text IS NULL OR LOWER(nickname) LIKE '%' || LOWER($3) || '%') \
               AND ($4::boolean IS NULL OR is_banned = $4)",
        )
        .bind(filter.phone.as_deref())
        .bind(filter.user_id)
        .bind(filter.nickname.as_deref())
        .bind(filter.is_banned)
        .fetch_one(&self.pool)
        .await?;
        Ok(count.0)
    }

    async fn find_users(
        &self,
        filter: &AdminUserFilter,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<AdminUserListRow>, AppError> {
        let rows = sqlx::query_as::<_, AdminUserListRow>(
            "SELECT id, phone, nickname, avatar, coin_balance, vip_level, is_banned, created_at \
             FROM users \
             WHERE deleted_at IS NULL \
               AND ($1::text IS NULL OR phone = $1) \
               AND ($2::uuid IS NULL OR id = $2) \
               AND ($3::text IS NULL OR LOWER(nickname) LIKE '%' || LOWER($3) || '%') \
               AND ($4::boolean IS NULL OR is_banned = $4) \
             ORDER BY created_at DESC \
             LIMIT $5 OFFSET $6",
        )
        .bind(filter.phone.as_deref())
        .bind(filter.user_id)
        .bind(filter.nickname.as_deref())
        .bind(filter.is_banned)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<AdminUserListRow>, AppError> {
        let row = sqlx::query_as::<_, AdminUserListRow>(
            "SELECT id, phone, nickname, avatar, coin_balance, vip_level, is_banned, created_at \
             FROM users \
             WHERE id = $1 \
               AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_ban_status(&self, id: Uuid, is_banned: bool) -> Result<bool, AppError> {
        let result = sqlx::query(
            "UPDATE users SET is_banned = $1, updated_at = NOW() \
             WHERE id = $2 AND deleted_at IS NULL",
        )
        .bind(is_banned)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

// ─── Fake 实现（内存，用于单元 / 集成测试）────────────────────────────────────

/// 内部存储条目（含软删除标记，供测试用）。
struct FakeUserEntry {
    row: AdminUserListRow,
    deleted_at: Option<DateTime<Utc>>,
}

/// 内存版 AdminUserRepository，用于单元 / 集成测试。
#[derive(Default)]
pub struct FakeAdminUserRepository {
    entries: Mutex<Vec<FakeUserEntry>>,
    /// 当为 true 时，find_user_by_id 返回 DB 错误（用于 SD-04 测试）
    force_find_by_id_error: Mutex<bool>,
    /// 当为 true 时，update_ban_status 返回 DB 错误（用于 RB-03/SB-05 测试）
    force_update_ban_error: Mutex<bool>,
}

impl FakeAdminUserRepository {
    /// 预置一条未删除的用户行（供 list 测试）。
    pub fn seed(&self, row: AdminUserListRow) {
        self.entries.lock().unwrap().push(FakeUserEntry {
            row,
            deleted_at: None,
        });
    }

    /// 预置一条已软删除的用户行（用于 R-06 测试）。
    pub fn seed_deleted(&self, row: AdminUserListRow) {
        self.entries.lock().unwrap().push(FakeUserEntry {
            row,
            deleted_at: Some(Utc::now()),
        });
    }

    /// 使 find_user_by_id 返回 DB 错误（用于 SD-04/UD 注入测试）。
    pub fn inject_find_by_id_error(&self) {
        *self.force_find_by_id_error.lock().unwrap() = true;
    }

    /// 使 update_ban_status 返回 DB 错误（用于 SB-05/RB DB 错误注入测试）。
    pub fn inject_update_ban_error(&self) {
        *self.force_update_ban_error.lock().unwrap() = true;
    }
}

#[async_trait]
impl AdminUserRepository for FakeAdminUserRepository {
    async fn count_users(&self, filter: &AdminUserFilter) -> Result<i64, AppError> {
        let guard = self.entries.lock().unwrap();
        let count = guard
            .iter()
            .filter(|e| e.deleted_at.is_none())
            .filter(|e| matches_user_filter(&e.row, filter))
            .count();
        Ok(count as i64)
    }

    async fn find_users(
        &self,
        filter: &AdminUserFilter,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<AdminUserListRow>, AppError> {
        let guard = self.entries.lock().unwrap();
        let mut results: Vec<AdminUserListRow> = guard
            .iter()
            .filter(|e| e.deleted_at.is_none())
            .filter(|e| matches_user_filter(&e.row, filter))
            .map(|e| e.row.clone())
            .collect();

        // 按 created_at DESC 排序
        results.sort_by_key(|r| std::cmp::Reverse(r.created_at));

        // 分页切片
        let start = (offset as usize).min(results.len());
        let end = ((offset + limit) as usize).min(results.len());
        Ok(results[start..end].to_vec())
    }

    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<AdminUserListRow>, AppError> {
        if *self.force_find_by_id_error.lock().unwrap() {
            return Err(AppError::DatabaseError("injected db error".to_string()));
        }
        let guard = self.entries.lock().unwrap();
        let result = guard
            .iter()
            .find(|e| e.deleted_at.is_none() && e.row.id == id)
            .map(|e| e.row.clone());
        Ok(result)
    }

    async fn update_ban_status(&self, id: Uuid, is_banned: bool) -> Result<bool, AppError> {
        if *self.force_update_ban_error.lock().unwrap() {
            return Err(AppError::DatabaseError(
                "injected update ban error".to_string(),
            ));
        }
        let mut guard = self.entries.lock().unwrap();
        match guard
            .iter_mut()
            .find(|e| e.deleted_at.is_none() && e.row.id == id)
        {
            Some(entry) => {
                entry.row.is_banned = is_banned;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

/// 辅助：检查用户行是否满足过滤条件。
fn matches_user_filter(row: &AdminUserListRow, filter: &AdminUserFilter) -> bool {
    if let Some(phone) = &filter.phone {
        if &row.phone != phone {
            return false;
        }
    }
    if let Some(user_id) = &filter.user_id {
        if &row.id != user_id {
            return false;
        }
    }
    if let Some(nickname) = &filter.nickname {
        if !row
            .nickname
            .to_lowercase()
            .contains(&nickname.to_lowercase())
        {
            return false;
        }
    }
    if let Some(is_banned) = filter.is_banned {
        if row.is_banned != is_banned {
            return false;
        }
    }
    true
}

// ─── Repository 单元测试（TDD T-10007 验收用例 R-01~R-07）────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use uuid::Uuid;

    // ── 测试辅助 ──────────────────────────────────────────────────────────────

    fn make_row(
        phone: &str,
        nickname: &str,
        is_banned: bool,
        created_at_offset_secs: i64,
    ) -> AdminUserListRow {
        AdminUserListRow {
            id: Uuid::new_v4(),
            phone: phone.to_string(),
            nickname: nickname.to_string(),
            avatar: Some("https://cdn.example.com/avatar.jpg".to_string()),
            coin_balance: 100,
            vip_level: 0,
            is_banned,
            created_at: Utc::now() + Duration::seconds(created_at_offset_secs),
        }
    }

    fn make_row_with_id(
        id: Uuid,
        phone: &str,
        nickname: &str,
        is_banned: bool,
    ) -> AdminUserListRow {
        AdminUserListRow {
            id,
            phone: phone.to_string(),
            nickname: nickname.to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned,
            created_at: Utc::now(),
        }
    }

    fn empty_filter() -> AdminUserFilter {
        AdminUserFilter::default()
    }

    // ── R-01: 无过滤条件，count 返回全部非软删除用户数，find 按 created_at DESC 排序 ──
    #[tokio::test]
    async fn r01_no_filter_returns_all_and_sorted_desc() {
        let repo = FakeAdminUserRepository::default();
        repo.seed(make_row("13800000001", "Alice", false, -30));
        repo.seed(make_row("13800000002", "Bob", false, -10));
        repo.seed(make_row("13800000003", "Charlie", false, -20));

        let count = repo.count_users(&empty_filter()).await.unwrap();
        assert_eq!(count, 3, "R-01: 无过滤应返回全部 3 个用户");

        let rows = repo.find_users(&empty_filter(), 0, 10).await.unwrap();
        assert_eq!(rows.len(), 3, "R-01: find 应返回 3 条");
        // 验证 created_at DESC 排序：Bob(-10) > Charlie(-20) > Alice(-30)
        assert_eq!(rows[0].nickname, "Bob", "R-01: 第一条应是最新的 Bob");
        assert_eq!(rows[1].nickname, "Charlie", "R-01: 第二条应是 Charlie");
        assert_eq!(rows[2].nickname, "Alice", "R-01: 第三条应是最旧的 Alice");
    }

    // ── R-02: phone 精确过滤 ─────────────────────────────────────────────────
    #[tokio::test]
    async fn r02_phone_exact_filter_returns_matching_only() {
        let repo = FakeAdminUserRepository::default();
        repo.seed(make_row("13800000001", "Alice", false, 0));
        repo.seed(make_row("13900000002", "Bob", false, 0));

        let filter = AdminUserFilter {
            phone: Some("13800000001".to_string()),
            ..Default::default()
        };

        let count = repo.count_users(&filter).await.unwrap();
        assert_eq!(count, 1, "R-02: phone 精确过滤应只匹配 1 个");

        let rows = repo.find_users(&filter, 0, 10).await.unwrap();
        assert_eq!(rows.len(), 1, "R-02: find 应只返回 1 条");
        assert_eq!(rows[0].phone, "13800000001", "R-02: 返回的手机号应匹配");
    }

    // ── R-03: nickname 模糊过滤（大小写不敏感）──────────────────────────────
    #[tokio::test]
    async fn r03_nickname_fuzzy_filter_case_insensitive() {
        let repo = FakeAdminUserRepository::default();
        repo.seed(make_row("111", "Alice Music", false, 0));
        repo.seed(make_row("222", "music bob", false, 0));
        repo.seed(make_row("333", "Charlie Games", false, 0));

        let filter = AdminUserFilter {
            nickname: Some("music".to_string()),
            ..Default::default()
        };

        let count = repo.count_users(&filter).await.unwrap();
        assert_eq!(count, 2, "R-03: 大小写不敏感 nickname 模糊应匹配 2 个");

        let rows = repo.find_users(&filter, 0, 10).await.unwrap();
        assert_eq!(rows.len(), 2, "R-03: find 应返回 2 条");
    }

    // ── R-04: is_banned 过滤 ─────────────────────────────────────────────────
    #[tokio::test]
    async fn r04_is_banned_filter_returns_correct_users() {
        let repo = FakeAdminUserRepository::default();
        repo.seed(make_row("111", "Normal User", false, 0));
        repo.seed(make_row("222", "Banned User", true, 0));
        repo.seed(make_row("333", "Another Normal", false, 0));

        // 只过滤封禁用户
        let banned_filter = AdminUserFilter {
            is_banned: Some(true),
            ..Default::default()
        };
        let count_banned = repo.count_users(&banned_filter).await.unwrap();
        assert_eq!(
            count_banned, 1,
            "R-04: is_banned=true 应只返回 1 个封禁用户"
        );
        let rows_banned = repo.find_users(&banned_filter, 0, 10).await.unwrap();
        assert!(
            rows_banned[0].is_banned,
            "R-04: 封禁过滤返回的用户应 is_banned=true"
        );

        // 只过滤正常用户
        let normal_filter = AdminUserFilter {
            is_banned: Some(false),
            ..Default::default()
        };
        let count_normal = repo.count_users(&normal_filter).await.unwrap();
        assert_eq!(count_normal, 2, "R-04: is_banned=false 应返回 2 个正常用户");
    }

    // ── R-05: user_id 精确过滤 ───────────────────────────────────────────────
    #[tokio::test]
    async fn r05_user_id_exact_filter_returns_single_user() {
        let repo = FakeAdminUserRepository::default();
        let target_id = Uuid::new_v4();
        repo.seed(make_row_with_id(target_id, "111", "Target", false));
        repo.seed(make_row("222", "Other", false, 0));

        let filter = AdminUserFilter {
            user_id: Some(target_id),
            ..Default::default()
        };

        let count = repo.count_users(&filter).await.unwrap();
        assert_eq!(count, 1, "R-05: user_id 精确过滤应只返回 1 个");

        let rows = repo.find_users(&filter, 0, 10).await.unwrap();
        assert_eq!(rows.len(), 1, "R-05: find 应返回 1 条");
        assert_eq!(rows[0].id, target_id, "R-05: 返回的 id 应匹配");
    }

    // ── R-06: 软删除用户不计入结果 ──────────────────────────────────────────
    #[tokio::test]
    async fn r06_soft_deleted_users_excluded() {
        let repo = FakeAdminUserRepository::default();
        repo.seed(make_row("111", "Active User", false, 0));
        repo.seed_deleted(make_row("222", "Deleted User", false, 0));

        let count = repo.count_users(&empty_filter()).await.unwrap();
        assert_eq!(count, 1, "R-06: 软删除用户不应计入 count");

        let rows = repo.find_users(&empty_filter(), 0, 10).await.unwrap();
        assert_eq!(rows.len(), 1, "R-06: 软删除用户不应出现在 find 结果中");
        assert_eq!(rows[0].nickname, "Active User", "R-06: 只返回未删除的用户");
    }

    // ── R-07: 分页正确：offset=5, limit=5，只返回第 6-10 条 ─────────────────
    #[tokio::test]
    async fn r07_pagination_offset_limit_returns_correct_slice() {
        let repo = FakeAdminUserRepository::default();
        // 预置 10 条数据，offset 递减确保 created_at 顺序可预测
        for i in 0..10 {
            let phone = format!("138000000{:02}", i);
            let nickname = format!("User{:02}", i);
            repo.seed(make_row(&phone, &nickname, false, -(i as i64)));
        }

        // 总数应为 10
        let count = repo.count_users(&empty_filter()).await.unwrap();
        assert_eq!(count, 10, "R-07: 应有 10 条数据");

        // offset=5, limit=5 → 取第 6-10 条
        let rows = repo.find_users(&empty_filter(), 5, 5).await.unwrap();
        assert_eq!(rows.len(), 5, "R-07: offset=5,limit=5 应返回 5 条");

        // 排序后第 0-4 是 User00~User04，第 5-9 是 User05~User09
        // created_at DESC: 偏移=-0 最新, -9 最旧
        // 排序结果：User00 > User01 > ... > User09
        // offset=5 → rows[0] 应是 User05
        assert_eq!(rows[0].nickname, "User05", "R-07: 分页起始应是 User05");
        assert_eq!(rows[4].nickname, "User09", "R-07: 分页末尾应是 User09");
    }

    // ── 额外：空结果时 count=0, find=[] ─────────────────────────────────────
    #[tokio::test]
    async fn r_empty_result_total_zero() {
        let repo = FakeAdminUserRepository::default();
        let filter = AdminUserFilter {
            phone: Some("00000000000".to_string()),
            ..Default::default()
        };

        let count = repo.count_users(&filter).await.unwrap();
        assert_eq!(count, 0, "空结果 count 应为 0");

        let rows = repo.find_users(&filter, 0, 10).await.unwrap();
        assert!(rows.is_empty(), "空结果 find 应返回空 Vec");
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10008 Repository 测试 RD-01~03
    // ════════════════════════════════════════════════════════════════════════

    // ── RD-01: 存在且未软删除的用户通过 ID 查找 → 返回 Some(row) ──────────────
    #[tokio::test]
    async fn rd01_existing_user_found_by_id() {
        let repo = FakeAdminUserRepository::default();
        let id = Uuid::new_v4();
        let row = AdminUserListRow {
            id,
            phone: "+8613800138001".to_string(),
            nickname: "RD01User".to_string(),
            avatar: Some("https://cdn.example.com/avatar.jpg".to_string()),
            coin_balance: 500,
            vip_level: 1,
            is_banned: false,
            created_at: Utc::now(),
        };
        repo.seed(row.clone());

        let result = repo.find_user_by_id(id).await.unwrap();
        assert!(result.is_some(), "RD-01: 存在的用户应返回 Some(row)");
        let found = result.unwrap();
        assert_eq!(found.id, id, "RD-01: 返回的 id 应与查询 id 一致");
        assert_eq!(found.phone, "+8613800138001", "RD-01: phone 字段应一致");
        assert_eq!(found.nickname, "RD01User", "RD-01: nickname 字段应一致");
        assert_eq!(found.coin_balance, 500, "RD-01: coin_balance 字段应一致");
        assert_eq!(found.vip_level, 1, "RD-01: vip_level 字段应一致");
    }

    // ── RD-02: 不存在的 UUID → 返回 None ────────────────────────────────────
    #[tokio::test]
    async fn rd02_nonexistent_uuid_returns_none() {
        let repo = FakeAdminUserRepository::default();
        // 预置一个用户但查询另一个 UUID
        repo.seed(make_row("111", "SomeUser", false, 0));

        let nonexistent_id = Uuid::new_v4();
        let result = repo.find_user_by_id(nonexistent_id).await.unwrap();
        assert!(result.is_none(), "RD-02: 不存在的 UUID 应返回 None");
    }

    // ── RD-03: 软删除的用户 → 返回 None ─────────────────────────────────────
    #[tokio::test]
    async fn rd03_soft_deleted_user_returns_none() {
        let repo = FakeAdminUserRepository::default();
        let id = Uuid::new_v4();
        let deleted_row = AdminUserListRow {
            id,
            phone: "+8613800138003".to_string(),
            nickname: "DeletedUser".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: Utc::now(),
        };
        repo.seed_deleted(deleted_row);

        let result = repo.find_user_by_id(id).await.unwrap();
        assert!(
            result.is_none(),
            "RD-03: 软删除的用户应返回 None（deleted_at IS NOT NULL）"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10009 Repository 测试 RB-01~03
    // ════════════════════════════════════════════════════════════════════════

    // ── RB-01: 存在的未软删除用户 → update_ban_status(id, true) 返回 Ok(true)，
    //          再用 find_user_by_id 查验 is_banned=true ────────────────────────
    #[tokio::test]
    async fn rb01_update_ban_status_existing_user_returns_true_and_updates_state() {
        let repo = FakeAdminUserRepository::default();
        let id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id,
            phone: "+8613800138001".to_string(),
            nickname: "RB01User".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: Utc::now(),
        });

        // 执行封禁
        let result = repo.update_ban_status(id, true).await.unwrap();
        assert!(result, "RB-01: 存在的用户 update_ban_status 应返回 true");

        // 验证状态已更新
        let found = repo.find_user_by_id(id).await.unwrap().unwrap();
        assert!(found.is_banned, "RB-01: 执行封禁后 is_banned 应为 true");
    }

    // ── RB-02: 已封禁用户 → update_ban_status(id, false) 返回 Ok(true)，
    //          再验证 is_banned=false ──────────────────────────────────────────
    #[tokio::test]
    async fn rb02_update_ban_status_banned_user_unban_returns_true_and_updates_state() {
        let repo = FakeAdminUserRepository::default();
        let id = Uuid::new_v4();
        repo.seed(AdminUserListRow {
            id,
            phone: "+8613800138002".to_string(),
            nickname: "RB02User".to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: true, // 已封禁
            created_at: Utc::now(),
        });

        // 执行解封
        let result = repo.update_ban_status(id, false).await.unwrap();
        assert!(result, "RB-02: 解封已封禁用户应返回 true");

        // 验证状态已更新
        let found = repo.find_user_by_id(id).await.unwrap().unwrap();
        assert!(!found.is_banned, "RB-02: 执行解封后 is_banned 应为 false");
    }

    // ── RB-03: 不存在的 UUID → update_ban_status 返回 Ok(false)（0 affected rows）──
    #[tokio::test]
    async fn rb03_update_ban_status_nonexistent_uuid_returns_false() {
        let repo = FakeAdminUserRepository::default();
        let nonexistent_id = Uuid::new_v4();

        let result = repo.update_ban_status(nonexistent_id, true).await.unwrap();
        assert!(
            !result,
            "RB-03: 不存在的 UUID update_ban_status 应返回 false（0 affected rows）"
        );
    }
}
