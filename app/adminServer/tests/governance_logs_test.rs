//! T-10016 治理日志查询 API — 集成测试 G16-01 ~ G16-08
//!
//! 测试覆盖：
//! - G16-01: 查询按 created_at DESC
//! - G16-02: 时间窗 >90 天 → 40003
//! - G16-03: room_id 过滤生效
//! - G16-04: target_user_id 过滤生效
//! - G16-05: mutes type 过滤生效
//! - G16-06: finance 角色 → 403
//! - G16-07: 查询写入 admin_logs
//! - G16-08: 分页参数校验（page=0 → 40003；limit > 100 → 截断为 100）

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{body::Body, http::{Request, StatusCode}};
use chrono::{Duration, Utc};
use tower::ServiceExt;
use uuid::Uuid;
use voice_room_shared::jwt::token::{encode_token, AdminClaims};

use voice_room_admin_server::{
    bootstrap::{build_app, AppState},
    modules::{
        audit::repository::FakeAuditRepository,
        governance::{
            repo::{FakeGovernanceRepo, KickLogItem, MuteLogItem},
            service::{GovernanceQueryParams, GovernanceService},
        },
    },
};

// ─── JWT 工具 ────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn make_jwt(role: &str) -> String {
    let claims = AdminClaims {
        sub: Uuid::new_v4().to_string(),
        role: role.to_string(),
        iss: "voiceroom-admin".into(),
        exp: now_secs() + 3600,
        iat: now_secs(),
    };
    encode_token(&claims, "test-secret".as_bytes()).unwrap()
}

// ─── FakeGovernanceRepo 辅助构造 ──────────────────────────────────────────────

fn make_kick(room_id: Option<Uuid>, target_user_id: Option<Uuid>, secs_ago: i64) -> KickLogItem {
    KickLogItem {
        id: Uuid::new_v4(),
        room_id: room_id.unwrap_or_else(Uuid::new_v4),
        room_title: "Test Room".to_string(),
        target_user_id: target_user_id.unwrap_or_else(Uuid::new_v4),
        target_nickname: "TargetUser".to_string(),
        operator_user_id: Uuid::new_v4(),
        operator_nickname: "Operator".to_string(),
        reason: Some("test reason".to_string()),
        created_at: Utc::now() - Duration::seconds(secs_ago),
    }
}

fn make_mute(
    room_id: Option<Uuid>,
    target_user_id: Option<Uuid>,
    mute_type: &str,
    secs_ago: i64,
) -> MuteLogItem {
    MuteLogItem {
        id: Uuid::new_v4(),
        room_id: room_id.unwrap_or_else(Uuid::new_v4),
        room_title: "Test Room".to_string(),
        target_user_id: target_user_id.unwrap_or_else(Uuid::new_v4),
        target_nickname: "TargetUser".to_string(),
        operator_user_id: Uuid::new_v4(),
        operator_nickname: "Operator".to_string(),
        mute_type: mute_type.to_string(),
        duration_sec: Some(3600),
        reason: Some("spam".to_string()),
        created_at: Utc::now() - Duration::seconds(secs_ago),
    }
}

fn default_params() -> GovernanceQueryParams {
    let now = Utc::now();
    GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(7)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(1),
        limit: Some(20),
    }
}

// ─── G16-01: 查询结果按 created_at DESC 排序 ─────────────────────────────────

/// G16-01: kicks 查询结果按 created_at DESC 排序（最新在最前）
#[tokio::test]
async fn g16_01_kicks_ordered_by_created_at_desc() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());

    // 插入顺序：最旧的先插，最新的后插
    repo.push_kick(make_kick(None, None, 300)); // 5分钟前
    repo.push_kick(make_kick(None, None, 60));  // 1分钟前
    repo.push_kick(make_kick(None, None, 120)); // 2分钟前

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let resp = service
        .query_kicks(default_params(), Uuid::new_v4(), None)
        .await
        .unwrap();

    assert_eq!(resp.items.len(), 3, "G16-01: 应返回 3 条 kicks");
    // 验证按 created_at DESC：第一条 created_at >= 第二条 >= 第三条
    assert!(
        resp.items[0].created_at >= resp.items[1].created_at,
        "G16-01: items[0] 应比 items[1] 更新"
    );
    assert!(
        resp.items[1].created_at >= resp.items[2].created_at,
        "G16-01: items[1] 应比 items[2] 更新"
    );
}

/// G16-01b: mutes 查询结果按 created_at DESC 排序
#[tokio::test]
async fn g16_01b_mutes_ordered_by_created_at_desc() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());

    repo.push_mute(make_mute(None, None, "mic", 300));
    repo.push_mute(make_mute(None, None, "chat", 60));
    repo.push_mute(make_mute(None, None, "mic", 120));

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let resp = service
        .query_mutes(default_params(), Uuid::new_v4(), None)
        .await
        .unwrap();

    assert_eq!(resp.items.len(), 3, "G16-01b: 应返回 3 条 mutes");
    assert!(
        resp.items[0].created_at >= resp.items[1].created_at,
        "G16-01b: items[0] 应比 items[1] 更新"
    );
}

// ─── G16-02: 时间窗 >90 天 → ValidationError(40003) ─────────────────────────

/// G16-02: from 到 to 超过 90 天 → 返回 ValidationError
#[tokio::test]
async fn g16_02_time_window_over_90_days_returns_validation_error() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(91)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(1),
        limit: Some(20),
    };

    let result = service.query_kicks(params, Uuid::new_v4(), None).await;
    assert!(
        matches!(
            result,
            Err(voice_room_admin_server::common::error::AppError::ValidationError(_))
        ),
        "G16-02: 时间窗 >90 天应返回 ValidationError，got: {:?}",
        result
    );
}

/// G16-02b: 刚好 90 天 → 正常返回（边界值合法）
#[tokio::test]
async fn g16_02b_time_window_exactly_90_days_is_ok() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(90)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(1),
        limit: Some(20),
    };

    let result = service.query_kicks(params, Uuid::new_v4(), None).await;
    assert!(result.is_ok(), "G16-02b: 刚好 90 天应合法，got: {:?}", result);
}

// ─── G16-03: room_id 过滤生效 ─────────────────────────────────────────────────

/// G16-03: 指定 room_id 后只返回该房间的踢人记录
#[tokio::test]
async fn g16_03_room_id_filter_works() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());

    let target_room = Uuid::new_v4();
    let other_room = Uuid::new_v4();

    repo.push_kick(make_kick(Some(target_room), None, 100));
    repo.push_kick(make_kick(Some(target_room), None, 200));
    repo.push_kick(make_kick(Some(other_room), None, 150));

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: Some(target_room),
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(1)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(1),
        limit: Some(20),
    };

    let resp = service.query_kicks(params, Uuid::new_v4(), None).await.unwrap();
    assert_eq!(resp.total, 2, "G16-03: room_id 过滤后应只有 2 条");
    assert!(
        resp.items.iter().all(|i| i.room_id == target_room),
        "G16-03: 所有结果的 room_id 应匹配"
    );
}

// ─── G16-04: target_user_id 过滤生效 ─────────────────────────────────────────

/// G16-04: 指定 target_user_id 后只返回对应用户的踢人记录
#[tokio::test]
async fn g16_04_target_user_id_filter_works() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());

    let target_user = Uuid::new_v4();
    let other_user = Uuid::new_v4();

    repo.push_kick(make_kick(None, Some(target_user), 100));
    repo.push_kick(make_kick(None, Some(other_user), 200));
    repo.push_kick(make_kick(None, Some(target_user), 300));

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: Some(target_user),
        operator_user_id: None,
        from: Some((now - Duration::days(1)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(1),
        limit: Some(20),
    };

    let resp = service.query_kicks(params, Uuid::new_v4(), None).await.unwrap();
    assert_eq!(resp.total, 2, "G16-04: target_user_id 过滤后应只有 2 条");
    assert!(
        resp.items.iter().all(|i| i.target_user_id == target_user),
        "G16-04: 所有结果的 target_user_id 应匹配"
    );
}

// ─── G16-05: mutes type 过滤生效 ─────────────────────────────────────────────

/// G16-05: 指定 type=mic 后只返回 mic 类型的禁言记录
#[tokio::test]
async fn g16_05_mute_type_filter_works() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());

    repo.push_mute(make_mute(None, None, "mic", 100));
    repo.push_mute(make_mute(None, None, "chat", 200));
    repo.push_mute(make_mute(None, None, "mic", 300));
    repo.push_mute(make_mute(None, None, "chat", 150));

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(1)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: Some("mic".to_string()),
        page: Some(1),
        limit: Some(20),
    };

    let resp = service.query_mutes(params, Uuid::new_v4(), None).await.unwrap();
    assert_eq!(resp.total, 2, "G16-05: type=mic 过滤后应只有 2 条");
    assert!(
        resp.items.iter().all(|i| i.mute_type == "mic"),
        "G16-05: 所有结果的 mute_type 应为 mic"
    );
}

/// G16-05b: 指定 type=chat 后只返回 chat 类型的禁言记录
#[tokio::test]
async fn g16_05b_mute_type_chat_filter_works() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());

    repo.push_mute(make_mute(None, None, "mic", 100));
    repo.push_mute(make_mute(None, None, "chat", 200));

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(1)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: Some("chat".to_string()),
        page: Some(1),
        limit: Some(20),
    };

    let resp = service.query_mutes(params, Uuid::new_v4(), None).await.unwrap();
    assert_eq!(resp.total, 1, "G16-05b: type=chat 过滤后应只有 1 条");
    assert_eq!(resp.items[0].mute_type, "chat", "G16-05b: mute_type 应为 chat");
}

// ─── G16-06: finance 角色 → HTTP 403 ─────────────────────────────────────────

/// G16-06a: finance 角色访问 kicks 接口 → 403
#[tokio::test]
async fn g16_06a_finance_role_forbidden_on_kicks() {
    let state = AppState::for_test();
    let app = build_app(state);
    let token = make_jwt("finance");

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/governance/kicks")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "G16-06a: finance 角色访问 kicks 应返回 403"
    );
}

/// G16-06b: finance 角色访问 mutes 接口 → 403
#[tokio::test]
async fn g16_06b_finance_role_forbidden_on_mutes() {
    let state = AppState::for_test();
    let app = build_app(state);
    let token = make_jwt("finance");

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/governance/mutes")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "G16-06b: finance 角色访问 mutes 应返回 403"
    );
}

/// G16-06c: cs 角色可以访问 kicks 接口（不是 403）
#[tokio::test]
async fn g16_06c_cs_role_can_access_kicks() {
    let state = AppState::for_test();
    let app = build_app(state);
    let token = make_jwt("cs");

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/governance/kicks")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_ne!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "G16-06c: cs 角色访问 kicks 不应返回 403"
    );
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "G16-06c: cs 角色访问 kicks 应返回 200"
    );
}

/// G16-06d: operator 角色可以访问 mutes 接口
#[tokio::test]
async fn g16_06d_operator_role_can_access_mutes() {
    let state = AppState::for_test();
    let app = build_app(state);
    let token = make_jwt("operator");

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/governance/mutes")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "G16-06d: operator 角色访问 mutes 应返回 200"
    );
}

/// G16-06e: finance 角色响应体包含 code=40301
#[tokio::test]
async fn g16_06e_finance_role_returns_code_40301() {
    let state = AppState::for_test();
    let app = build_app(state);
    let token = make_jwt("finance");

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/governance/kicks")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(
        body["code"].as_i64().unwrap(),
        40301,
        "G16-06e: finance 403 响应体 code 应为 40301"
    );
}

// ─── G16-07: 查询写入 admin_logs ──────────────────────────────────────────────

/// G16-07a: query_kicks 成功后写入 admin_logs，action=query_kick_records
#[tokio::test]
async fn g16_07a_query_kicks_writes_audit_log() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let admin_id = Uuid::new_v4();

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(
            audit_repo.clone(),
        )),
    );

    service
        .query_kicks(default_params(), admin_id, Some("1.2.3.4".to_string()))
        .await
        .unwrap();

    let logs = audit_repo.get_logs();
    assert_eq!(logs.len(), 1, "G16-07a: 应写入 1 条审计日志");
    assert_eq!(
        logs[0].action, "query_kick_records",
        "G16-07a: action 应为 query_kick_records"
    );
    assert_eq!(
        logs[0].admin_id, admin_id,
        "G16-07a: admin_id 应匹配"
    );
}

/// G16-07b: query_mutes 成功后写入 admin_logs，action=query_mute_records
#[tokio::test]
async fn g16_07b_query_mutes_writes_audit_log() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let admin_id = Uuid::new_v4();

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(
            audit_repo.clone(),
        )),
    );

    service
        .query_mutes(default_params(), admin_id, Some("10.0.0.1".to_string()))
        .await
        .unwrap();

    let logs = audit_repo.get_logs();
    assert_eq!(logs.len(), 1, "G16-07b: 应写入 1 条审计日志");
    assert_eq!(
        logs[0].action, "query_mute_records",
        "G16-07b: action 应为 query_mute_records"
    );
}

/// G16-07c: 审计日志中包含 filters detail 字段
#[tokio::test]
async fn g16_07c_audit_log_contains_filters_detail() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let admin_id = Uuid::new_v4();
    let room_id = Uuid::new_v4();

    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(
            audit_repo.clone(),
        )),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: Some(room_id),
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(7)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(1),
        limit: Some(20),
    };

    service.query_kicks(params, admin_id, None).await.unwrap();

    let logs = audit_repo.get_logs();
    let detail = logs[0].detail.as_ref().expect("G16-07c: detail 不应为 None");
    assert!(
        detail.get("filters").is_some(),
        "G16-07c: detail 应包含 filters 字段"
    );
}

// ─── G16-08: 分页参数校验 ─────────────────────────────────────────────────────

/// G16-08a: page=0 → ValidationError(40003)
#[tokio::test]
async fn g16_08a_page_zero_returns_validation_error() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(7)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(0), // ← 非法
        limit: Some(20),
    };

    let result = service.query_kicks(params, Uuid::new_v4(), None).await;
    assert!(
        matches!(
            result,
            Err(voice_room_admin_server::common::error::AppError::ValidationError(_))
        ),
        "G16-08a: page=0 应返回 ValidationError，got: {:?}",
        result
    );
}

/// G16-08b: limit > 100 → 截断为 100（不返回错误）
#[tokio::test]
async fn g16_08b_limit_over_100_is_clamped_to_100() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(7)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(1),
        limit: Some(200), // ← 超过 100，应截断
    };

    let result = service.query_kicks(params, Uuid::new_v4(), None).await;
    assert!(
        result.is_ok(),
        "G16-08b: limit=200 不应返回错误，got: {:?}",
        result
    );
    let resp = result.unwrap();
    assert_eq!(
        resp.limit, 100,
        "G16-08b: limit 应被截断为 100，得到 {}",
        resp.limit
    );
}

/// G16-08c: limit=100（边界值合法）不报错
#[tokio::test]
async fn g16_08c_limit_exactly_100_is_ok() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(7)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: Some(1),
        limit: Some(100),
    };

    let result = service.query_kicks(params, Uuid::new_v4(), None).await;
    assert!(result.is_ok(), "G16-08c: limit=100 应合法，got: {:?}", result);
    assert_eq!(result.unwrap().limit, 100, "G16-08c: limit 应为 100");
}

/// G16-08d: page=1（合法下界）不报错
#[tokio::test]
async fn g16_08d_page_one_is_ok() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let result = service
        .query_kicks(default_params(), Uuid::new_v4(), None)
        .await;
    assert!(result.is_ok(), "G16-08d: page=1 应合法，got: {:?}", result);
}

/// G16-08e: 默认分页（page/limit 均为 None）使用默认值（page=1, limit=20）
#[tokio::test]
async fn g16_08e_default_pagination_values() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let now = Utc::now();
    let params = GovernanceQueryParams {
        room_id: None,
        target_user_id: None,
        operator_user_id: None,
        from: Some((now - Duration::days(7)).to_rfc3339()),
        to: Some(now.to_rfc3339()),
        mute_type: None,
        page: None,   // 应默认为 1
        limit: None,  // 应默认为 20
    };

    let resp = service
        .query_kicks(params, Uuid::new_v4(), None)
        .await
        .unwrap();

    assert_eq!(resp.page, 1, "G16-08e: 默认 page 应为 1");
    assert_eq!(resp.limit, 20, "G16-08e: 默认 limit 应为 20");
}

// ─── 额外边界用例 ─────────────────────────────────────────────────────────────

/// 未携带 JWT → 访问 kicks 返回 401
#[tokio::test]
async fn no_auth_returns_401_on_kicks() {
    let state = AppState::for_test();
    let app = build_app(state);

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/governance/kicks")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "无 JWT 应返回 401"
    );
}

/// super_admin 角色可以访问 kicks 接口
#[tokio::test]
async fn super_admin_can_access_kicks() {
    let state = AppState::for_test();
    let app = build_app(state);
    let token = make_jwt("super_admin");

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/admin/governance/kicks")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "super_admin 应能访问 kicks"
    );
}

/// 空结果时 total=0, items=[]
#[tokio::test]
async fn empty_result_returns_total_zero() {
    let repo = Arc::new(FakeGovernanceRepo::default());
    let audit_repo = Arc::new(FakeAuditRepository::default());
    let service = GovernanceService::new(
        repo,
        Arc::new(voice_room_admin_server::modules::audit::service::AuditLogger::new(audit_repo)),
    );

    let resp = service
        .query_kicks(default_params(), Uuid::new_v4(), None)
        .await
        .unwrap();

    assert_eq!(resp.total, 0, "空 repo 应返回 total=0");
    assert!(resp.items.is_empty(), "空 repo 应返回空 items");
}
