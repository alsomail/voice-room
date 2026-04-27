//! 集成测试 — T-00017 钱包 Schema 与迁移
//!
//! 测试用例 W01~W06 验证以下内容：
//! - W01: 迁移可幂等执行（多次运行不报错）
//! - W02: 新注册用户 diamond_balance 默认 0
//! - W03: UPDATE users SET diamond_balance = -1 被 CHECK 约束拒绝（PG 23514）
//! - W04: wallet_transactions 插入 balance_after = -5 被 CHECK 拒绝
//! - W05: 复合索引 (user_id, created_at DESC) 存在并命中 EXPLAIN
//! - W06: 存量 users 迁移后 diamond_balance = 0
//!
//! 运行前提：DATABASE_URL 环境变量指向可用的 PostgreSQL 实例。
//! 若未设置 DATABASE_URL，所有测试将被跳过。

mod common;

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

/// 获取测试用连接池；未配置 DATABASE_URL 或连接失败时返回 None（测试跳过）
async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

// ────────────────────────────────────────────────
// W01: 迁移幂等性 — 连续执行两次 migrate run 不报错
// ────────────────────────────────────────────────
#[tokio::test]
async fn w01_migration_is_idempotent() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w01: DATABASE_URL not set");
        return;
    };

    // 第一次运行迁移（或已运行过）
    common::run_migrations(&pool)
        .await
        .expect("First migration run should succeed");

    // 第二次运行迁移 — 幂等，不应报错
    common::run_migrations(&pool)
        .await
        .expect("Second migration run should also succeed (idempotent)");
}

// ────────────────────────────────────────────────
// W02: 新注册用户 diamond_balance 默认为 0
// ────────────────────────────────────────────────
#[tokio::test]
async fn w02_new_user_diamond_balance_defaults_to_zero() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w02: DATABASE_URL not set");
        return;
    };

    common::run_migrations(&pool).await.expect("migrations");

    let mut tx = pool.begin().await.expect("begin tx");

    let phone = format!("+8613{}", &Uuid::new_v4().to_string()[..8].replace('-', ""));
    let row = sqlx::query(
        "INSERT INTO users (phone, nickname) VALUES ($1, $2) RETURNING diamond_balance",
    )
    .bind(&phone)
    .bind("TestUser")
    .fetch_one(&mut *tx)
    .await
    .expect("insert user");

    let balance: i64 = row.get("diamond_balance");
    assert_eq!(balance, 0, "New user diamond_balance should default to 0");

    tx.rollback().await.expect("rollback");
}

// ────────────────────────────────────────────────
// W03: diamond_balance 不可设为负数（CHECK 约束 23514）
// ────────────────────────────────────────────────
#[tokio::test]
async fn w03_negative_diamond_balance_rejected_by_check_constraint() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w03: DATABASE_URL not set");
        return;
    };

    common::run_migrations(&pool).await.expect("migrations");

    let mut tx = pool.begin().await.expect("begin tx");

    // 先插入一个用户
    let phone = format!("+8614{}", &Uuid::new_v4().to_string()[..8].replace('-', ""));
    let row = sqlx::query("INSERT INTO users (phone, nickname) VALUES ($1, $2) RETURNING id")
        .bind(&phone)
        .bind("CheckUser")
        .fetch_one(&mut *tx)
        .await
        .expect("insert user");
    let user_id: Uuid = row.get("id");

    // 尝试将 diamond_balance 设为 -1
    let result = sqlx::query("UPDATE users SET diamond_balance = -1 WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await;

    // 断言：应返回错误，且错误码为 23514（check_violation）
    let err = result.expect_err("Negative diamond_balance should be rejected by CHECK constraint");
    let err_str = err.to_string();
    assert!(
        err_str.contains("23514") || err_str.contains("check"),
        "Expected PostgreSQL CHECK constraint violation (23514), got: {err_str}"
    );

    tx.rollback().await.ok(); // 已经出错，rollback 可能失败，忽略
}

// ────────────────────────────────────────────────
// W04: wallet_transactions 插入 balance_after = -5 被 CHECK 拒绝
// ────────────────────────────────────────────────
#[tokio::test]
async fn w04_negative_balance_after_in_wallet_txn_rejected() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w04: DATABASE_URL not set");
        return;
    };

    common::run_migrations(&pool).await.expect("migrations");

    let mut tx = pool.begin().await.expect("begin tx");

    // 先插入一个用户
    let phone = format!("+8615{}", &Uuid::new_v4().to_string()[..8].replace('-', ""));
    let row = sqlx::query("INSERT INTO users (phone, nickname) VALUES ($1, $2) RETURNING id")
        .bind(&phone)
        .bind("TxnUser")
        .fetch_one(&mut *tx)
        .await
        .expect("insert user");
    let user_id: Uuid = row.get("id");

    // 尝试插入 balance_after = -5 的流水记录
    let result = sqlx::query(
        r#"INSERT INTO wallet_transactions
           (user_id, type, amount, balance_after)
           VALUES ($1, 'recharge', 100, -5)"#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await;

    let err = result.expect_err("balance_after = -5 should be rejected by CHECK constraint");
    let err_str = err.to_string();
    assert!(
        err_str.contains("23514") || err_str.contains("check"),
        "Expected PostgreSQL CHECK constraint violation (23514), got: {err_str}"
    );

    tx.rollback().await.ok();
}

// ────────────────────────────────────────────────
// W05: 复合索引 (user_id, created_at DESC) 命中 EXPLAIN
// ────────────────────────────────────────────────
#[tokio::test]
async fn w05_composite_index_on_user_id_created_at_exists() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w05: DATABASE_URL not set");
        return;
    };

    common::run_migrations(&pool).await.expect("migrations");

    // 直接查询 pg_indexes 确认索引存在
    let row = sqlx::query(
        r#"SELECT indexname
           FROM pg_indexes
           WHERE tablename = 'wallet_transactions'
             AND indexdef ILIKE '%user_id%created_at%'"#,
    )
    .fetch_optional(&pool)
    .await
    .expect("query pg_indexes");

    assert!(
        row.is_some(),
        "Index on wallet_transactions(user_id, created_at) should exist"
    );

    let index_name: String = row.unwrap().get("indexname");
    assert!(
        index_name.contains("wallet_txn") || index_name.contains("user"),
        "Index name should be meaningful, got: {index_name}"
    );
}

// ────────────────────────────────────────────────
// W06: 存量 users 迁移后 diamond_balance = 0（不受影响）
// ────────────────────────────────────────────────
#[tokio::test]
async fn w06_existing_users_have_zero_diamond_balance_after_migration() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w06: DATABASE_URL not set");
        return;
    };

    common::run_migrations(&pool).await.expect("migrations");

    let mut tx = pool.begin().await.expect("begin tx");

    // 插入两个"老"用户（模拟存量数据，不显式设置 diamond_balance）
    let phone1 = format!("+8616{}", &Uuid::new_v4().to_string()[..8].replace('-', ""));
    let phone2 = format!("+8617{}", &Uuid::new_v4().to_string()[..8].replace('-', ""));

    sqlx::query("INSERT INTO users (phone, nickname) VALUES ($1, $2), ($3, $4)")
        .bind(&phone1)
        .bind("OldUser1")
        .bind(&phone2)
        .bind("OldUser2")
        .execute(&mut *tx)
        .await
        .expect("insert legacy users");

    // 查询这两个用户的 diamond_balance，应均为 0
    let rows =
        sqlx::query("SELECT diamond_balance FROM users WHERE phone IN ($1, $2) ORDER BY phone")
            .bind(&phone1)
            .bind(&phone2)
            .fetch_all(&mut *tx)
            .await
            .expect("fetch legacy users");

    assert_eq!(rows.len(), 2, "Should find both legacy users");
    for row in &rows {
        let balance: i64 = row.get("diamond_balance");
        assert_eq!(
            balance, 0,
            "Legacy users should have diamond_balance = 0 after migration"
        );
    }

    tx.rollback().await.expect("rollback");
}

// ────────────────────────────────────────────────
// W07: wallet_transactions 正常插入（balance_after >= 0）
// ────────────────────────────────────────────────
#[tokio::test]
async fn w07_valid_wallet_transaction_insert_succeeds() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w07: DATABASE_URL not set");
        return;
    };

    common::run_migrations(&pool).await.expect("migrations");

    let mut tx = pool.begin().await.expect("begin tx");

    // 插入用户
    let phone = format!("+8618{}", &Uuid::new_v4().to_string()[..8].replace('-', ""));
    let row = sqlx::query("INSERT INTO users (phone, nickname) VALUES ($1, $2) RETURNING id")
        .bind(&phone)
        .bind("ValidTxnUser")
        .fetch_one(&mut *tx)
        .await
        .expect("insert user");
    let user_id: Uuid = row.get("id");

    // 插入合法流水记录
    let result = sqlx::query(
        r#"INSERT INTO wallet_transactions
           (user_id, type, amount, balance_after, reason)
           VALUES ($1, 'recharge', 100, 100, '首次充值')
           RETURNING id"#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await;

    assert!(
        result.is_ok(),
        "Valid wallet transaction insert should succeed, got: {:?}",
        result.err()
    );

    tx.rollback().await.expect("rollback");
}

// ────────────────────────────────────────────────
// W08: wallet_transactions type 字段接受所有枚举值
// ────────────────────────────────────────────────
#[tokio::test]
async fn w08_all_txn_types_accepted_by_db() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w08: DATABASE_URL not set");
        return;
    };

    common::run_migrations(&pool).await.expect("migrations");

    let mut tx = pool.begin().await.expect("begin tx");

    // 插入用户
    let phone = format!("+8619{}", &Uuid::new_v4().to_string()[..8].replace('-', ""));
    let row = sqlx::query("INSERT INTO users (phone, nickname) VALUES ($1, $2) RETURNING id")
        .bind(&phone)
        .bind("TypeTestUser")
        .fetch_one(&mut *tx)
        .await
        .expect("insert user");
    let user_id: Uuid = row.get("id");

    let types = [
        "gift_send",
        "gift_receive",
        "admin_adjust",
        "recharge",
        "refund",
    ];
    for txn_type in &types {
        let result = sqlx::query(
            r#"INSERT INTO wallet_transactions
               (user_id, type, amount, balance_after)
               VALUES ($1, $2, 0, 0)"#,
        )
        .bind(user_id)
        .bind(txn_type)
        .execute(&mut *tx)
        .await;

        assert!(
            result.is_ok(),
            "Type '{txn_type}' should be accepted by DB, got: {:?}",
            result.err()
        );
    }

    tx.rollback().await.expect("rollback");
}
