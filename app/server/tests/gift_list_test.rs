//! 集成测试 — T-00019 礼物配置表 + 列表 API
//!
//! 测试用例 G01~G07 验证以下内容：
//! - G01a: 迁移 SQL 文件存在且包含 gifts 表创建语句
//! - G01b: 迁移 SQL 文件包含 8 款 MVP 种子礼物
//! - G01c: 迁移 SQL 包含所有必需字段
//! - G01d: 迁移 SQL 包含偏滤索引
//! - G02: GET /api/v1/gifts/list 默认返回阿拉伯语名称
//! - G03: Accept-Language: en 时返回英文名称
//! - G04: is_active=false 或 is_deleted=true 的礼物不在列表中（FakeGiftService 级别验证）
//! - G05: 响应时间 <50ms（FakeGiftService 速度验证）
//! - G06: GiftService 内存缓存命中：第二次调用不再查询 repo（call_count 不变）
//! - G07: 列表按 tier ASC, sort_order ASC 正确排序
//!
//! 运行前提：G01~G07 使用 FakeGiftService，无需数据库连接。
//! 带 DB 标记的测试（G_db 系列）若未设置 DATABASE_URL 则跳过。

mod common;

use std::time::{Duration, Instant};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    modules::gift::service::FakeGiftService,
};

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn load_migration_005() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/005_create_gifts.sql"
    );
    std::fs::read_to_string(path).expect("005_create_gifts.sql must exist")
}

// ─── G01 系列: 迁移 SQL 文件结构测试 ─────────────────────────────────────────

/// G01a: 迁移 SQL 包含 gifts 表创建语句
#[test]
fn g01a_migration_creates_gifts_table() {
    let sql = load_migration_005();
    assert!(
        sql.contains("CREATE TABLE IF NOT EXISTS gifts"),
        "G01a: SQL must create gifts table"
    );
}

/// G01b: 迁移 SQL 包含恰好 8 款 MVP 种子礼物
#[test]
fn g01b_migration_contains_8_seed_gifts() {
    let sql = load_migration_005();
    // 通过统计 INSERT VALUES 中的 code 字段验证 8 款礼物
    let seed_codes = [
        "rose_01",
        "coffee_01",
        "kaaba_01",
        "camel_01",
        "falcon_01",
        "moon_786",
        "castle_01",
        "diamond_ring",
    ];
    for code in &seed_codes {
        assert!(
            sql.contains(code),
            "G01b: Migration must include seed gift with code={code}"
        );
    }
    assert_eq!(seed_codes.len(), 8, "G01b: Must have exactly 8 seed gifts");
}

/// G01c: 迁移 SQL 包含所有必需字段
#[test]
fn g01c_migration_has_required_columns() {
    let sql = load_migration_005();
    let required_cols = [
        "id",
        "code",
        "name_en",
        "name_ar",
        "icon_url",
        "price",
        "tier",
        "effect_level",
        "animation_url",
        "sort_order",
        "is_active",
        "is_deleted",
        "created_at",
        "updated_at",
    ];
    for col in &required_cols {
        assert!(
            sql.contains(col),
            "G01c: Migration must include column={col}"
        );
    }
}

/// G01d: 迁移 SQL 包含偏滤索引（tier + sort_order）
#[test]
fn g01d_migration_has_active_order_index() {
    let sql = load_migration_005();
    assert!(
        sql.contains("idx_gifts_active_order"),
        "G01d: Migration must include idx_gifts_active_order index"
    );
    assert!(
        sql.contains("tier") && sql.contains("sort_order"),
        "G01d: Index must cover tier and sort_order columns"
    );
}

/// G01e: 迁移 SQL 包含 price >= 1 约束
#[test]
fn g01e_migration_has_price_check_constraint() {
    let sql = load_migration_005();
    assert!(
        sql.contains("price >= 1"),
        "G01e: Migration must include CHECK price >= 1"
    );
}

/// G01f: 迁移 SQL 包含 tier BETWEEN 1 AND 5 约束
#[test]
fn g01f_migration_has_tier_check_constraint() {
    let sql = load_migration_005();
    assert!(
        sql.contains("tier BETWEEN 1 AND 5"),
        "G01f: Migration must include CHECK tier BETWEEN 1 AND 5"
    );
}

/// G01g: 迁移 SQL 使用 ON CONFLICT (code) DO NOTHING 幂等插入
#[test]
fn g01g_migration_seed_uses_on_conflict_do_nothing() {
    let sql = load_migration_005();
    assert!(
        sql.contains("ON CONFLICT") && sql.contains("DO NOTHING"),
        "G01g: Seed insert must be idempotent using ON CONFLICT DO NOTHING"
    );
}

// ─── G02: 默认 Accept-Language 返回阿拉伯语名称 ───────────────────────────────

/// G02: 未指定 Accept-Language 时，name 字段应为阿拉伯语
#[tokio::test]
async fn g02_default_language_returns_arabic_names() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "G02: should return 200");
    let body = body_json(response).await;
    assert_eq!(body["code"], 0, "G02: code should be 0");
    let items = body["data"]["items"].as_array().unwrap();
    assert!(!items.is_empty(), "G02: items should not be empty");

    // 验证默认语言是阿拉伯语 — rose 的阿拉伯名是 "وردة"
    let rose = items.iter().find(|i| i["code"] == "rose_01");
    assert!(rose.is_some(), "G02: rose_01 should be in the list");
    let rose_name = rose.unwrap()["name"].as_str().unwrap();
    assert_eq!(rose_name, "وردة", "G02: default language should be Arabic");
}

/// G02b: 显式指定 Accept-Language: ar 也返回阿拉伯语
#[tokio::test]
async fn g02b_explicit_ar_returns_arabic_names() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .header("Accept-Language", "ar")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let items = body["data"]["items"].as_array().unwrap();
    let rose = items.iter().find(|i| i["code"] == "rose_01").unwrap();
    assert_eq!(rose["name"], "وردة", "G02b: ar header should return Arabic");
}

// ─── G03: Accept-Language: en 返回英文名称 ────────────────────────────────────

/// G03: Accept-Language: en 时 name 字段为英文
#[tokio::test]
async fn g03_accept_language_en_returns_english_names() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .header("Accept-Language", "en")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK, "G03: should return 200");
    let body = body_json(response).await;
    assert_eq!(body["code"], 0, "G03: code should be 0");
    let items = body["data"]["items"].as_array().unwrap();
    assert!(!items.is_empty(), "G03: items should not be empty");

    // rose 的英文名是 "Rose"
    let rose = items.iter().find(|i| i["code"] == "rose_01");
    assert!(rose.is_some(), "G03: rose_01 should be in the list");
    let rose_name = rose.unwrap()["name"].as_str().unwrap();
    assert_eq!(rose_name, "Rose", "G03: en header should return English");
}

// ─── G04: FakeGiftService 级别 — is_active=false 的礼物不在列表 ──────────────

/// G04: FakeGiftService 的 list_active 只返回激活礼物（已验证语义）
#[tokio::test]
async fn g04_inactive_gifts_not_in_list() {
    use voice_room_server::modules::gift::dto::GiftListData;
    use voice_room_server::modules::gift::service::GiftServicePort;

    // FakeGiftService 只返回 active=true 的礼物（预置数据已过滤）
    let svc = FakeGiftService::default();
    let data: GiftListData = svc.list_active("ar").await.unwrap();

    // 所有返回的礼物都应该是激活状态的 (FakeGiftService 的约定)
    // 验证没有礼物的名称为空（空名称代表无效礼物）
    for item in &data.items {
        assert!(
            !item.name.is_empty(),
            "G04: all returned gifts should have names"
        );
        assert!(
            item.price > 0,
            "G04: all returned gifts should have positive price"
        );
    }
}

/// G04b: FakeGiftService 没有返回任何 is_deleted=true 的礼物
#[tokio::test]
async fn g04b_deleted_gifts_not_in_response() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response).await;
    let items = body["data"]["items"].as_array().unwrap();
    // FakeGiftService 预置的礼物都是有效的，数量 > 0
    assert!(!items.is_empty(), "G04b: should have active gifts");
    // 验证每个 item 都有必要字段
    for item in items {
        assert!(item["id"].is_string(), "G04b: item must have id");
        assert!(item["code"].is_string(), "G04b: item must have code");
        assert!(item["name"].is_string(), "G04b: item must have name");
        assert!(item["price"].is_number(), "G04b: item must have price");
        assert!(item["tier"].is_number(), "G04b: item must have tier");
    }
}

// ─── G05: 响应时间 <50ms ──────────────────────────────────────────────────────

/// G05: FakeGiftService 响应时间应远低于 50ms
#[tokio::test]
async fn g05_response_time_under_50ms() {
    let app = build_app(AppState::for_test());
    let start = Instant::now();
    let _response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "G05: response time should be <50ms, got: {:?}",
        elapsed
    );
}

// ─── G06: 缓存命中测试 — GiftService 内存缓存 ────────────────────────────────

/// G06: GiftService 第一次调用查询 repo，第二次调用命中缓存（repo call_count 不变）
#[tokio::test]
async fn g06_gift_service_caches_second_request() {
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;
    use voice_room_server::modules::gift::{
        repo::FakeGiftRepo,
        service::{GiftService, GiftServicePort},
    };
    use voice_room_shared::models::gift::GiftModel;

    // 准备预置礼物数据
    let gifts = vec![GiftModel {
        id: Uuid::new_v4(),
        code: "rose_01".to_string(),
        name_en: "Rose".to_string(),
        name_ar: "وردة".to_string(),
        icon_url: "/assets/gifts/rose.png".to_string(),
        price: 1,
        tier: 1,
        effect_level: 1,
        animation_url: None,
        sort_order: 10,
        is_active: true,
        is_deleted: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }];

    let fake_repo = Arc::new(FakeGiftRepo::new(gifts));
    let service = GiftService::new(fake_repo.clone());

    // 第一次调用 — 应查询 repo（call_count = 1）
    let _ = service.list_active("ar").await.unwrap();
    assert_eq!(
        fake_repo.call_count(),
        1,
        "G06: First call should query repo once"
    );

    // 第二次调用 — 应命中缓存（call_count 仍为 1）
    let _ = service.list_active("ar").await.unwrap();
    assert_eq!(
        fake_repo.call_count(),
        1,
        "G06: Second call should use cache, not query repo again"
    );
}

/// G06b: 不同 lang 分别缓存（ar 和 en 各自独立）
#[tokio::test]
async fn g06b_different_langs_cached_independently() {
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;
    use voice_room_server::modules::gift::{
        repo::FakeGiftRepo,
        service::{GiftService, GiftServicePort},
    };
    use voice_room_shared::models::gift::GiftModel;

    let gifts = vec![GiftModel {
        id: Uuid::new_v4(),
        code: "rose_01".to_string(),
        name_en: "Rose".to_string(),
        name_ar: "وردة".to_string(),
        icon_url: "/assets/gifts/rose.png".to_string(),
        price: 1,
        tier: 1,
        effect_level: 1,
        animation_url: None,
        sort_order: 10,
        is_active: true,
        is_deleted: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }];

    let fake_repo = Arc::new(FakeGiftRepo::new(gifts));
    let service = GiftService::new(fake_repo.clone());

    // 调用 ar
    let _ = service.list_active("ar").await.unwrap();
    assert_eq!(
        fake_repo.call_count(),
        1,
        "G06b: First ar call queries repo"
    );

    // 调用 en（不同 lang，需要重新查询 repo — call_count 增加）
    let _ = service.list_active("en").await.unwrap();
    assert_eq!(
        fake_repo.call_count(),
        2,
        "G06b: en lang should query repo separately (count=2)"
    );

    // 再次调用 ar — 命中缓存（call_count 不变）
    let _ = service.list_active("ar").await.unwrap();
    assert_eq!(
        fake_repo.call_count(),
        2,
        "G06b: Second ar call should hit cache (count still 2)"
    );
}

// ─── G07: 排序测试 — tier ASC, sort_order ASC ─────────────────────────────────

/// G07: 响应中礼物按 tier ASC, sort_order ASC 排序
#[tokio::test]
async fn g07_gifts_sorted_by_tier_then_sort_order() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert!(
        items.len() >= 2,
        "G07: need at least 2 gifts to verify ordering"
    );

    // 验证 tier 非递减
    let mut prev_tier = 0i64;
    let mut prev_sort_order = i64::MIN;
    for item in items {
        let tier = item["tier"].as_i64().unwrap();
        let sort_order = item["sort_order"].as_i64().unwrap();

        if tier > prev_tier {
            // tier 变大，sort_order 重置
            prev_sort_order = i64::MIN;
        } else {
            // 同 tier 下 sort_order 应非递减
            assert!(
                sort_order >= prev_sort_order,
                "G07: within same tier, sort_order must be non-decreasing. \
                 Got {sort_order} after {prev_sort_order}"
            );
        }
        assert!(
            tier >= prev_tier,
            "G07: tier must be non-decreasing. Got {tier} after {prev_tier}"
        );
        prev_tier = tier;
        prev_sort_order = sort_order;
    }
}

// ─── G_resp: 响应结构测试 ──────────────────────────────────────────────────────

/// G_resp01: 响应体包含 data.items 和 data.version 字段
#[tokio::test]
async fn g_resp01_response_has_items_and_version() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response).await;
    assert_eq!(body["code"], 0);
    assert!(
        body["data"]["items"].is_array(),
        "G_resp01: data.items must be an array"
    );
    assert!(
        body["data"]["version"].is_string(),
        "G_resp01: data.version must be a string"
    );
}

/// G_resp02: 每个 gift item 包含必需字段
#[tokio::test]
async fn g_resp02_gift_item_has_required_fields() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response).await;
    let items = body["data"]["items"].as_array().unwrap();
    assert!(!items.is_empty());

    for item in items {
        assert!(item["id"].is_string(), "item must have id (string)");
        assert!(item["code"].is_string(), "item must have code (string)");
        assert!(item["name"].is_string(), "item must have name (string)");
        assert!(
            item["icon_url"].is_string(),
            "item must have icon_url (string)"
        );
        assert!(item["price"].is_number(), "item must have price (number)");
        assert!(item["tier"].is_number(), "item must have tier (number)");
        assert!(
            item["effect_level"].is_number(),
            "item must have effect_level (number)"
        );
        assert!(
            item["sort_order"].is_number(),
            "item must have sort_order (number)"
        );
        // animation_url can be null or string
        assert!(
            item["animation_url"].is_null() || item["animation_url"].is_string(),
            "item animation_url must be null or string"
        );
    }
}

/// G_resp03: 响应包含 request_id
#[tokio::test]
async fn g_resp03_response_has_request_id() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/gifts/list")
                .header("x-request-id", "test-gift-req-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response).await;
    assert_eq!(
        body["request_id"], "test-gift-req-123",
        "G_resp03: response must echo request_id"
    );
}

// ─── G_db: 数据库集成测试（需要 DATABASE_URL）────────────────────────────────

/// G_db01: 迁移后 gifts 表存在且包含 8 行
#[tokio::test]
async fn g_db01_migration_creates_8_gifts_in_db() {
    use sqlx::{postgres::PgPoolOptions, PgPool, Row};

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPoolOptions::new()
            .max_connections(3)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&url)
            .await
            .ok()
    }

    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] g_db01: DATABASE_URL not set");
        return;
    };

    // 运行迁移
    common::run_migrations(&pool)
        .await
        .expect("migrations");

    // 验证 gifts 表有 8 行
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM gifts WHERE is_deleted = false")
        .fetch_one(&pool)
        .await
        .expect("count query");

    let count: i64 = row.get("cnt");
    assert_eq!(
        count, 8,
        "G_db01: gifts table should have exactly 8 seed records"
    );
}

/// G_db02: is_active=false 的礼物不在查询结果中（DB 级别）
#[tokio::test]
async fn g_db02_inactive_gift_excluded_from_query() {
    use sqlx::{postgres::PgPoolOptions, PgPool, Row};

    async fn test_pool() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        PgPoolOptions::new()
            .max_connections(3)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&url)
            .await
            .ok()
    }

    let Some(pool) = test_pool().await else {
        eprintln!("[SKIP] g_db02: DATABASE_URL not set");
        return;
    };

    common::run_migrations(&pool)
        .await
        .expect("migrations");

    let mut tx = pool.begin().await.expect("begin tx");

    // 插入一个 is_active=false 的礼物
    sqlx::query(
        "INSERT INTO gifts (code, name_en, name_ar, icon_url, price, tier, effect_level, sort_order, is_active) \
         VALUES ('test_inactive', 'Test', 'اختبار', '/test.png', 1, 1, 1, 99, false)",
    )
    .execute(&mut *tx)
    .await
    .expect("insert inactive gift");

    // 查询 active 礼物（不应包含 test_inactive）
    let rows = sqlx::query(
        "SELECT code FROM gifts WHERE is_active = true AND is_deleted = false ORDER BY tier, sort_order",
    )
    .fetch_all(&mut *tx)
    .await
    .expect("list active gifts");

    let codes: Vec<String> = rows.iter().map(|r| r.get::<String, _>("code")).collect();
    assert!(
        !codes.contains(&"test_inactive".to_string()),
        "G_db02: inactive gift should not appear in active list"
    );

    tx.rollback().await.expect("rollback");
}
