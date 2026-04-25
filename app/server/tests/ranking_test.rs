//! 集成测试 — T-00021 魅力/财富榜单 API
//!
//! 测试用例 R01~R08 验证以下内容：
//! - R01: 填充 10 条 ZSet 数据后 limit=5 返回 Top5 且按分数降序
//! - R02: 当前用户未入榜时 me.rank=null, me.score=0
//! - R03: 当前用户第 42 名 me.rank=42
//! - R04: Top3 medal = gold/silver/bronze，第 4 名及以后 medal=null
//! - R05: type=xxx 非法返回 40003
//! - R06: limit=101 返回 40003
//! - R07: scheduler 归档函数触发后 ranking_archive:charm:day:{yesterday} 存在
//! - R08: 响应时间 <100ms（并发 20 个 top() 调用 p95 <100ms）
//!
//! 运行前提：REDIS_URL 指向可用 Redis 实例（R01~R04、R07~R08）；
//!           DATABASE_URL 指向可用 PostgreSQL 实例（R01~R04、R08）。
//! 未设置时对应测试自动跳过。

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    modules::ranking::{scheduler::do_archive_day, service::RankingService, RankingType},
};

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 获取测试用 PostgreSQL 连接池；未配置 DATABASE_URL 则返回 None（跳过）
async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

/// 获取 Redis URL；未配置 REDIS_URL 则返回 None（跳过）
fn redis_url() -> Option<String> {
    std::env::var("REDIS_URL").ok()
}

/// 创建 Redis 连接；失败则返回 None（跳过）
async fn test_redis_conn(url: &str) -> Option<redis::aio::MultiplexedConnection> {
    let client = redis::Client::open(url).ok()?;
    client.get_multiplexed_async_connection().await.ok()
}

/// 插入测试用户（phone、nickname），返回 user_id
async fn insert_test_user(pool: &PgPool, nickname: &str) -> Uuid {
    let user_id = Uuid::new_v4();
    let phone = format!("+861{}", &user_id.to_string().replace('-', "")[..10]);
    sqlx::query("INSERT INTO users (id, phone, nickname) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(&phone)
        .bind(nickname)
        .execute(pool)
        .await
        .expect("insert_test_user");
    user_id
}

/// 向指定 ZSet key 填充分数数据（批量 ZADD）
async fn zadd_scores(
    conn: &mut redis::aio::MultiplexedConnection,
    key: &str,
    entries: &[(Uuid, f64)],
) {
    use redis::AsyncCommands;
    for (uid, score) in entries {
        let _: redis::RedisResult<i64> = conn.zadd(key, uid.to_string(), *score).await;
    }
}

/// 删除 Redis 键（测试后清理）
async fn del_key(conn: &mut redis::aio::MultiplexedConnection, key: &str) {
    use redis::AsyncCommands;
    let _: redis::RedisResult<i64> = conn.del(key).await;
}

/// 生成测试用 JWT token
fn make_test_jwt(user_id: Uuid) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    use voice_room_shared::jwt::token::{encode_token, AppClaims};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let claims = AppClaims {
        sub: user_id.to_string(),
        iss: "voiceroom".to_string(),
        exp: now + 3600,
        iat: now,
    };
    encode_token(&claims, b"test-secret").expect("encode JWT")
}

// ─────────────────────────────────────────────────────────────────────────────
// R01: limit=5 返回按分数降序的 Top5
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn r01_top_n_sorted_by_score_descending() {
    let (Some(pool), Some(redis_url)) = (test_pool().await, redis_url()) else {
        eprintln!("[SKIP] R01: DATABASE_URL or REDIS_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let mut conn = test_redis_conn(&redis_url).await.expect("redis connection");

    // 构造 10 个用户，插入 PG + Redis ZSet
    let test_suffix = Uuid::new_v4().to_string().replace('-', "");
    let key = format!("ranking:charm:day:test-{}", &test_suffix[..8]);

    let mut users: Vec<(Uuid, f64)> = Vec::new();
    for i in 1..=10u32 {
        let uid = insert_test_user(&pool, &format!("RankUser_{}", i)).await;
        users.push((uid, (i * 100) as f64)); // score: 100, 200, ..., 1000
    }
    zadd_scores(&mut conn, &key, &users).await;

    let svc = RankingService::new(pool.clone(), redis_url.clone());

    // 调用时直接传入 key（便于测试隔离）
    let result = svc
        .top_by_key(&key, 5, None)
        .await
        .expect("R01: top_by_key should succeed");

    assert_eq!(result.items.len(), 5, "R01: should return exactly 5 items");

    // 验证按分数降序：item[0].score > item[1].score > ...
    for i in 0..4 {
        assert!(
            result.items[i].score >= result.items[i + 1].score,
            "R01: items should be sorted by score descending, but items[{}].score={} < items[{}].score={}",
            i, result.items[i].score, i+1, result.items[i+1].score
        );
    }
    // 验证排名从 1 开始连续递增
    for (i, item) in result.items.iter().enumerate() {
        assert_eq!(
            item.rank,
            (i + 1) as u32,
            "R01: rank should be 1-based consecutive"
        );
    }
    // 验证 Top 1 的分数最高（1000）
    assert_eq!(
        result.items[0].score, 1000,
        "R01: top1 score should be 1000"
    );

    // 清理
    del_key(&mut conn, &key).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// R02: 未入榜用户 me.rank=null, me.score=0
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn r02_viewer_not_in_list_rank_null() {
    let (Some(pool), Some(redis_url)) = (test_pool().await, redis_url()) else {
        eprintln!("[SKIP] R02: DATABASE_URL or REDIS_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let mut conn = test_redis_conn(&redis_url).await.expect("redis connection");
    let test_suffix = Uuid::new_v4().to_string().replace('-', "");
    let key = format!("ranking:charm:day:test-r02-{}", &test_suffix[..8]);

    // 填充 3 个用户
    let mut users: Vec<(Uuid, f64)> = Vec::new();
    for i in 1..=3u32 {
        let uid = insert_test_user(&pool, &format!("R02User_{}", i)).await;
        users.push((uid, (i * 100) as f64));
    }
    zadd_scores(&mut conn, &key, &users).await;

    // viewer 是一个不在榜单的新用户
    let viewer_id = insert_test_user(&pool, "R02Viewer").await;

    let svc = RankingService::new(pool.clone(), redis_url.clone());
    let result = svc
        .top_by_key(&key, 50, Some(viewer_id))
        .await
        .expect("R02: top_by_key should succeed");

    assert!(
        result.me.rank.is_none(),
        "R02: viewer not on list should have me.rank=null"
    );
    assert_eq!(
        result.me.score, 0,
        "R02: viewer not on list should have me.score=0"
    );

    del_key(&mut conn, &key).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// R03: 当前用户第 42 名 → me.rank=42
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn r03_viewer_rank_42() {
    let (Some(pool), Some(redis_url)) = (test_pool().await, redis_url()) else {
        eprintln!("[SKIP] R03: DATABASE_URL or REDIS_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let mut conn = test_redis_conn(&redis_url).await.expect("redis connection");
    let test_suffix = Uuid::new_v4().to_string().replace('-', "");
    let key = format!("ranking:charm:day:test-r03-{}", &test_suffix[..8]);

    // 插入 50 个用户，分数 1-50（升序）
    // 第 42 名 = 分数排第 9（50-42+1=9，即分数=9）
    let mut users: Vec<(Uuid, f64)> = Vec::new();
    let viewer_id = insert_test_user(&pool, "R03Viewer").await;
    let viewer_score = 9.0f64; // 50 - 42 + 1 = 9（从高到低排第 42）

    for i in 1..=50u32 {
        if i == 9 {
            // 第 42 名的位置（从高到低）是分数最小那批之一
            // 从低到高排序：分数 1..50，ZREVRANK 0-based 第 41 位 = 分数第 9 名（因为有 41 个分数更高的）
            users.push((viewer_id, viewer_score));
        } else {
            let uid = insert_test_user(&pool, &format!("R03User_{}", i)).await;
            users.push((uid, i as f64));
        }
    }
    zadd_scores(&mut conn, &key, &users).await;

    let svc = RankingService::new(pool.clone(), redis_url.clone());
    let result = svc
        .top_by_key(&key, 50, Some(viewer_id))
        .await
        .expect("R03: top_by_key should succeed");

    assert!(result.me.rank.is_some(), "R03: viewer should have a rank");
    assert_eq!(
        result.me.rank.unwrap(),
        42,
        "R03: viewer should be at rank 42"
    );
    assert_eq!(
        result.me.score, viewer_score as i64,
        "R03: viewer score should match"
    );

    del_key(&mut conn, &key).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// R04: Top3 medal = gold/silver/bronze，第 4 名 medal=null
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn r04_top3_medals() {
    let (Some(pool), Some(redis_url)) = (test_pool().await, redis_url()) else {
        eprintln!("[SKIP] R04: DATABASE_URL or REDIS_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let mut conn = test_redis_conn(&redis_url).await.expect("redis connection");
    let test_suffix = Uuid::new_v4().to_string().replace('-', "");
    let key = format!("ranking:charm:day:test-r04-{}", &test_suffix[..8]);

    // 插入 5 个用户
    let mut users: Vec<(Uuid, f64)> = Vec::new();
    for i in 1..=5u32 {
        let uid = insert_test_user(&pool, &format!("R04User_{}", i)).await;
        users.push((uid, (i * 10) as f64));
    }
    zadd_scores(&mut conn, &key, &users).await;

    let svc = RankingService::new(pool.clone(), redis_url.clone());
    let result = svc
        .top_by_key(&key, 5, None)
        .await
        .expect("R04: top_by_key should succeed");

    assert!(
        result.items.len() >= 4,
        "R04: should return at least 4 items"
    );

    // Top1 = gold
    assert_eq!(
        result.items[0].medal.as_deref(),
        Some("gold"),
        "R04: rank1 should have gold medal"
    );
    // Top2 = silver
    assert_eq!(
        result.items[1].medal.as_deref(),
        Some("silver"),
        "R04: rank2 should have silver medal"
    );
    // Top3 = bronze
    assert_eq!(
        result.items[2].medal.as_deref(),
        Some("bronze"),
        "R04: rank3 should have bronze medal"
    );
    // Top4 = null
    assert_eq!(
        result.items[3].medal, None,
        "R04: rank4 should have no medal"
    );

    del_key(&mut conn, &key).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// R05: type=xxx 非法 → HTTP 400 code=40003
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn r05_invalid_type_returns_40003() {
    let user_id = Uuid::new_v4();
    let token = make_test_jwt(user_id);

    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ranking?type=invalid&period=day&limit=10")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "R05: invalid type should return 400"
    );

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], 40003, "R05: error code should be 40003");
}

// ─────────────────────────────────────────────────────────────────────────────
// R06: limit=101 → HTTP 400 code=40003
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn r06_limit_out_of_range_returns_40003() {
    let user_id = Uuid::new_v4();
    let token = make_test_jwt(user_id);

    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ranking?type=charm&period=day&limit=101")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "R06: limit=101 should return 400"
    );

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], 40003, "R06: error code should be 40003");
}

// ─────────────────────────────────────────────────────────────────────────────
// R07: 归档函数触发后 ranking_archive:charm:day:{date} 存在
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn r07_archive_creates_ranking_archive_key() {
    let Some(redis_url) = redis_url() else {
        eprintln!("[SKIP] R07: REDIS_URL not set");
        return;
    };

    let mut conn = test_redis_conn(&redis_url).await.expect("redis connection");

    // 构建一个"昨天"的日期字符串（用 UTC）
    let yesterday = {
        let d = chrono::Utc::now() - chrono::Duration::days(1);
        d.format("%Y-%m-%d").to_string()
    };

    // 往昨天的日榜 key 写入一些数据
    let src_key = format!("ranking:charm:day:{}", yesterday);
    let uid = Uuid::new_v4().to_string();
    {
        use redis::AsyncCommands;
        let _: redis::RedisResult<i64> = conn.zadd(&src_key, &uid, 100.0f64).await;
    }

    // 执行归档
    do_archive_day(&mut conn, RankingType::Charm, &yesterday)
        .await
        .expect("R07: do_archive_day should succeed");

    // 验证 archive key 已创建
    let archive_key = format!("ranking_archive:charm:day:{}", yesterday);
    {
        use redis::AsyncCommands;
        let exists: bool = conn.exists(&archive_key).await.expect("R07: exists check");
        assert!(
            exists,
            "R07: archive key {} should exist after archive",
            archive_key
        );

        // 验证 archive key 中有数据
        let count: i64 = conn.zcard(&archive_key).await.expect("R07: zcard");
        assert!(count > 0, "R07: archive key should have data");
    }

    // 清理
    del_key(&mut conn, &src_key).await;
    del_key(&mut conn, &archive_key).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// R08: 响应时间 <100ms（并发 20 个 top_by_key 调用 p95 <100ms）
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn r08_response_time_under_100ms() {
    let (Some(pool), Some(redis_url)) = (test_pool().await, redis_url()) else {
        eprintln!("[SKIP] R08: DATABASE_URL or REDIS_URL not set");
        return;
    };
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let mut conn = test_redis_conn(&redis_url).await.expect("redis connection");
    let test_suffix = Uuid::new_v4().to_string().replace('-', "");
    let key = format!("ranking:charm:day:test-r08-{}", &test_suffix[..8]);

    // 准备 20 个用户
    let mut users: Vec<(Uuid, f64)> = Vec::new();
    for i in 1..=20u32 {
        let uid = insert_test_user(&pool, &format!("R08User_{}", i)).await;
        users.push((uid, (i * 50) as f64));
    }
    zadd_scores(&mut conn, &key, &users).await;

    let svc = Arc::new(RankingService::new(pool.clone(), redis_url.clone()));

    // 并发 20 个请求
    let mut handles = Vec::new();
    for _ in 0..20 {
        let svc_clone = Arc::clone(&svc);
        let key_clone = key.clone();
        handles.push(tokio::spawn(async move {
            let start = Instant::now();
            let _result = svc_clone.top_by_key(&key_clone, 10, None).await;
            start.elapsed()
        }));
    }

    let mut durations: Vec<Duration> = Vec::new();
    for handle in handles {
        durations.push(handle.await.expect("join handle"));
    }

    durations.sort();
    let p95_idx = (durations.len() as f64 * 0.95) as usize;
    let p95 = durations[p95_idx.min(durations.len() - 1)];

    assert!(
        p95 < Duration::from_millis(100),
        "R08: p95 latency should be <100ms, got {:?}",
        p95
    );

    del_key(&mut conn, &key).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// 额外单元测试：验证 RankingType/Period 解析
// ─────────────────────────────────────────────────────────────────────────────

/// 补充测试：缺少 type 参数 → 40003
#[tokio::test]
async fn r05b_missing_type_returns_40003() {
    let user_id = Uuid::new_v4();
    let token = make_test_jwt(user_id);

    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ranking?period=day&limit=10")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "R05b: missing type should return 400"
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], 40003, "R05b: error code should be 40003");
}

/// 补充测试：无 JWT → 401
#[tokio::test]
async fn r_auth_required() {
    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ranking?type=charm&period=day")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "ranking API should require JWT"
    );
}

/// 补充测试：limit=0 → 40003
#[tokio::test]
async fn r06b_limit_zero_returns_40003() {
    let user_id = Uuid::new_v4();
    let token = make_test_jwt(user_id);

    let app = build_app(AppState::for_test());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ranking?type=charm&period=day&limit=0")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "R06b: limit=0 should return 400"
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], 40003, "R06b: error code should be 40003");
}
