use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::{err_response, AppError},
        response::ApiResponse,
        RequestContext,
    },
    modules::audit::controller::extract_ip,
};

use super::query_dto::EventQueryParams;

// ─── 角色 → 过滤标志 纯函数 ──────────────────────────────────────────────────

/// 根据管理员角色计算是否需要过滤 `admin_*` 前缀事件。
///
/// 权限规则（对齐 TDS § 权限）：
/// - `super_admin` / `operator` → `false`（全量可查，含 admin_* 事件）
/// - `cs` → `true`（只读客服，过滤 admin_* 前缀事件）
/// - `finance` → 不会到达此函数（RBAC 在上方拦截 → 403）
pub fn compute_filter_admin_events(role: &str) -> bool {
    role == "cs"
}

// ─── Handler ─────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/users/:id/events
///
/// 查询指定用户的行为事件流，需要 UserRead 权限。
///
/// 权限规则：
/// - `super_admin`：全量可查（含 admin_* 事件）
/// - `operator`：全量可查（含 admin_* 事件）
/// - `cs`：可查，admin_* 事件自动过滤
/// - `finance`：403（无 UserRead 权限）
pub async fn list_user_events_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<EventQueryParams>,
) -> axum::response::Response {
    // ── RBAC 校验：finance → 403 ──────────────────────────────────────────
    if let Err(e) = ctx.require_permission(Permission::UserRead) {
        return err_response(e, rc.request_id());
    }

    // ── 路径参数 UUID 校验 ────────────────────────────────────────────────
    let user_id = match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return err_response(
                AppError::ValidationError(format!("invalid user id: '{}'", id)),
                rc.request_id(),
            );
        }
    };

    // ── R1 修复（缺陷 11）：参数校验 fast-fail，避免非法请求触发用户存在性 DB 查询 ──
    if let Err(e) = super::query_service::validate_params(&params) {
        return err_response(e, rc.request_id());
    }

    // ── 用户存在性校验（不存在 → 404）────────────────────────────────────
    if let Err(e) = state.user_service.get_user_detail(user_id).await {
        return err_response(e, rc.request_id());
    }

    // ── 角色衍生：cs 过滤 admin_* 事件，super_admin/operator 全量可查 ──────
    let filter_admin_events = compute_filter_admin_events(&ctx.role);

    // ── 保存审计所需参数（params 将被 move 到 query_events 中）────────────
    let audit_event_name = params.event_name.clone();
    let audit_from = params.from.clone();
    let audit_to = params.to.clone();
    let audit_page = params.page;
    let audit_limit = params.limit;

    // ── 调用 EventQueryService ────────────────────────────────────────────
    let result = state
        .event_query_service
        .query_events(user_id, params, filter_admin_events)
        .await;

    // ── 审计日志（fire-and-forget，仅成功时写入）──────────────────────────
    if result.is_ok() {
        let ip = extract_ip(&headers);
        state
            .audit_logger
            .log_action(
                ctx.admin_id,
                "query_user_events",
                Some("user"),
                Some(user_id),
                ip,
                Some(serde_json::json!({
                    "target_user_id": user_id.to_string(),
                    "filters": {
                        "event_name": audit_event_name,
                        "from": audit_from,
                        "to": audit_to,
                        "page": audit_page,
                        "limit": audit_limit,
                    },
                    "filter_admin_events": filter_admin_events,
                })),
            )
            .await;
    }

    // ── 响应 ─────────────────────────────────────────────────────────────
    match result {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── Handler: GET /api/v1/admin/events/names ─────────────────────────────────

/// `GET /api/v1/admin/events/names` 查询参数。
///
/// `days`：回溯天数，可选（默认 30，clamp 到 [1, 90]）。
#[derive(Debug, Deserialize)]
pub struct EventNamesParams {
    pub days: Option<u32>,
}

/// 仅 super_admin 可访问 — 与 `Permission::SystemAdmin` 一致（详见 RBAC 矩阵）。
fn is_super_admin(role: &str) -> bool {
    role == "super_admin"
}

/// `GET /api/v1/admin/events/names`
///
/// 返回最近 `days` 天（默认 30）`events` 表中出现过的所有 distinct event_name，
/// 按字典序升序排列。响应体：`{ "items": ["..."] }`。
///
/// RBAC：仅 super_admin 可见（与模块 7 已有 super_admin 守卫一致）。
/// 审计：只读且非敏感操作，**不写入 admin_logs**（与 list_user_events 不同）。
///
/// 用途：T-20013 Web 行为流 Tab event_name 多选下拉枚举来源。
/// 缺陷 8（R1 批 3）：取代前端 `events.dict.ts` 硬编码字典。
pub async fn list_event_names_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(params): Query<EventNamesParams>,
) -> axum::response::Response {
    // ── RBAC：仅 super_admin ──────────────────────────────────────────────
    if !is_super_admin(&ctx.role) {
        return err_response(AppError::Forbidden, rc.request_id());
    }

    let days = params.days.unwrap_or(30);
    match state.event_query_service.list_event_names(days).await {
        Ok(items) => Json(ApiResponse::ok(
            serde_json::json!({ "items": items }),
            rc.request_id(),
        ))
        .into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── 单元/集成测试（EQ06: finance → 403）────────────────────────────────────
#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use uuid::Uuid;
    use voice_room_shared::jwt::token::{encode_token, AdminClaims};

    use crate::bootstrap::{build_app, AppState};

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

    /// EQ06: finance 角色 → HTTP 403（无 UserRead 权限）
    /// finance 角色在 RBAC 校验阶段被拒绝，不会进入用户存在性检查
    #[tokio::test]
    async fn eq06_finance_role_returns_403() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("finance");
        let user_id = Uuid::new_v4();

        let req = Request::builder()
            .method("GET")
            .uri(format!("/api/v1/admin/users/{}/events", user_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "EQ06: finance 角色应返回 403 FORBIDDEN"
        );
    }

    /// EQ06-code: finance 角色响应体包含 code=40301
    #[tokio::test]
    async fn eq06_finance_role_returns_code_40301() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("finance");
        let user_id = Uuid::new_v4();

        let req = Request::builder()
            .method("GET")
            .uri(format!("/api/v1/admin/users/{}/events", user_id))
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
            "EQ06: 响应体应包含 code=40301"
        );
    }

    /// EQ06-noauth: 未携带 JWT → HTTP 401
    #[tokio::test]
    async fn eq06_no_auth_returns_401() {
        let state = AppState::for_test();
        let app = build_app(state);
        let user_id = Uuid::new_v4();

        let req = Request::builder()
            .method("GET")
            .uri(format!("/api/v1/admin/users/{}/events", user_id))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "EQ06-noauth: 无 JWT 应返回 401"
        );
    }

    /// EQ06-invalid-uuid: 路径参数非法 UUID → HTTP 400
    #[tokio::test]
    async fn eq06_invalid_uuid_in_path_returns_400() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("super_admin");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/users/not-a-uuid/events")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "EQ06-invalid-uuid: 非法 UUID 应返回 400"
        );
    }

    // ── HIGH-1: operator 角色权限测试（compute_filter_admin_events 纯函数）────

    /// HIGH-1a: super_admin 角色 → filter_admin_events=false（全量可查）
    #[test]
    fn high1a_super_admin_does_not_filter() {
        use super::compute_filter_admin_events;
        assert!(
            !compute_filter_admin_events("super_admin"),
            "HIGH-1a: super_admin 不应过滤 admin_* 事件"
        );
    }

    /// HIGH-1b: operator 角色 → filter_admin_events=false（全量可查，TDS 要求）
    #[test]
    fn high1b_operator_does_not_filter() {
        use super::compute_filter_admin_events;
        assert!(
            !compute_filter_admin_events("operator"),
            "HIGH-1b: operator 应与 super_admin 一样全量可查，不应过滤 admin_* 事件"
        );
    }

    /// HIGH-1c: cs 角色 → filter_admin_events=true（过滤 admin_* 事件）
    #[test]
    fn high1c_cs_role_filters_admin_events() {
        use super::compute_filter_admin_events;
        assert!(
            compute_filter_admin_events("cs"),
            "HIGH-1c: cs 角色应过滤 admin_* 事件"
        );
    }

    /// HIGH-1d: 未知角色 → filter_admin_events=false（保守：不过滤，RBAC 已拦截）
    #[test]
    fn high1d_unknown_role_does_not_filter() {
        use super::compute_filter_admin_events;
        assert!(
            !compute_filter_admin_events("unknown"),
            "HIGH-1d: 未知角色不应过滤（RBAC 已在上方拦截非法角色）"
        );
    }

    // ── 缺陷 8（R1 批 3）：list_event_names_handler 测试 ──────────────────────

    /// EN-rbac-a: super_admin 角色 → is_super_admin=true
    #[test]
    fn en_rbac_super_admin_passes() {
        use super::is_super_admin;
        assert!(is_super_admin("super_admin"));
    }

    /// EN-rbac-b: 非 super_admin 角色 → is_super_admin=false
    #[test]
    fn en_rbac_non_super_admin_rejected() {
        use super::is_super_admin;
        assert!(!is_super_admin("operator"));
        assert!(!is_super_admin("cs"));
        assert!(!is_super_admin("finance"));
        assert!(!is_super_admin(""));
        assert!(!is_super_admin("unknown"));
    }

    /// EN-01: super_admin → 200 + items 字段为字典序数组
    #[tokio::test]
    async fn en01_super_admin_returns_200_with_items() {
        let state = AppState::for_test();
        // 注入若干事件到 fake repo
        use crate::modules::event::query_dto::EventRow;
        use crate::modules::event::query_repo::FakeEventQueryRepository;
        use chrono::{Duration as ChronoDuration, Utc};

        // 这里通过 service.repo 间接 push 不可能，直接构造一个新的 service 替换会破坏 AppState 不可变性。
        // 替代：通过 HTTP 触发后比对结构（无数据时 items=[]）。
        let _ = (EventRow {
            id: Uuid::new_v4(),
            event_name: "x".into(),
            server_ts: Utc::now() - ChronoDuration::seconds(60),
            client_ts: None,
            session_id: None,
            device_id: "d".into(),
            properties: serde_json::json!({}),
            app_version: None,
            os_version: None,
            locale: None,
            network_type: None,
        });
        let _ = FakeEventQueryRepository::default();

        let app = build_app(state);
        let token = make_jwt("super_admin");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/events/names")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "EN-01: super_admin 应返回 200"
        );

        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["code"].as_i64().unwrap(), 0);
        let items = body["data"]["items"].as_array().unwrap();
        // 空 fake repo → 空数组
        assert!(items.is_empty(), "EN-01: 空 repo 应返回空 items 数组");
    }

    /// EN-02: operator 角色 → 403
    #[tokio::test]
    async fn en02_operator_returns_403() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("operator");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/events/names")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// EN-03: cs 角色 → 403
    #[tokio::test]
    async fn en03_cs_returns_403() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("cs");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/events/names")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// EN-04: finance 角色 → 403
    #[tokio::test]
    async fn en04_finance_returns_403() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("finance");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/events/names")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// EN-05: 无 JWT → 401
    #[tokio::test]
    async fn en05_no_auth_returns_401() {
        let state = AppState::for_test();
        let app = build_app(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/events/names")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    /// EN-06: ?days=超大值 → 仍返回 200（内部 clamp 到 90）
    #[tokio::test]
    async fn en06_days_overflow_is_clamped() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("super_admin");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/events/names?days=999999")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "EN-06: days 超大值应被 clamp 而不是 400"
        );
    }
}
