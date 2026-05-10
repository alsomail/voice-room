//! 集成测试 — T-00050 支付 Schema 与迁移验证
//!
//! 验收用例（§4 实现结果测试）：
//! - W50-01: 迁移幂等执行（连续两次不报错）
//! - W50-02: 5 条 SKU 种子数据（sku_id 精确匹配协议 §附录 B）
//! - W50-03: UNIQUE(provider, purchase_token) 约束正常工作
//! - W50-04: payment_order_state CHECK 约束（无效 state 被拒绝）
//! - W50-05: wallet_transactions.source 列已存在（迁移 011 添加）
//!
//! 运行前提：DATABASE_URL 环境变量指向可用的 PostgreSQL 实例。
//! 未设置时所有测试自动跳过。

mod common;

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use uuid::Uuid;

/// 获取测试连接池；无 DATABASE_URL 或连接失败时返回 None（测试跳过）
async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

// ─── W50-01: 迁移幂等性 ───────────────────────────────────────────────────────

#[tokio::test]
async fn w50_01_payment_migration_is_idempotent() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w50_01: DATABASE_URL not set");
        return;
    };

    // 第一次（可能已经执行过）
    common::run_migrations(&pool)
        .await
        .expect("First migration run should succeed");

    // 第二次 — 幂等，不应报错
    common::run_migrations(&pool)
        .await
        .expect("Second migration run should also succeed (idempotent)");
}

// ─── W50-02: 5 条 SKU 种子数据 ────────────────────────────────────────────────

#[tokio::test]
async fn w50_02_five_sku_seeds_present() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w50_02: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    // 协议 §附录 B 规定的 5 个 SKU ID
    let expected_skus = [
        "diamond_60",
        "diamond_300",
        "diamond_980",
        "diamond_1980",
        "diamond_3280",
    ];

    let rows = sqlx::query("SELECT sku_id FROM payment_skus WHERE provider = 'google_play' ORDER BY sort_order")
        .fetch_all(&pool)
        .await
        .expect("Should query payment_skus");

    let found_ids: Vec<String> = rows
        .iter()
        .map(|r| r.get::<String, _>("sku_id"))
        .collect();

    assert_eq!(
        found_ids.len(),
        5,
        "Expected 5 SKU seeds, got {}: {:?}",
        found_ids.len(),
        found_ids
    );

    for expected in &expected_skus {
        assert!(
            found_ids.contains(&expected.to_string()),
            "Missing expected SKU: {expected}. Found: {found_ids:?}"
        );
    }
}

// ─── W50-03: UNIQUE(provider, purchase_token) 约束 ───────────────────────────

#[tokio::test]
async fn w50_03_unique_provider_purchase_token_constraint() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w50_03: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    // 插入一个需要 user_id 的测试用户
    let user_id: Uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (user_id, phone, password_hash, nickname)
         VALUES ($1, $2, 'hash', 'test_user')
         ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(format!("+1555{}", &user_id.to_string()[..8]))
    .execute(&pool)
    .await
    .unwrap();

    let token = format!("test_token_{}", Uuid::new_v4());

    // 第一次插入 — 成功
    sqlx::query(
        "INSERT INTO payment_orders (user_id, sku_id, provider, state, purchase_token)
         VALUES ($1, 'diamond_60', 'google_play', 'PENDING', $2)",
    )
    .bind(user_id)
    .bind(&token)
    .execute(&pool)
    .await
    .expect("First insert with purchase_token should succeed");

    // 第二次插入相同 (provider, purchase_token) — 应违反 UNIQUE 约束
    let result = sqlx::query(
        "INSERT INTO payment_orders (user_id, sku_id, provider, state, purchase_token)
         VALUES ($1, 'diamond_60', 'google_play', 'PENDING', $2)",
    )
    .bind(user_id)
    .bind(&token)
    .execute(&pool)
    .await;

    assert!(result.is_err(), "Duplicate (provider, purchase_token) should fail");
    let err_str = result.unwrap_err().to_string();
    // PG error code 23505 = unique_violation
    assert!(
        err_str.contains("23505") || err_str.to_lowercase().contains("unique"),
        "Error should be unique constraint violation, got: {err_str}"
    );
}

// ─── W50-04: payment_orders state 类型约束 ───────────────────────────────────

#[tokio::test]
async fn w50_04_payment_order_state_type_enforced() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w50_04: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    let user_id: Uuid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (user_id, phone, password_hash, nickname)
         VALUES ($1, $2, 'hash', 'test_user2')
         ON CONFLICT (user_id) DO NOTHING",
    )
    .bind(user_id)
    .bind(format!("+1555{}", &user_id.to_string()[..8]))
    .execute(&pool)
    .await
    .unwrap();

    // 尝试插入无效的 state 值 — 应被 ENUM 类型拒绝
    let result = sqlx::query(
        "INSERT INTO payment_orders (user_id, sku_id, provider, state)
         VALUES ($1, 'diamond_60', 'google_play', 'INVALID_STATE'::payment_order_state)",
    )
    .bind(user_id)
    .execute(&pool)
    .await;

    assert!(
        result.is_err(),
        "Invalid payment_order_state should be rejected by Postgres ENUM"
    );
    let err_str = result.unwrap_err().to_string();
    // PG error: invalid input value for enum or invalid type cast
    assert!(
        err_str.contains("22P02") || err_str.to_lowercase().contains("invalid input value"),
        "Expected ENUM validation error, got: {err_str}"
    );
}

// ─── W50-05: wallet_transactions.source 列存在 ───────────────────────────────

#[tokio::test]
async fn w50_05_wallet_transactions_source_column_exists() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w50_05: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    let row = sqlx::query(
        "SELECT column_name
         FROM information_schema.columns
         WHERE table_name = 'wallet_transactions'
           AND column_name = 'source'",
    )
    .fetch_optional(&pool)
    .await
    .expect("information_schema query should succeed");

    assert!(
        row.is_some(),
        "Column wallet_transactions.source should exist after migration 011"
    );
}

// ─── W50-06: payment_skus 种子数据 diamonds 字段正确 ─────────────────────────

#[tokio::test]
async fn w50_06_sku_diamonds_values_correct() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w50_06: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    struct SkuExpect {
        sku_id: &'static str,
        diamonds: i64,
    }

    let expectations = vec![
        SkuExpect { sku_id: "diamond_60", diamonds: 60 },
        SkuExpect { sku_id: "diamond_300", diamonds: 300 },
        SkuExpect { sku_id: "diamond_980", diamonds: 980 },
        SkuExpect { sku_id: "diamond_1980", diamonds: 1980 },
        SkuExpect { sku_id: "diamond_3280", diamonds: 3280 },
    ];

    for exp in &expectations {
        let row = sqlx::query(
            "SELECT diamonds FROM payment_skus WHERE sku_id = $1",
        )
        .bind(exp.sku_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|_| panic!("SKU {} should exist", exp.sku_id));

        let diamonds: i64 = row.get("diamonds");
        assert_eq!(
            diamonds, exp.diamonds,
            "SKU {} should have {} diamonds, got {}",
            exp.sku_id, exp.diamonds, diamonds
        );
    }
}

// ─── W50-07: rtdn_processed 幂等 upsert ──────────────────────────────────────

#[tokio::test]
async fn w50_07_rtdn_processed_idempotent_upsert() {
    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] w50_07: DATABASE_URL not set");
        return;
    };
    common::run_migrations(&pool).await.unwrap();

    let message_id = format!("msg_{}", Uuid::new_v4());

    // 第一次插入
    sqlx::query(
        "INSERT INTO rtdn_processed (message_id, notification_type, processed_at)
         VALUES ($1, 'ONE_TIME_PRODUCT_NOTIFICATION', NOW())
         ON CONFLICT (message_id) DO NOTHING",
    )
    .bind(&message_id)
    .execute(&pool)
    .await
    .expect("First rtdn_processed insert should succeed");

    // 第二次相同 message_id — 幂等，不报错
    sqlx::query(
        "INSERT INTO rtdn_processed (message_id, notification_type, processed_at)
         VALUES ($1, 'ONE_TIME_PRODUCT_NOTIFICATION', NOW())
         ON CONFLICT (message_id) DO NOTHING",
    )
    .bind(&message_id)
    .execute(&pool)
    .await
    .expect("Second rtdn_processed insert should be idempotent (DO NOTHING)");

    // 验证只有一条记录
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rtdn_processed WHERE message_id = $1",
    )
    .bind(&message_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 1, "Only 1 rtdn_processed record should exist after idempotent upsert");
}
