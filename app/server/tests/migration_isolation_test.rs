//! T-0000M：双服务共库 Migration 表隔离 — 集成测试
//!
//! 覆盖 TDS §3.1 / §3.3：
//! - U-1 迁移记录隔离（两张登记表共存，行数 = 9 / 4）
//! - U-2 启动顺序无关（先 admin → app；先 app → admin 两种顺序结果一致）
//! - U-3 重复启动幂等（连续 3 次行数恒定）
//! - N-1 旧 `_sqlx_migrations` 残留库兼容（不污染新表）
//! - N-3 删除某迁移后启动，错误消息含表名 `_sqlx_app_migrations`
//!
//! 运行前提：DATABASE_URL 指向可用 PostgreSQL 实例；未设置则跳过。
//!
//! 注意：本测试仅以 superuser（`postgres`）连接的 DATABASE_URL 才能动态创建/销毁
//! 隔离用 schema。受限账号 `app_server_user` 在 dev 环境下有 `CREATE ON SCHEMA public`
//! 权限，亦可执行（见 init-db.sh）。

mod common;

use sqlx::migrate::Migrator;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;
use voice_room_shared::migrate::{run_migrations_with_table, MigrateTableError};

async fn try_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

/// 为每个测试用例创建独立 schema 名（避免与产线表撞名）。
fn fresh_schema_name() -> String {
    let s = Uuid::new_v4().simple().to_string();
    format!("t0m_{}", &s[..16])
}

async fn create_isolated_schema(pool: &PgPool, schema: &str) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema}\""))
        .execute(pool)
        .await?;
    sqlx::query(&format!("SET search_path TO \"{schema}\""))
        .execute(pool)
        .await?;
    Ok(())
}

async fn drop_schema(pool: &PgPool, schema: &str) {
    let _ = sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"))
        .execute(pool)
        .await;
}

/// 在 schema-隔离上运行：每个测试自己开 PgConnectOptions 并 application_name + search_path。
async fn isolated_pool(schema: &str) -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let opts: sqlx::postgres::PgConnectOptions = url
        .parse::<sqlx::postgres::PgConnectOptions>()
        .ok()?
        .options([("search_path", schema)]);
    PgPoolOptions::new()
        .max_connections(3)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(opts)
        .await
        .ok()
}

async fn count_rows(pool: &PgPool, table: &str) -> i64 {
    let row = sqlx::query(&format!("SELECT COUNT(*)::BIGINT AS c FROM {table}"))
        .fetch_one(pool)
        .await
        .expect("count");
    row.get::<i64, _>("c")
}

fn admin_migrations_path() -> std::path::PathBuf {
    // 从 app/server/tests/ 出发到 app/adminServer/migrations
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    Path::new(&manifest)
        .join("..")
        .join("adminServer")
        .join("migrations")
}

// ───────────────────────────────────────────────────────────────────────────
// U-1: 双服务在同一库内共存，迁移记录两张表互不感知
// ───────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn u1_dual_migrations_table_coexist() {
    let Some(setup_pool) = try_pool().await else {
        eprintln!("[SKIP] u1: DATABASE_URL not set");
        return;
    };
    let schema = fresh_schema_name();
    create_isolated_schema(&setup_pool, &schema).await.unwrap();
    let pool = isolated_pool(&schema).await.expect("isolated pool");

    // 先启动 AppServer（9 条迁移）
    run_migrations_with_table(
        &pool,
        &sqlx::migrate!("./migrations"),
        "_sqlx_app_migrations",
    )
    .await
    .expect("app migrations");

    // 再启动 AdminServer（4 条迁移）— 用 runtime Migrator 加载 adminServer 目录
    let admin_migrator = Migrator::new(admin_migrations_path().as_path())
        .await
        .expect("load admin migrator");
    run_migrations_with_table(&pool, &admin_migrator, "_sqlx_admin_migrations")
        .await
        .expect("admin migrations");

    let app_count = count_rows(&pool, "_sqlx_app_migrations").await;
    let admin_count = count_rows(&pool, "_sqlx_admin_migrations").await;
    assert_eq!(app_count, 9, "AppServer should record 9 migrations");
    assert_eq!(admin_count, 4, "AdminServer should record 4 migrations");

    drop_schema(&setup_pool, &schema).await;
}

// ───────────────────────────────────────────────────────────────────────────
// U-2: 启动顺序无关（先 admin 再 app）
// ───────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn u2_startup_order_independent() {
    let Some(setup_pool) = try_pool().await else {
        eprintln!("[SKIP] u2: DATABASE_URL not set");
        return;
    };
    let schema = fresh_schema_name();
    create_isolated_schema(&setup_pool, &schema).await.unwrap();
    let pool = isolated_pool(&schema).await.expect("isolated pool");

    // 反序：先 admin 再 app
    let admin_migrator = Migrator::new(admin_migrations_path().as_path())
        .await
        .expect("load admin migrator");
    run_migrations_with_table(&pool, &admin_migrator, "_sqlx_admin_migrations")
        .await
        .expect("admin migrations first");

    run_migrations_with_table(
        &pool,
        &sqlx::migrate!("./migrations"),
        "_sqlx_app_migrations",
    )
    .await
    .expect("app migrations second");

    assert_eq!(count_rows(&pool, "_sqlx_app_migrations").await, 9);
    assert_eq!(count_rows(&pool, "_sqlx_admin_migrations").await, 4);

    drop_schema(&setup_pool, &schema).await;
}

// ───────────────────────────────────────────────────────────────────────────
// U-3: 重复启动幂等（连续 3 次行数不变）
// ───────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn u3_repeated_startup_is_idempotent() {
    let Some(setup_pool) = try_pool().await else {
        eprintln!("[SKIP] u3: DATABASE_URL not set");
        return;
    };
    let schema = fresh_schema_name();
    create_isolated_schema(&setup_pool, &schema).await.unwrap();
    let pool = isolated_pool(&schema).await.expect("isolated pool");

    for _ in 0..3 {
        run_migrations_with_table(
            &pool,
            &sqlx::migrate!("./migrations"),
            "_sqlx_app_migrations",
        )
        .await
        .expect("idempotent app migrations");
    }
    assert_eq!(count_rows(&pool, "_sqlx_app_migrations").await, 9);

    let admin_migrator = Migrator::new(admin_migrations_path().as_path())
        .await
        .expect("load admin migrator");
    for _ in 0..3 {
        run_migrations_with_table(&pool, &admin_migrator, "_sqlx_admin_migrations")
            .await
            .expect("idempotent admin migrations");
    }
    assert_eq!(count_rows(&pool, "_sqlx_admin_migrations").await, 4);

    drop_schema(&setup_pool, &schema).await;
}

// ───────────────────────────────────────────────────────────────────────────
// N-1: 旧 `_sqlx_migrations` 残留库不污染新自定义表
// ───────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn n1_legacy_sqlx_migrations_does_not_pollute() {
    let Some(setup_pool) = try_pool().await else {
        eprintln!("[SKIP] n1: DATABASE_URL not set");
        return;
    };
    let schema = fresh_schema_name();
    create_isolated_schema(&setup_pool, &schema).await.unwrap();
    let pool = isolated_pool(&schema).await.expect("isolated pool");

    // 模拟历史残留：手工建一张默认 `_sqlx_migrations` 并塞 1 行混杂版本
    sqlx::query(
        "CREATE TABLE _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),
            success BOOLEAN NOT NULL,
            checksum BYTEA NOT NULL,
            execution_time BIGINT NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (999, 'legacy', TRUE, '\\x00', 0)")
        .execute(&pool).await.unwrap();

    // 启动 AppServer：应不报错地建立 `_sqlx_app_migrations`
    run_migrations_with_table(
        &pool,
        &sqlx::migrate!("./migrations"),
        "_sqlx_app_migrations",
    )
    .await
    .expect("startup with legacy table");

    assert_eq!(count_rows(&pool, "_sqlx_app_migrations").await, 9);
    assert_eq!(
        count_rows(&pool, "_sqlx_migrations").await,
        1,
        "legacy untouched"
    );

    drop_schema(&setup_pool, &schema).await;
}

// ───────────────────────────────────────────────────────────────────────────
// N-3: 模拟「删除 009_*.sql」后启动 — 报「missing migration」且消息含表名
// ───────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn n3_missing_migration_error_mentions_table_name() {
    let Some(setup_pool) = try_pool().await else {
        eprintln!("[SKIP] n3: DATABASE_URL not set");
        return;
    };
    let schema = fresh_schema_name();
    create_isolated_schema(&setup_pool, &schema).await.unwrap();
    let pool = isolated_pool(&schema).await.expect("isolated pool");

    // 先正常跑完 9 条迁移
    run_migrations_with_table(
        &pool,
        &sqlx::migrate!("./migrations"),
        "_sqlx_app_migrations",
    )
    .await
    .expect("first run");

    // 模拟「删除 009_*.sql」：构造一个只含 1..3 三条迁移的 Migrator
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let trimmed_dir = std::path::PathBuf::from(&manifest)
        .join("target")
        .join("t0m_trimmed_migrations")
        .join(Uuid::new_v4().simple().to_string());
    std::fs::create_dir_all(&trimmed_dir).unwrap();
    let migrations_dir = std::path::Path::new(&manifest).join("migrations");
    let mut entries: Vec<_> = std::fs::read_dir(&migrations_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "sql").unwrap_or(false))
        .collect();
    entries.sort();
    // 只复制前 3 条
    for p in entries.iter().take(3) {
        let dst = trimmed_dir.join(p.file_name().unwrap());
        std::fs::copy(p, &dst).unwrap();
    }
    let trimmed = Migrator::new(trimmed_dir.as_path())
        .await
        .expect("trimmed migrator");

    let err = run_migrations_with_table(&pool, &trimmed, "_sqlx_app_migrations")
        .await
        .expect_err("should fail with missing migration");
    let msg = err.to_string();
    assert!(
        matches!(err, MigrateTableError::VersionMissing { .. }),
        "expected VersionMissing, got {err:?}"
    );
    assert!(
        msg.contains("_sqlx_app_migrations"),
        "error msg should mention the migrations table; got: {msg}"
    );

    // 清理临时目录
    let _ = std::fs::remove_dir_all(&trimmed_dir);
    drop_schema(&setup_pool, &schema).await;
}

// 占位：保证 Migrator 引用不会被 cargo 标记为未使用
#[allow(dead_code)]
fn _unused() {
    let _ = std::any::type_name::<Migrator>();
}
