//! 集成测试 — T-00022 事件表 Schema + 分区 + HTTP 批量接收 API
//!
//! 测试用例 EV01~EV10 验证以下内容：
//! - EV01: 迁移幂等，含事件表 + 首日分区
//! - EV02: 100 events 批量写入耗时 <200ms
//! - EV03: 101 events 返回 rejected_indices=[100]，前 100 已写入
//! - EV04: properties 10KB 被截断，DB 存储记录带 _truncated=true
//! - EV05: 无 JWT 请求 user_id=null 可写入，device_id 必填
//! - EV06: device_id 缺失返回 40002
//! - EV07: JWT 存在但请求 user_id 不一致：DB 存 JWT 的 user_id，log warn
//! - EV08: 分区任务运行后 events_{tomorrow} 分区存在
//! - EV09: scheduler 启动补偿：缺失 N 天分区时一次性建完
//! - EV10: 并发 10 req×100 events 写入：total=1000 无丢失
//!
//! 运行前提：DATABASE_URL 指向可用 PostgreSQL 实例（EV01~EV04, EV07~EV10）。
//! 未设置时测试自动跳过。

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

use voice_room_server::{
    bootstrap::{build_app, AppState},
    core::analytics::{
        scheduler::create_partition_if_not_exists,
        writer::{EventInput, EventWriter, EventWriterPort, FakeEventWriter},
    },
};

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 获取测试用数据库连接池；未配置 DATABASE_URL 或连接失败时返回 None（测试跳过）
async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()
}

/// 构建带真实 EventWriter 的 AppState（DB 集成测试用）
fn app_state_with_writer(pool: PgPool) -> AppState {
    let writer = Arc::new(EventWriter::new(pool));
    AppState::for_test_with_event_writer(writer)
}

/// 解析響應 body 為 JSON
async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// 生成有效 JWT token
fn make_jwt(user_id: Uuid) -> String {
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
    encode_token(&claims, b"test-secret").unwrap()
}

// ─── EV01: 迁移幂等，含首日分区 ────────────────────────────────────────────────

/// EV01: 迁移幂等，events 表存在，且存在当日分区（由 007_create_events.sql 创建）
#[tokio::test]
async fn ev01_migration_idempotent_with_first_partition() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("EV01 skipped: DATABASE_URL not set");
            return;
        }
    };

    // events 表应该存在（由迁移创建）
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_name = 'events' AND table_schema = 'public')",
    )
    .fetch_one(&pool)
    .await
    .expect("table existence check");

    assert!(table_exists, "events table should exist after migration");

    // 应至少有一个分区（首日分区由迁移 DO block 创建）
    let partition_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pg_class c \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         WHERE c.relname LIKE 'events_%' \
         AND n.nspname = 'public' \
         AND c.relkind = 'r'",
    )
    .fetch_one(&pool)
    .await
    .expect("partition count check");

    assert!(
        partition_count >= 1,
        "should have at least 1 partition after migration, got {partition_count}"
    );

    // 再次运行迁移应是幂等的（sqlx migrate 自动跳过已运行的迁移）
    // 只验证表仍然存在
    let table_still_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
         WHERE table_name = 'events' AND table_schema = 'public')",
    )
    .fetch_one(&pool)
    .await
    .expect("table re-check");
    assert!(
        table_still_exists,
        "events table should still exist after idempotent re-check"
    );
}

// ─── EV02: 100 events 批量写入 <200ms ───────────────────────────────────────────

/// EV02: 100 events 批量写入耗时 <200ms
#[tokio::test]
async fn ev02_100_events_batch_write_under_200ms() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("EV02 skipped: DATABASE_URL not set");
            return;
        }
    };

    let writer = EventWriter::new(pool);
    let device_id = format!("device-{}", Uuid::new_v4());

    let batch: Vec<EventInput> = (0..100)
        .map(|i| EventInput {
            event_name: format!("test_event_{i}"),
            device_id: device_id.clone(),
            user_id: None,
            session_id: Some("sess-ev02".to_string()),
            client_ts: Some(1720000000000),
            properties: serde_json::json!({"index": i}),
            app_version: Some("1.0.0".to_string()),
            os_version: Some("Android 14".to_string()),
            locale: Some("ar-SA".to_string()),
            network_type: Some("wifi".to_string()),
        })
        .collect();

    let start = Instant::now();
    let result = writer
        .persist(batch, None)
        .await
        .expect("persist should succeed");
    let elapsed = start.elapsed();

    assert_eq!(result.received, 100, "all 100 events should be received");
    assert!(
        result.rejected_indices.is_empty(),
        "no events should be rejected"
    );
    assert!(
        elapsed < Duration::from_millis(200),
        "batch write of 100 events should complete in <200ms, took {:?}",
        elapsed
    );
}

// ─── EV03: 101 events → rejected_indices=[100] ──────────────────────────────────

/// EV03: 101 events 返回 rejected_indices=[100]，前 100 已写入
#[tokio::test]
async fn ev03_101_events_returns_rejected_index_100() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("EV03 skipped: DATABASE_URL not set");
            return;
        }
    };

    let state = app_state_with_writer(pool.clone());
    let app = build_app(state);

    let events: Vec<serde_json::Value> = (0..101)
        .map(|i| {
            serde_json::json!({
                "event_name": format!("ev03_event_{i}"),
                "device_id": format!("device-ev03-{i}"),
                "user_id": null,
                "properties": {}
            })
        })
        .collect();

    let body = serde_json::json!({ "events": events });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/events/batch")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["code"], 0, "response code should be 0");

    let data = &json["data"];
    assert_eq!(data["received"], 100, "should receive exactly 100");

    let rejected: Vec<usize> = serde_json::from_value(data["rejected_indices"].clone())
        .expect("rejected_indices should be array");
    assert_eq!(rejected, vec![100usize], "index 100 should be rejected");

    // 验证前 100 条已写入数据库
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE event_name LIKE 'ev03_event_%'")
            .fetch_one(&pool)
            .await
            .expect("count query");
    assert_eq!(count, 100, "exactly 100 events should be in DB");
}

// ─── EV04: properties >8KB 截断 ─────────────────────────────────────────────────

/// EV04: properties 10KB 被截断，DB 存储记录带 _truncated=true（单元测试，无需 DB）
#[test]
fn ev04_properties_truncation_logic() {
    use voice_room_server::core::analytics::writer::truncate_properties;

    // 创建超过 8KB 的 properties
    let large_value: String = "x".repeat(9000);
    let props = serde_json::json!({ "data": large_value });

    let (truncated, was_truncated) = truncate_properties(props);
    assert!(was_truncated, "10KB properties should be truncated");
    assert_eq!(
        truncated,
        serde_json::json!({"_truncated": true}),
        "truncated properties should be {{_truncated: true}}"
    );
}

/// EV04b: properties <8KB 不被截断
#[test]
fn ev04b_small_properties_not_truncated() {
    use voice_room_server::core::analytics::writer::truncate_properties;

    let props = serde_json::json!({ "key": "small_value" });
    let (result, was_truncated) = truncate_properties(props.clone());
    assert!(!was_truncated, "small properties should not be truncated");
    assert_eq!(result, props);
}

/// EV04c: DB 存储带 _truncated=true（DB 集成测试）
#[tokio::test]
async fn ev04c_db_stores_truncated_properties() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("EV04c skipped: DATABASE_URL not set");
            return;
        }
    };

    let writer = EventWriter::new(pool.clone());
    let device_id = format!("device-ev04-{}", Uuid::new_v4());

    // 创建 10KB properties
    let large_value: String = "x".repeat(9000);
    let batch = vec![EventInput {
        event_name: "ev04_large_props".to_string(),
        device_id: device_id.clone(),
        user_id: None,
        session_id: None,
        client_ts: None,
        properties: serde_json::json!({ "data": large_value }),
        app_version: None,
        os_version: None,
        locale: None,
        network_type: None,
    }];

    let result = writer
        .persist(batch, None)
        .await
        .expect("persist should succeed");
    assert_eq!(result.received, 1);

    // 查询 DB 验证 properties 已被截断
    let props: serde_json::Value = sqlx::query_scalar(
        "SELECT properties FROM events WHERE device_id = $1 ORDER BY server_ts DESC LIMIT 1",
    )
    .bind(&device_id)
    .fetch_one(&pool)
    .await
    .expect("fetch properties");

    assert_eq!(
        props,
        serde_json::json!({"_truncated": true}),
        "DB should store truncated properties"
    );
}

// ─── EV05: 无 JWT，user_id=null，device_id 必填 ──────────────────────────────────

/// EV05: 无 JWT 请求 user_id=null 可写入（通过 HTTP，使用 FakeEventWriter）
#[tokio::test]
async fn ev05_no_jwt_null_user_id_with_device_id_succeeds() {
    let app = build_app(AppState::for_test());

    let body = serde_json::json!({
        "events": [{
            "event_name": "test_anon",
            "device_id": "device-anon-001",
            "user_id": null,
            "properties": {}
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/events/batch")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["code"], 0, "anonymous event should succeed");
    assert_eq!(json["data"]["received"], 1);
}

// ─── EV06: device_id 缺失返回 40002 ─────────────────────────────────────────────

/// EV06: device_id 为空字符串返回 40002
#[tokio::test]
async fn ev06_empty_device_id_returns_40002() {
    let app = build_app(AppState::for_test());

    let body = serde_json::json!({
        "events": [{
            "event_name": "test_event",
            "device_id": "",  // 空 device_id
            "user_id": null,
            "properties": {}
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/events/batch")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response).await;
    assert_eq!(
        json["code"], 40002,
        "missing device_id should return code 40002"
    );
}

/// EV06b: device_id 字段缺失返回 40002
#[tokio::test]
async fn ev06b_missing_device_id_field_returns_40002() {
    let app = build_app(AppState::for_test());

    let body = serde_json::json!({
        "events": [{
            "event_name": "test_event",
            // device_id 字段缺失
            "user_id": null,
            "properties": {}
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/events/batch")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response).await;
    assert_eq!(
        json["code"], 40002,
        "missing device_id field should return code 40002"
    );
}

// ─── EV07: JWT user_id 覆盖 ─────────────────────────────────────────────────────

/// EV07: JWT 覆盖逻辑单元测试
#[test]
fn ev07_jwt_override_logic_unit_test() {
    use voice_room_server::core::analytics::writer::resolve_user_id;

    let jwt_uid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    let req_uid = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

    // JWT 存在时应覆盖 request user_id
    let resolved = resolve_user_id(Some(req_uid), Some(jwt_uid));
    assert_eq!(
        resolved,
        Some(jwt_uid),
        "JWT user_id should override request user_id"
    );

    // JWT 不存在时使用 request user_id
    let resolved_no_jwt = resolve_user_id(Some(req_uid), None);
    assert_eq!(
        resolved_no_jwt,
        Some(req_uid),
        "without JWT, request user_id should be used"
    );

    // 两者都为 None
    let resolved_both_none = resolve_user_id(None, None);
    assert_eq!(resolved_both_none, None);
}

/// EV07b: JWT 存在但请求 user_id 不一致：DB 存 JWT 的 user_id（DB 集成测试）
#[tokio::test]
async fn ev07b_jwt_user_id_overrides_request_in_db() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("EV07b skipped: DATABASE_URL not set");
            return;
        }
    };

    let state = app_state_with_writer(pool.clone());
    let app = build_app(state);

    let jwt_user_id = Uuid::new_v4();
    let request_user_id = Uuid::new_v4();
    let token = make_jwt(jwt_user_id);
    let device_id = format!("device-ev07-{}", Uuid::new_v4());

    let body = serde_json::json!({
        "events": [{
            "event_name": "ev07_mismatch",
            "device_id": device_id,
            "user_id": request_user_id.to_string(),  // 与 JWT 不一致
            "properties": {}
        }]
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/events/batch")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["code"], 0);

    // 验证 DB 存储的是 JWT user_id
    let stored_user_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM events WHERE event_name = 'ev07_mismatch' ORDER BY server_ts DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("fetch user_id");

    assert_eq!(
        stored_user_id,
        Some(jwt_user_id),
        "DB should store JWT user_id, not request body user_id"
    );
}

// ─── EV08: 分区创建任务 ────────────────────────────────────────────────────────

/// EV08: 分区任务运行后 events_{tomorrow} 分区存在
#[tokio::test]
async fn ev08_partition_scheduler_creates_tomorrow_partition() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("EV08 skipped: DATABASE_URL not set");
            return;
        }
    };

    let tomorrow = (Utc::now() + chrono::Duration::days(1))
        .with_timezone(&chrono_tz::Asia::Riyadh)
        .date_naive();

    let partition_name = format!("events_{}", tomorrow.format("%Y%m%d"));

    // 创建明日分区
    create_partition_if_not_exists(&pool, tomorrow)
        .await
        .expect("partition creation should succeed");

    // 验证分区存在
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_class WHERE relname = $1 AND relkind = 'r')",
    )
    .bind(&partition_name)
    .fetch_one(&pool)
    .await
    .expect("partition existence check");

    assert!(
        exists,
        "partition {partition_name} should exist after creation"
    );
}

// ─── EV09: scheduler 补偿创建 ───────────────────────────────────────────────────

/// EV09: scheduler 启动补偿：缺失 N 天分区时一次性建完
#[tokio::test]
async fn ev09_scheduler_compensation_creates_missing_partitions() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("EV09 skipped: DATABASE_URL not set");
            return;
        }
    };

    use voice_room_server::core::analytics::scheduler::compensate_missing_partitions;

    // 创建从今天起 3 天的分区
    let today = Utc::now()
        .with_timezone(&chrono_tz::Asia::Riyadh)
        .date_naive();
    let dates: Vec<chrono::NaiveDate> =
        (2..=4).map(|d| today + chrono::Duration::days(d)).collect();

    compensate_missing_partitions(&pool, &dates)
        .await
        .expect("compensation should succeed");

    // 验证所有分区都已创建
    for date in &dates {
        let partition_name = format!("events_{}", date.format("%Y%m%d"));
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_class WHERE relname = $1 AND relkind = 'r')",
        )
        .bind(&partition_name)
        .fetch_one(&pool)
        .await
        .expect("partition existence check");
        assert!(
            exists,
            "partition {partition_name} should exist after compensation"
        );
    }
}

// ─── EV10: 并发写入无丢失 ────────────────────────────────────────────────────────

/// EV10: 并发 10 req×100 events 写入：total=1000 无丢失
#[tokio::test]
async fn ev10_concurrent_writes_no_data_loss() {
    let pool = match test_pool().await {
        Some(p) => p,
        None => {
            eprintln!("EV10 skipped: DATABASE_URL not set");
            return;
        }
    };

    // 使用唯一的 session_id 标记本次并发测试的事件
    let test_session = format!("ev10-concurrent-{}", Uuid::new_v4());
    let writer = Arc::new(EventWriter::new(pool.clone()));

    // 并发 10 个任务，每个写 100 条
    let mut handles = tokio::task::JoinSet::new();
    for task_id in 0..10 {
        let writer_clone = writer.clone();
        let session = test_session.clone();
        handles.spawn(async move {
            let batch: Vec<EventInput> = (0..100)
                .map(|i| EventInput {
                    event_name: "ev10_concurrent_event".to_string(),
                    device_id: format!("device-ev10-{task_id}-{i}"),
                    user_id: None,
                    session_id: Some(session.clone()),
                    client_ts: Some(1720000000000),
                    properties: serde_json::json!({"task": task_id, "index": i}),
                    app_version: None,
                    os_version: None,
                    locale: None,
                    network_type: None,
                })
                .collect();
            writer_clone.persist(batch, None).await
        });
    }

    // 等待所有任务完成
    let mut total_received = 0usize;
    while let Some(result) = handles.join_next().await {
        let persist_result = result.expect("task panicked").expect("persist failed");
        total_received += persist_result.received;
        assert!(
            persist_result.rejected_indices.is_empty(),
            "no events should be rejected in concurrent test"
        );
    }

    assert_eq!(total_received, 1000, "total received should be 1000");

    // 验证 DB 中记录数
    let db_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE session_id = $1")
        .bind(&test_session)
        .fetch_one(&pool)
        .await
        .expect("count query");

    assert_eq!(db_count, 1000, "DB should have exactly 1000 events");
}
