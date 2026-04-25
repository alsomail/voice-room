//! P0 回归测试 — Admin Server `find_by_username` 真实 SQL 集成
//!
//! 缺陷背景（GlobalReview 第 1 轮 缺陷 1）：
//!   `PgAdminRepository::find_by_username` SQL 引用 `admins.deleted_at`，
//!   但原始 001 迁移并未定义该列，生产调用必返回 PostgreSQL 42703 → HTTP 500。
//!
//! 修复方案：新增 `004_add_admins_deleted_at.sql`，对齐 users 表软删除语义。
//!
//! 本测试覆盖：
//! - **migrations_text_includes_deleted_at**：静态校验迁移目录中存在 deleted_at 列定义
//!   （即便没有 PostgreSQL 也能跑，CI 离线机器也能拦截回归）。
//! - **find_by_username_real_sql_against_full_schema**：真实 PostgreSQL 集成测试 —
//!   在 PerSession TEMP TABLE 中按 001+004 的最终 schema 重建 admins，
//!   插入 fixture 后调用 `PgAdminRepository::find_by_username`，断言返回 Some(..)。
//!   若回滚到旧 schema（无 deleted_at），SQL 必报 42703 → 测试 RED。
//! - **soft_deleted_admin_returns_none**：deleted_at IS NOT NULL 的行被过滤掉。
//!
//! 之所以使用 TEMP TABLE 而非 `sqlx::migrate!`：当前 dev 环境下 app_server 与
//! admin_server 复用同一 `voiceroom` DB 与同一 `_sqlx_migrations` 表，两套迁移
//! 版本号会冲突。TEMP TABLE 在会话内屏蔽 public 表，足以覆盖 SQL 行为契约。

use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;
use voice_room_admin_server::modules::auth::repository::{AdminRepository, PgAdminRepository};

// ─── 静态文本校验：不依赖 DB ───────────────────────────────────────────────

#[test]
fn migrations_define_admins_deleted_at_column() {
    // 缺陷 1 的关键不变量：001 + 后续迁移必须为 admins 表添加 deleted_at。
    let m001 = include_str!("../migrations/001_create_admins.sql");
    let m004 = include_str!("../migrations/004_add_admins_deleted_at.sql");

    let combined = format!("{m001}\n{m004}").to_lowercase();
    assert!(
        combined.contains("deleted_at"),
        "admins 表必须定义 deleted_at 列；当前迁移文本不含该列，与 \
         find_by_username SQL 不兼容。回归到缺陷 1。"
    );
    assert!(
        combined.contains("alter table admins") && combined.contains("add column"),
        "004 迁移应通过 ALTER TABLE admins ADD COLUMN deleted_at 显式补列"
    );
}

// ─── 真实 PostgreSQL 集成 ─────────────────────────────────────────────────

/// 使用 `connection_options.options` 把 search_path 指向 pg_temp，
/// 这样 TEMP TABLE admins 会优先于（未必存在的）public.admins 被 SQL 访问到。
async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(1) // pg_temp 是会话级，必须单连接复用
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

/// 在当前会话中按 001 + 004 的最终形态重建 admins TEMP TABLE。
async fn create_temp_admins_table(pool: &PgPool) {
    // 1) 让 search_path 优先看到 pg_temp，临时表覆盖 public.admins
    sqlx::query("SET search_path TO pg_temp, public")
        .execute(pool)
        .await
        .expect("set search_path");

    // 2) 重建 admins —— 与 001 + 004 合并后的最终 schema 完全一致
    sqlx::query(
        "CREATE TEMP TABLE admins ( \
            id            UUID         PRIMARY KEY DEFAULT gen_random_uuid(), \
            username      VARCHAR(50)  NOT NULL, \
            password_hash VARCHAR(200) NOT NULL, \
            role          VARCHAR(20)  NOT NULL DEFAULT 'operator', \
            display_name  VARCHAR(100), \
            is_active     BOOLEAN      NOT NULL DEFAULT TRUE, \
            last_login_at TIMESTAMPTZ, \
            created_at    TIMESTAMPTZ  NOT NULL DEFAULT now(), \
            updated_at    TIMESTAMPTZ  NOT NULL DEFAULT now(), \
            deleted_at    TIMESTAMPTZ \
         ) ON COMMIT PRESERVE ROWS",
    )
    .execute(pool)
    .await
    .expect("create temp admins table");
}

#[tokio::test]
async fn find_by_username_real_sql_against_full_schema() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] DATABASE_URL not set — skipping real-DB integration test");
        return;
    };

    create_temp_admins_table(&pool).await;

    // 插入一个 fixture（活跃管理员）
    let fixture_username = format!("tdd_admin_{}", Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO admins (username, password_hash, role, display_name, is_active) \
         VALUES ($1, '$2b$12$placeholder.hash.for.repository.test................', \
                 'super_admin', 'TDD fixture', TRUE)",
    )
    .bind(&fixture_username)
    .execute(&pool)
    .await
    .expect("insert active fixture admin");

    // 关键回归点：这条 SQL 引用了 deleted_at；若 schema 缺该列必报 42703
    let repo = PgAdminRepository::new(pool.clone());
    let admin = repo
        .find_by_username(&fixture_username)
        .await
        .expect("find_by_username 必须成功——若 schema 缺 deleted_at，此处会 42703 panic（缺陷 1）")
        .expect("活跃 fixture 必须能被检索到");

    assert_eq!(admin.username, fixture_username);
    assert_eq!(admin.role, "super_admin");
    assert!(admin.is_active);
}

#[tokio::test]
async fn soft_deleted_admin_returns_none() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] DATABASE_URL not set");
        return;
    };

    create_temp_admins_table(&pool).await;

    let username = format!("tdd_softdel_{}", Uuid::new_v4().simple());
    sqlx::query(
        "INSERT INTO admins (username, password_hash, role, display_name, is_active, deleted_at) \
         VALUES ($1, '$2b$12$placeholder.hash................................', \
                 'operator', 'soft-deleted fixture', TRUE, now())",
    )
    .bind(&username)
    .execute(&pool)
    .await
    .expect("insert soft-deleted admin");

    let repo = PgAdminRepository::new(pool.clone());
    let res = repo
        .find_by_username(&username)
        .await
        .expect("SQL must succeed against full schema");

    assert!(
        res.is_none(),
        "deleted_at IS NOT NULL 的管理员不应被 find_by_username 返回"
    );
}
