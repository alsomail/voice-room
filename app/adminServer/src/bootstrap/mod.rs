use std::sync::Arc;

use axum::{middleware, routing::{get, post}, Router};

use crate::{
    infrastructure::logging::request_context_middleware,
    modules::audit::{
        controller::list_logs_handler,
        repository::AuditRepository,
        service::{AuditLogger, AuditService},
    },
    modules::auth::{controller::login_handler, repository::AdminLogRepository, AdminAuthService},
    modules::auth::repository::AdminRepository,
    modules::event::publisher::EventPublisher,
    modules::room::{
        controller::{force_close_room_handler, get_room_detail_handler, list_rooms_handler},
        AdminRoomRepository, AdminRoomService,
    },
    modules::stats::{
        controller::stats_overview_handler,
        AdminStatsRepository, AdminStatsService,
    },
    modules::user::{
        controller::{ban_user_handler, get_user_handler, list_users_handler},
        AdminUserRepository, AdminUserService,
    },
};

// ─── AppState ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AdminAuthService>,
    pub room_service: Arc<AdminRoomService>,
    pub stats_service: Arc<AdminStatsService>,
    pub user_service: Arc<AdminUserService>,
    pub jwt_secret: String,
    pub event_publisher: Arc<dyn EventPublisher>,
    /// 审计日志写入器（T-10012）
    pub audit_logger: Arc<AuditLogger>,
    /// 审计日志查询服务（T-10012）
    pub audit_service: Arc<AuditService>,
}

impl AppState {
    pub fn new(
        admin_repo: Arc<dyn AdminRepository>,
        log_repo: Arc<dyn AdminLogRepository>,
        room_repo: Arc<dyn AdminRoomRepository>,
        user_repo: Arc<dyn AdminUserRepository>,
        stats_repo: Arc<dyn AdminStatsRepository>,
        jwt_secret: String,
        event_publisher: Arc<dyn EventPublisher>,
        audit_repo: Arc<dyn AuditRepository>,
    ) -> Self {
        let auth_service = Arc::new(AdminAuthService::new(
            admin_repo,
            log_repo,
            jwt_secret.clone(),
        ));
        let room_service = Arc::new(AdminRoomService::new(room_repo, event_publisher.clone()));
        let stats_service = Arc::new(AdminStatsService::new(stats_repo));
        let user_service = Arc::new(AdminUserService::new(user_repo, event_publisher.clone()));
        let audit_logger = Arc::new(AuditLogger::new(audit_repo.clone()));
        let audit_service = Arc::new(AuditService::new(audit_repo));
        Self {
            auth_service,
            room_service,
            stats_service,
            user_service,
            jwt_secret,
            event_publisher,
            audit_logger,
            audit_service,
        }
    }

    /// 用于单元/集成测试的空状态（无预置管理员、无预置房间、无预置用户）。
    #[cfg(test)]
    pub fn for_test() -> Self {
        use crate::modules::audit::repository::FakeAuditRepository;
        use crate::modules::auth::repository::{FakeAdminLogRepository, FakeAdminRepository};
        use crate::modules::event::publisher::NoopEventPublisher;
        use crate::modules::room::repository::FakeAdminRoomRepository;
        use crate::modules::stats::FakeAdminStatsRepository;
        use crate::modules::user::repository::FakeAdminUserRepository;
        Self::new(
            Arc::new(FakeAdminRepository::default()),
            Arc::new(FakeAdminLogRepository::default()),
            Arc::new(FakeAdminRoomRepository::default()),
            Arc::new(FakeAdminUserRepository::default()),
            Arc::new(FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        )
    }
}

// ─── Router ──────────────────────────────────────────────────────────────────

/// 构建 Admin Server 路由。
pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/admin/login", post(login_handler))
        .route("/api/v1/admin/rooms", get(list_rooms_handler))
        .route(
            "/api/v1/admin/rooms/{id}",
            get(get_room_detail_handler).delete(force_close_room_handler),
        )
        .route("/api/v1/admin/users", get(list_users_handler))
        .route("/api/v1/admin/users/{id}", get(get_user_handler))
        .route("/api/v1/admin/users/{id}/ban", post(ban_user_handler))
        .route("/api/v1/admin/stats/overview", get(stats_overview_handler))
        .route("/api/v1/admin/logs", get(list_logs_handler))
        .layer(middleware::from_fn(request_context_middleware))
        .with_state(state)
}

// ─── 测试专用路由（T-10003）──────────────────────────────────────────────────

/// 构建含 RBAC 测试路由的 App（仅用于集成测试）。
#[cfg(test)]
pub fn build_test_app_rbac(state: AppState) -> Router {
    use axum::{routing::get, Extension, Json};
    use crate::common::{
        auth::{AdminAuthContext, Permission},
        error::err_response,
        RequestContext,
    };

    /// 任意合法 JWT 即可访问，返回 admin_id 和 role（用于验证注入）
    async fn protected_handler(
        ctx: AdminAuthContext,
        Extension(req_ctx): Extension<RequestContext>,
    ) -> impl axum::response::IntoResponse {
        Json(serde_json::json!({
            "code": 0,
            "admin_id": ctx.admin_id.to_string(),
            "role": ctx.role,
            "request_id": req_ctx.request_id(),
        }))
    }

    /// 需要 FinanceRead 权限（operator/cs/未知角色均被拒绝）
    async fn finance_handler(
        ctx: AdminAuthContext,
        Extension(req_ctx): Extension<RequestContext>,
    ) -> axum::response::Response {
        match ctx.require_permission(Permission::FinanceRead) {
            Ok(_) => Json(serde_json::json!({"code": 0})).into_response(),
            Err(e) => err_response(e, req_ctx.request_id()),
        }
    }

    /// 需要 UserWrite 权限（cs 不可访问此端点）
    async fn user_write_handler(
        ctx: AdminAuthContext,
        Extension(req_ctx): Extension<RequestContext>,
    ) -> axum::response::Response {
        match ctx.require_permission(Permission::UserWrite) {
            Ok(_) => Json(serde_json::json!({"code": 0})).into_response(),
            Err(e) => err_response(e, req_ctx.request_id()),
        }
    }

    use axum::response::IntoResponse;

    Router::new()
        .route("/api/v1/admin/login", post(login_handler))
        .route("/api/v1/admin/test/protected", get(protected_handler))
        .route("/api/v1/admin/test/finance", get(finance_handler))
        .route("/api/v1/admin/test/user-write", post(user_write_handler))
        .layer(middleware::from_fn(request_context_middleware))
        .with_state(state)
}

// ─── 集成测试（T-10002 HTTP 层验收用例）─────────────────────────────────────
//
// TDD 工作流：
//   RED  — controller / bootstrap 骨架不存在时，以下测试无法编译/运行
//   GREEN — 实现 login_handler + build_app 后，所有断言通过
//
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::Utc;
    use tower::ServiceExt;
    use uuid::Uuid;
    use voice_room_shared::models::AdminModel;

    use super::*;
    use crate::modules::audit::repository::FakeAuditRepository;
    use crate::modules::auth::repository::{FakeAdminLogRepository, FakeAdminRepository};
    use crate::modules::event::publisher::NoopEventPublisher;

    // ── 测试辅助 ──────────────────────────────────────────────────────────────

    /// 低 cost(4) 快速 bcrypt 哈希，仅用于测试。
    fn test_hash(password: &str) -> String {
        bcrypt::hash(password, 4).unwrap()
    }

    fn make_admin(username: &str, password: &str, role: &str, is_active: bool) -> AdminModel {
        AdminModel {
            id: Uuid::new_v4(),
            username: username.to_string(),
            password_hash: test_hash(password),
            role: role.to_string(),
            display_name: Some("集成测试运营".to_string()),
            is_active,
            last_login_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 构建带预置管理员的 App
    fn app_with_admin(admin: AdminModel) -> axum::Router {
        use crate::modules::room::repository::FakeAdminRoomRepository;
        use crate::modules::stats::FakeAdminStatsRepository;
        use crate::modules::user::repository::FakeAdminUserRepository;
        let admin_repo = Arc::new(FakeAdminRepository::default());
        admin_repo.seed(admin);
        build_app(AppState::new(
            admin_repo,
            Arc::new(FakeAdminLogRepository::default()),
            Arc::new(FakeAdminRoomRepository::default()),
            Arc::new(FakeAdminUserRepository::default()),
            Arc::new(FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ))
    }

    /// 构建空 App（无管理员）
    fn app_empty() -> axum::Router {
        build_app(AppState::for_test())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // ── T-10003 测试专用 helper ───────────────────────────────────────────────

    use std::time::{SystemTime, UNIX_EPOCH};
    use voice_room_shared::jwt::token::{encode_token, AdminClaims};

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn make_jwt(secret: &str, role: &str) -> String {
        let claims = AdminClaims {
            sub: Uuid::new_v4().to_string(),
            role: role.to_string(),
            iss: "voiceroom-admin".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        encode_token(&claims, secret.as_bytes()).unwrap()
    }

    fn make_expired_jwt(secret: &str) -> String {
        let claims = AdminClaims {
            sub: Uuid::new_v4().to_string(),
            role: "operator".into(),
            iss: "voiceroom-admin".into(),
            exp: now_secs() - 120, // 已过期
            iat: now_secs() - 180,
        };
        encode_token(&claims, secret.as_bytes()).unwrap()
    }

    fn rbac_app() -> axum::Router {
        build_test_app_rbac(AppState::for_test())
    }

    // ── I-01: 账号不存在 → HTTP 401，错误码 40106 ───────────────────────────
    //
    // RED: 路由不存在时此测试返回 404；controller 未实现时返回错误类型不符
    // GREEN: 实现 login_handler 后通过
    #[tokio::test]
    async fn post_login_unknown_username_returns_401_40106() {
        let app = app_empty();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"nobody","password":"anything"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(
            json["code"].as_i64().unwrap(),
            40106,
            "账号不存在的错误码必须是 40106"
        );
    }

    // ── I-02: 密码错误 → HTTP 401，错误码 40106 ─────────────────────────────
    #[tokio::test]
    async fn post_login_wrong_password_returns_401_40106() {
        let admin = make_admin("op1", "correct_pass", "operator", true);
        let app = app_with_admin(admin);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"username":"op1","password":"wrong_pass"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(
            json["code"].as_i64().unwrap(),
            40106,
            "密码错误的错误码必须是 40106"
        );
    }

    // ── I-03: 正确凭证 → HTTP 200，含 token 和 admin 信息 ─────────────────
    #[tokio::test]
    async fn post_login_success_returns_200_with_token_and_admin() {
        let admin = make_admin("op1", "pass1234", "operator", true);
        let app = app_with_admin(admin);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/login")
                    .header("content-type", "application/json")
                    .header("x-request-id", "test-req-1")
                    .body(Body::from(r#"{"username":"op1","password":"pass1234"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "成功时 code 必须为 0");
        assert_eq!(json["request_id"], "test-req-1", "响应必须回传 request_id");

        let token = json["data"]["token"].as_str().unwrap();
        assert!(!token.is_empty(), "token 不能为空");

        assert_eq!(
            json["data"]["expires_in"].as_u64().unwrap(),
            604800,
            "有效期必须为 604800 秒（7 天）"
        );
        assert_eq!(json["data"]["admin"]["username"], "op1");
        assert_eq!(json["data"]["admin"]["role"], "operator");
        assert!(
            json["data"]["admin"]["last_login_at"].as_str().is_some(),
            "last_login_at 必须在成功响应中返回"
        );
    }

    // ── I-04: 账号被禁用 → HTTP 403，错误码 40302 ───────────────────────────
    //
    // RED: service 不检查 is_active 时此测试返回 401 而非 403
    // GREEN: service Step 3 实现后通过
    #[tokio::test]
    async fn post_login_disabled_account_returns_403_40302() {
        let admin = make_admin("disabled_op", "pass1234", "operator", false); // is_active = false
        let app = app_with_admin(admin);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"disabled_op","password":"pass1234"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(
            json["code"].as_i64().unwrap(),
            40302,
            "禁用账号的错误码必须是 40302"
        );
    }

    // ── I-05: 缺少 Content-Type → 415 (Axum 自动处理) ──────────────────────
    #[tokio::test]
    async fn post_login_missing_content_type_returns_415() {
        let app = app_empty();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/login")
                    // 不设置 content-type
                    .body(Body::from(r#"{"username":"op1","password":"pass"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Axum 对无 Content-Type 的 JSON body 返回 415 Unsupported Media Type
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    // ── I-06: 响应头回传 X-Request-Id（由中间件注入）───────────────────────
    #[tokio::test]
    async fn login_response_header_contains_request_id() {
        let admin = make_admin("op1", "pass1234", "operator", true);
        let app = app_with_admin(admin);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/login")
                    .header("content-type", "application/json")
                    .header("x-request-id", "echo-this-id")
                    .body(Body::from(r#"{"username":"op1","password":"pass1234"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        let header_val = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(header_val, "echo-this-id", "响应头必须回传 X-Request-Id");
    }

    // ── I-07: IP 地址从 X-Forwarded-For 提取并写入日志 ─────────────────────
    #[tokio::test]
    async fn post_login_records_client_ip_from_x_forwarded_for() {
        use crate::modules::room::repository::FakeAdminRoomRepository;
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let log_repo_clone = log_repo.clone();

        let admin = make_admin("op1", "pass1234", "operator", true);
        admin_repo.seed(admin);

        let app = build_app(AppState::new(
            admin_repo,
            log_repo,
            Arc::new(FakeAdminRoomRepository::default()),
            Arc::new(crate::modules::user::repository::FakeAdminUserRepository::default()),
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ));

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/login")
                .header("content-type", "application/json")
                .header("x-forwarded-for", "10.0.0.1, 172.16.0.1")
                .body(Body::from(r#"{"username":"op1","password":"pass1234"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

        let logs = log_repo_clone.get_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(
            logs[0].ip_address,
            Some("10.0.0.1".to_string()),
            "必须提取 X-Forwarded-For 的第一个 IP"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10003 集成测试：JWT 中间件 + RBAC
    // ════════════════════════════════════════════════════════════════════════

    // ── I-08: 无 token → 401，request_id 不为空 ─────────────────────────────
    #[tokio::test]
    async fn jwt_no_token_returns_401_with_request_id() {
        let app = rbac_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/protected")
                    .header("x-request-id", "req-no-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "无 token 错误码必须是 40101");
        assert_eq!(
            json["request_id"].as_str().unwrap(),
            "req-no-token",
            "拒绝响应必须包含 request_id"
        );
    }

    // ── I-08b: 无 token（无 X-Request-Id）→ 401，request_id 非空（自动生成）─
    #[tokio::test]
    async fn jwt_no_token_auto_generated_request_id_not_empty() {
        let app = rbac_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/protected")
                    // 不传 X-Request-Id，中间件自动生成
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        let rid = json["request_id"].as_str().unwrap_or("");
        assert!(!rid.is_empty(), "自动生成的 request_id 不能为空");
    }

    // ── I-09: 签名无效 → 401 (40101) ────────────────────────────────────────
    #[tokio::test]
    async fn jwt_invalid_signature_returns_401() {
        let bad_token = make_jwt("wrong-secret", "operator");
        let app = rbac_app(); // app uses "test-secret"

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/protected")
                    .header("Authorization", format!("Bearer {bad_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101);
    }

    // ── I-10: token 过期 → 401 (40102) ──────────────────────────────────────
    #[tokio::test]
    async fn jwt_expired_token_returns_401_40102() {
        let token = make_expired_jwt("test-secret");
        let app = rbac_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/protected")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40102, "过期 token 错误码必须是 40102");
    }

    // ── I-11: 合法 token → 200，注入 admin_id 和 role ─────────────────────
    #[tokio::test]
    async fn jwt_valid_token_returns_200_with_admin_id_and_role() {
        let token = make_jwt("test-secret", "operator");
        let app = rbac_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/protected")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0);
        assert_eq!(json["role"].as_str().unwrap(), "operator");
        // admin_id 是有效 UUID
        let admin_id_str = json["admin_id"].as_str().unwrap();
        assert!(uuid::Uuid::parse_str(admin_id_str).is_ok(), "admin_id 必须是合法 UUID");
    }

    // ── I-12: super_admin 可访问 finance 端点 ────────────────────────────────
    #[tokio::test]
    async fn rbac_super_admin_can_access_finance_endpoint() {
        let token = make_jwt("test-secret", "super_admin");
        let app = rbac_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/finance")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // ── I-13: operator 不能访问 finance 端点 → 403 (40301) ──────────────────
    #[tokio::test]
    async fn rbac_operator_cannot_access_finance_returns_403() {
        let token = make_jwt("test-secret", "operator");
        let app = rbac_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/finance")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(
            json["code"].as_i64().unwrap(),
            40301,
            "operator 访问 finance 端点必须返回 40301"
        );
    }

    // ── I-14: cs 不能执行用户写操作 → 403 (40301) ──────────────────────────
    #[tokio::test]
    async fn rbac_cs_cannot_access_user_write_returns_403() {
        let token = make_jwt("test-secret", "cs");
        let app = rbac_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/admin/test/user-write")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(
            json["code"].as_i64().unwrap(),
            40301,
            "cs 访问 user-write 端点必须返回 40301"
        );
    }

    // ── I-15: finance 可访问 finance 端点 ────────────────────────────────────
    #[tokio::test]
    async fn rbac_finance_can_access_finance_endpoint() {
        let token = make_jwt("test-secret", "finance");
        let app = rbac_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/finance")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // ── I-16: cs 可以访问 user-read 端点（protected 路由）─────────────────
    #[tokio::test]
    async fn rbac_cs_can_access_protected_handler() {
        let token = make_jwt("test-secret", "cs");
        let app = rbac_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/test/protected")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["role"].as_str().unwrap(), "cs");
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10004 集成测试：房间列表接口 GET /api/v1/admin/rooms
    // ════════════════════════════════════════════════════════════════════════

    use crate::modules::room::dto::AdminRoomListRow;
    use crate::modules::room::repository::FakeAdminRoomRepository;
    use chrono::Duration;

    // ── 测试辅助 ─────────────────────────────────────────────────────────────

    fn make_room_row(title: &str, status: &str, offset_secs: i64) -> AdminRoomListRow {
        AdminRoomListRow {
            id: Uuid::new_v4(),
            title: title.to_string(),
            status: status.to_string(),
            room_type: "normal".to_string(),
            member_count: 2,
            max_members: 50,
            owner_id: Uuid::new_v4(),
            owner_nickname: "TestOwner".to_string(),
            owner_avatar: Some("https://avatar.test/1.png".to_string()),
            created_at: Utc::now() - Duration::seconds(offset_secs),
        }
    }

    /// 构建带预置房间数据的 App（operator JWT 有 RoomRead 权限）
    fn room_app(rooms: Vec<AdminRoomListRow>) -> axum::Router {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        for room in rooms {
            room_repo.seed(room);
        }
        build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo,
            Arc::new(crate::modules::user::repository::FakeAdminUserRepository::default()),
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ))
    }

    /// 发起认证 GET 请求，返回响应
    async fn get_rooms(app: axum::Router, token: &str, query: &str) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/rooms{query}"))
                .header("Authorization", format!("Bearer {token}"))
                .header("x-request-id", "test-req-rooms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    fn make_app_jwt(secret: &str) -> String {
        use voice_room_shared::jwt::token::{encode_token, AppClaims};
        let claims = AppClaims {
            sub: Uuid::new_v4().to_string(),
            iss: "voiceroom".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        encode_token(&claims, secret.as_bytes()).unwrap()
    }

    // ── L-01: 默认参数 → 200，返回所有房间（active+closed）─────────────────
    #[tokio::test]
    async fn l01_default_params_returns_all_rooms() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![
            make_room_row("Active Room", "active", 10),
            make_room_row("Closed Room", "closed", 20),
        ]);

        let resp = get_rooms(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0);
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 2, "L-01: 应返回全部 2 个房间");
    }

    // ── L-02: ?status=active → 仅 active ────────────────────────────────────
    #[tokio::test]
    async fn l02_status_active_filter_returns_only_active() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![
            make_room_row("Active Room 1", "active", 10),
            make_room_row("Active Room 2", "active", 20),
            make_room_row("Closed Room", "closed", 30),
        ]);

        let resp = get_rooms(app, &token, "?status=active").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 2, "L-02: active 过滤应返回 2 个");
        for item in json["data"]["items"].as_array().unwrap() {
            assert_eq!(item["status"].as_str().unwrap(), "active");
        }
    }

    // ── L-03: ?status=closed → 仅 closed ────────────────────────────────────
    #[tokio::test]
    async fn l03_status_closed_filter_returns_only_closed() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![
            make_room_row("Active Room", "active", 10),
            make_room_row("Closed Room 1", "closed", 20),
        ]);

        let resp = get_rooms(app, &token, "?status=closed").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 1, "L-03: closed 过滤应返回 1 个");
        assert_eq!(
            json["data"]["items"][0]["status"].as_str().unwrap(),
            "closed"
        );
    }

    // ── L-04: ?keyword=关键词 → 按 title 过滤 ───────────────────────────────
    #[tokio::test]
    async fn l04_keyword_filters_by_title() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![
            make_room_row("Music Room Alpha", "active", 10),
            make_room_row("gaming zone", "active", 20),
            make_room_row("Music Lounge Beta", "closed", 30),
        ]);

        let resp = get_rooms(app, &token, "?keyword=music").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(
            json["data"]["total"].as_i64().unwrap(),
            2,
            "L-04: 'music' 应匹配 2 个房间（大小写不敏感）"
        );
    }

    // ── L-05: ?keyword=xyz（无匹配）→ total=0, items=[] ─────────────────────
    #[tokio::test]
    async fn l05_keyword_no_match_returns_empty() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![make_room_row("Some Room", "active", 10)]);

        let resp = get_rooms(app, &token, "?keyword=xyz_no_match").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 0, "L-05: 无匹配时 total=0");
        assert!(
            json["data"]["items"].as_array().unwrap().is_empty(),
            "L-05: 无匹配时 items=[]"
        );
    }

    // ── L-06: 分页参数正确反映在响应中 ──────────────────────────────────────
    #[tokio::test]
    async fn l06_pagination_params_reflected_in_response() {
        let token = make_jwt("test-secret", "operator");
        let rooms: Vec<_> = (0..5)
            .map(|i| make_room_row(&format!("Room {i}"), "active", i * 10))
            .collect();
        let app = room_app(rooms);

        let resp = get_rooms(app, &token, "?page=2&page_size=2").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["data"]["page"].as_i64().unwrap(), 2, "L-06: page 应为 2");
        assert_eq!(json["data"]["page_size"].as_i64().unwrap(), 2, "L-06: page_size 应为 2");
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 5, "L-06: total 应为 5");
        assert_eq!(
            json["data"]["items"].as_array().unwrap().len(),
            2,
            "L-06: 第 2 页应有 2 条"
        );
    }

    // ── L-09: items 包含 status 字段 ─────────────────────────────────────────
    #[tokio::test]
    async fn l09_items_contain_status_field() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![make_room_row("Test Room", "active", 10)]);

        let resp = get_rooms(app, &token, "").await;
        let json = body_json(resp).await;
        let item = &json["data"]["items"][0];
        assert!(item["status"].as_str().is_some(), "L-09: items 中每条必须含 status 字段");
        assert_eq!(item["status"].as_str().unwrap(), "active");
    }

    // ── L-10: items 包含 owner_id/owner_nickname/owner_avatar ────────────────
    #[tokio::test]
    async fn l10_items_contain_owner_fields() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![make_room_row("Owner Test Room", "active", 10)]);

        let resp = get_rooms(app, &token, "").await;
        let json = body_json(resp).await;
        let item = &json["data"]["items"][0];

        assert!(item["owner_id"].as_str().is_some(), "L-10: 必须包含 owner_id");
        assert!(item["owner_nickname"].as_str().is_some(), "L-10: 必须包含 owner_nickname");
        // owner_avatar 可以为 null（本测试 make_room_row 返回 Some）
        assert!(
            !item["owner_nickname"].as_str().unwrap().is_empty(),
            "L-10: owner_nickname 不能为空"
        );
    }

    // ── E-01: 无 Authorization → 401/40101 ───────────────────────────────────
    #[tokio::test]
    async fn e01_no_auth_header_returns_401_40101() {
        let app = room_app(vec![]);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/rooms")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "E-01: 无 token 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "E-01: 错误码应为 40101");
    }

    // ── E-02: C 端 JWT（iss="voiceroom"）→ 401/40101 ─────────────────────────
    #[tokio::test]
    async fn e02_app_jwt_returns_401_40101() {
        let app_token = make_app_jwt("test-secret");
        let app = room_app(vec![]);
        let resp = get_rooms(app, &app_token, "").await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "E-02: C 端 JWT 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "E-02: 错误码应为 40101");
    }

    // ── E-03: 过期 JWT → 401/40102 ───────────────────────────────────────────
    #[tokio::test]
    async fn e03_expired_jwt_returns_401_40102() {
        let expired_token = make_expired_jwt("test-secret");
        let app = room_app(vec![]);
        let resp = get_rooms(app, &expired_token, "").await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "E-03: 过期 JWT 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40102, "E-03: 错误码应为 40102");
    }

    // ── E-04: finance 角色无 RoomRead 权限 → 403/40301 ───────────────────────
    #[tokio::test]
    async fn e04_finance_role_returns_403_40301() {
        let finance_token = make_jwt("test-secret", "finance");
        let app = room_app(vec![]);
        let resp = get_rooms(app, &finance_token, "").await;

        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "E-04: finance 角色应返回 403");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40301, "E-04: 错误码应为 40301");
    }

    // ── E-05: page=0 → 400/40003 ─────────────────────────────────────────────
    #[tokio::test]
    async fn e05_page_zero_returns_400_40003() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![]);
        let resp = get_rooms(app, &token, "?page=0").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "E-05: page=0 应返回 400");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "E-05: 错误码应为 40003");
    }

    // ── E-06: page_size=0 → 400/40003 ────────────────────────────────────────
    #[tokio::test]
    async fn e06_page_size_zero_returns_400_40003() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![]);
        let resp = get_rooms(app, &token, "?page_size=0").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "E-06: page_size=0 应返回 400");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "E-06: 错误码应为 40003");
    }

    // ── E-07: page_size=101 → 400/40003 ──────────────────────────────────────
    #[tokio::test]
    async fn e07_page_size_101_returns_400_40003() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![]);
        let resp = get_rooms(app, &token, "?page_size=101").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "E-07: page_size=101 应返回 400");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "E-07: 错误码应为 40003");
    }

    // ── E-08: status=invalid → 400/40003 ─────────────────────────────────────
    #[tokio::test]
    async fn e08_invalid_status_returns_400_40003() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![]);
        let resp = get_rooms(app, &token, "?status=invalid").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "E-08: 非法 status 应返回 400");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "E-08: 错误码应为 40003");
    }

    // ── 额外：super_admin 可访问房间列表 ──────────────────────────────────────
    #[tokio::test]
    async fn rooms_super_admin_can_access() {
        let token = make_jwt("test-secret", "super_admin");
        let app = room_app(vec![make_room_row("Room X", "active", 5)]);
        let resp = get_rooms(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // ── 额外：cs 可访问房间列表 ────────────────────────────────────────────────
    #[tokio::test]
    async fn rooms_cs_can_access() {
        let token = make_jwt("test-secret", "cs");
        let app = room_app(vec![make_room_row("CS Room", "active", 5)]);
        let resp = get_rooms(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0);
    }

    // ── 额外：空仓库 → total=0, items=[] ─────────────────────────────────────
    #[tokio::test]
    async fn rooms_empty_repo_returns_zero() {
        let token = make_jwt("test-secret", "operator");
        let app = room_app(vec![]);
        let resp = get_rooms(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 0);
        assert!(json["data"]["items"].as_array().unwrap().is_empty());
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10005 集成测试：房间详情接口 GET /api/v1/admin/rooms/{id}
    // ════════════════════════════════════════════════════════════════════════

    use crate::modules::room::dto::AdminRoomDetailRow;

    fn make_detail_row_for_test(title: &str, status: &str) -> AdminRoomDetailRow {
        AdminRoomDetailRow {
            id: Uuid::new_v4(),
            title: title.to_string(),
            status: status.to_string(),
            room_type: "normal".to_string(),
            member_count: 3,
            max_members: 50,
            owner_id: Uuid::new_v4(),
            owner_nickname: "DetailOwner".to_string(),
            owner_avatar: Some("https://avatar.test/detail.png".to_string()),
            created_at: Utc::now() - Duration::seconds(100),
            updated_at: Utc::now(),
        }
    }

    /// 构建含详情预置数据的 App（room_repo 同时支持 seed + seed_detail）
    fn detail_app(detail_rows: Vec<AdminRoomDetailRow>) -> axum::Router {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        for row in detail_rows {
            room_repo.seed_detail(row);
        }
        build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo,
            Arc::new(crate::modules::user::repository::FakeAdminUserRepository::default()),
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ))
    }

    /// 构建含软删除详情数据的 App
    fn detail_app_with_deleted(row: AdminRoomDetailRow) -> axum::Router {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        room_repo.seed_detail_deleted(row);
        build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo,
            Arc::new(crate::modules::user::repository::FakeAdminUserRepository::default()),
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ))
    }

    /// 发起认证 GET /api/v1/admin/rooms/{id} 请求
    async fn get_room_detail(
        app: axum::Router,
        token: &str,
        id: &str,
    ) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/rooms/{id}"))
                .header("Authorization", format!("Bearer {token}"))
                .header("x-request-id", "test-req-detail")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // ── D-01: 存在的 active 房间 → 200，返回完整详情 ─────────────────────────
    #[tokio::test]
    async fn d01_existing_active_room_returns_200_with_detail() {
        let token = make_jwt("test-secret", "operator");
        let row = make_detail_row_for_test("Music Room", "active");
        let id = row.id;
        let app = detail_app(vec![row]);

        let resp = get_room_detail(app, &token, &id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::OK, "D-01: 已存在 active 房间应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0);
        assert_eq!(json["data"]["room_id"].as_str().unwrap(), id.to_string());
        assert_eq!(json["data"]["title"].as_str().unwrap(), "Music Room");
        assert_eq!(json["data"]["status"].as_str().unwrap(), "active");
    }

    // ── D-02: closed 房间 → 200（后台与 C 端不同，closed 也返回详情）──────────
    #[tokio::test]
    async fn d02_closed_room_returns_200() {
        let token = make_jwt("test-secret", "operator");
        let row = make_detail_row_for_test("Closed Room", "closed");
        let id = row.id;
        let app = detail_app(vec![row]);

        let resp = get_room_detail(app, &token, &id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::OK, "D-02: closed 房间后台也应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["data"]["status"].as_str().unwrap(), "closed");
    }

    // ── D-03: 不存在的 room_id → 404/40400 ───────────────────────────────────
    #[tokio::test]
    async fn d03_nonexistent_room_returns_404_40400() {
        let token = make_jwt("test-secret", "operator");
        let app = detail_app(vec![]);
        let nonexistent_id = Uuid::new_v4().to_string();

        let resp = get_room_detail(app, &token, &nonexistent_id).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "D-03: 不存在的房间应返回 404");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40400, "D-03: 错误码应为 40400");
    }

    // ── D-04: 软删除的房间 → 404/40400 ──────────────────────────────────────
    #[tokio::test]
    async fn d04_soft_deleted_room_returns_404_40400() {
        let token = make_jwt("test-secret", "operator");
        let row = make_detail_row_for_test("Deleted Room", "active");
        let id = row.id;
        let app = detail_app_with_deleted(row);

        let resp = get_room_detail(app, &token, &id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "D-04: 软删除房间应返回 404");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40400, "D-04: 错误码应为 40400");
    }

    // ── D-05: 无效 UUID 格式 → 400/40003 ─────────────────────────────────────
    #[tokio::test]
    async fn d05_invalid_uuid_returns_400_40003() {
        let token = make_jwt("test-secret", "operator");
        let app = detail_app(vec![]);

        let resp = get_room_detail(app, &token, "not-a-uuid").await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "D-05: 无效 UUID 应返回 400");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "D-05: 错误码应为 40003");
    }

    // ── D-06: 无 Authorization → 401/40101 ───────────────────────────────────
    #[tokio::test]
    async fn d06_no_auth_returns_401_40101() {
        let app = detail_app(vec![]);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/admin/rooms/{}", Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "D-06: 无 token 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "D-06: 错误码应为 40101");
    }

    // ── D-07: finance 角色无 RoomRead 权限 → 403/40301 ───────────────────────
    #[tokio::test]
    async fn d07_finance_role_returns_403_40301() {
        let finance_token = make_jwt("test-secret", "finance");
        let row = make_detail_row_for_test("Finance Test Room", "active");
        let id = row.id;
        let app = detail_app(vec![row]);

        let resp = get_room_detail(app, &finance_token, &id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "D-07: finance 角色应返回 403");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40301, "D-07: 错误码应为 40301");
    }

    // ── D-08: 响应包含 owner 嵌套对象和 mic_slots=[] ────────────────────────
    #[tokio::test]
    async fn d08_response_has_owner_object_and_empty_mic_slots() {
        let token = make_jwt("test-secret", "operator");
        let row = make_detail_row_for_test("Full Detail Room", "active");
        let id = row.id;
        let owner_id = row.owner_id;
        let app = detail_app(vec![row]);

        let resp = get_room_detail(app, &token, &id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;

        // owner 是嵌套对象，包含 user_id / nickname / avatar
        let owner = &json["data"]["owner"];
        assert_eq!(
            owner["user_id"].as_str().unwrap(),
            owner_id.to_string(),
            "D-08: owner.user_id 应正确映射"
        );
        assert_eq!(
            owner["nickname"].as_str().unwrap(),
            "DetailOwner",
            "D-08: owner.nickname 应正确映射"
        );
        assert!(
            owner["avatar"].as_str().is_some(),
            "D-08: owner.avatar 应存在"
        );

        // mic_slots 应为空数组（MVP）
        let mic_slots = json["data"]["mic_slots"].as_array().unwrap();
        assert!(mic_slots.is_empty(), "D-08: MVP 阶段 mic_slots 应为空数组");

        // created_at 和 updated_at 必须存在
        assert!(
            json["data"]["created_at"].as_str().is_some(),
            "D-08: created_at 必须存在"
        );
        assert!(
            json["data"]["updated_at"].as_str().is_some(),
            "D-08: updated_at 必须存在"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10006 集成测试：强制关闭房间接口 DELETE /api/v1/admin/rooms/{id}
    // ════════════════════════════════════════════════════════════════════════

    /// 构建含详情数据的 App，同时返回 room_repo（用于 FC-08 跨接口联动测试）
    fn force_close_app(
        detail_rows: Vec<AdminRoomDetailRow>,
    ) -> (axum::Router, Arc<FakeAdminRoomRepository>) {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        for row in detail_rows {
            room_repo.seed_detail(row);
        }
        let router = build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo.clone(),
            Arc::new(crate::modules::user::repository::FakeAdminUserRepository::default()),
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ));
        (router, room_repo)
    }

    /// 构建含详情数据的 App，同时返回 room_repo 和 audit_repo（用于 CA 审计验证测试）
    fn force_close_app_with_audit(
        detail_rows: Vec<AdminRoomDetailRow>,
    ) -> (axum::Router, Arc<FakeAdminRoomRepository>, Arc<FakeAuditRepository>) {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        for row in detail_rows {
            room_repo.seed_detail(row);
        }
        let audit_repo = Arc::new(FakeAuditRepository::default());
        let router = build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo.clone(),
            Arc::new(crate::modules::user::repository::FakeAdminUserRepository::default()),
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            audit_repo.clone(),
        ));
        (router, room_repo, audit_repo)
    }

    /// 发起认证 DELETE /api/v1/admin/rooms/{id} 请求
    async fn delete_room(
        app: axum::Router,
        token: &str,
        id: &str,
    ) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/admin/rooms/{id}"))
                .header("Authorization", format!("Bearer {token}"))
                .header("x-request-id", "test-req-fc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // ── FC-01: super_admin + active 房间 → 200 + code=0 + data=null ───────────
    #[tokio::test]
    async fn fc01_super_admin_closes_active_room_returns_200() {
        let token = make_jwt("test-secret", "super_admin");
        let row = make_detail_row_for_test("Active Room", "active");
        let id = row.id;
        let (app, _) = force_close_app(vec![row]);

        let resp = delete_room(app, &token, &id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::OK, "FC-01: super_admin 关闭 active 房间应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "FC-01: 成功 code 应为 0");
        assert!(json["data"].is_null(), "FC-01: 成功响应 data 应为 null");
    }

    // ── FC-02: operator + 非房主的 active 房间 → 200（无 owner 检查）────────────
    #[tokio::test]
    async fn fc02_operator_closes_non_owned_room_returns_200() {
        let token = make_jwt("test-secret", "operator");
        let mut row = make_detail_row_for_test("Non-Owned Room", "active");
        row.owner_id = Uuid::new_v4(); // 随机 owner，与 operator 无关
        let id = row.id;
        let (app, _) = force_close_app(vec![row]);

        let resp = delete_room(app, &token, &id.to_string()).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "FC-02: operator 无需是房主即可强制关闭，应返回 200"
        );
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0);
    }

    // ── FC-03: 有效 JWT + 不存在房间 → 404/40400 ────────────────────────────────
    #[tokio::test]
    async fn fc03_valid_jwt_nonexistent_room_returns_404_40400() {
        let token = make_jwt("test-secret", "operator");
        let (app, _) = force_close_app(vec![]);
        let nonexistent_id = Uuid::new_v4().to_string();

        let resp = delete_room(app, &token, &nonexistent_id).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "FC-03: 不存在的房间应返回 404");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40400, "FC-03: 错误码应为 40400");
    }

    // ── FC-04: 有效 JWT + 已 closed 房间 → 409/40901 ────────────────────────────
    #[tokio::test]
    async fn fc04_valid_jwt_closed_room_returns_409_40901() {
        let token = make_jwt("test-secret", "operator");
        let row = make_detail_row_for_test("Closed Room", "closed");
        let id = row.id;
        let (app, _) = force_close_app(vec![row]);

        let resp = delete_room(app, &token, &id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT, "FC-04: 已 closed 房间应返回 409");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40901, "FC-04: 错误码应为 40901");
    }

    // ── FC-05: 有效 JWT + 非法 UUID → 400/40003 ─────────────────────────────────
    #[tokio::test]
    async fn fc05_valid_jwt_invalid_uuid_returns_400_40003() {
        let token = make_jwt("test-secret", "operator");
        let (app, _) = force_close_app(vec![]);

        let resp = delete_room(app, &token, "not-a-uuid").await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "FC-05: 非法 UUID 应返回 400");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "FC-05: 错误码应为 40003");
    }

    // ── FC-06: 无 Authorization → 401/40101 ──────────────────────────────────────
    #[tokio::test]
    async fn fc06_no_auth_returns_401_40101() {
        let row = make_detail_row_for_test("Some Room", "active");
        let id = row.id;
        let (app, _) = force_close_app(vec![row]);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/admin/rooms/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "FC-06: 无 token 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "FC-06: 错误码应为 40101");
    }

    // ── FC-07: finance 角色无 RoomForceClose 权限 → 403/40301 ────────────────────
    #[tokio::test]
    async fn fc07_finance_role_returns_403_40301() {
        let token = make_jwt("test-secret", "finance");
        let row = make_detail_row_for_test("Finance Test Room", "active");
        let id = row.id;
        let (app, _) = force_close_app(vec![row]);

        let resp = delete_room(app, &token, &id.to_string()).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "FC-07: finance 角色应返回 403"
        );
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40301, "FC-07: 错误码应为 40301");
    }

    // ── FC-07b: cs 角色无 RoomForceClose 权限 → 403/40301 ────────────────────────
    #[tokio::test]
    async fn fc07b_cs_role_returns_403_40301() {
        let token = make_jwt("test-secret", "cs");
        let row = make_detail_row_for_test("CS Test Room", "active");
        let id = row.id;
        let (app, _) = force_close_app(vec![row]);

        let resp = delete_room(app, &token, &id.to_string()).await;
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "FC-07b: cs 角色应返回 403（cs 无 RoomForceClose 权限）"
        );
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40301, "FC-07b: 错误码应为 40301");
    }

    // ── FC-08: 关闭后 GET detail → 200 + status=="closed"（跨接口联动测试）──────
    #[tokio::test]
    async fn fc08_after_close_get_detail_returns_closed_status() {
        let token = make_jwt("test-secret", "operator");
        let row = make_detail_row_for_test("Will Be Closed", "active");
        let id = row.id;
        let (app, _) = force_close_app(vec![row]);

        // Step 1: DELETE → 强制关闭
        let delete_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/admin/rooms/{id}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            delete_resp.status(),
            StatusCode::OK,
            "FC-08: DELETE 应成功"
        );

        // Step 2: GET → 验证状态已变更为 closed
        let get_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/admin/rooms/{id}"))
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            get_resp.status(),
            StatusCode::OK,
            "FC-08: GET 应返回 200"
        );
        let json = body_json(get_resp).await;
        assert_eq!(
            json["data"]["status"].as_str().unwrap(),
            "closed",
            "FC-08: 关闭后 GET detail 中 status 应为 closed"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10007 集成测试：用户列表接口 GET /api/v1/admin/users
    // ════════════════════════════════════════════════════════════════════════

    use crate::modules::user::dto::AdminUserListRow as AdminUserListRowUser;
    use crate::modules::user::repository::FakeAdminUserRepository;

    // ── 用户测试辅助 ──────────────────────────────────────────────────────────

    fn make_user_row(
        phone: &str,
        nickname: &str,
        is_banned: bool,
        offset_secs: i64,
    ) -> AdminUserListRowUser {
        AdminUserListRowUser {
            id: Uuid::new_v4(),
            phone: phone.to_string(),
            nickname: nickname.to_string(),
            avatar: Some("https://cdn.example.com/avatar.jpg".to_string()),
            coin_balance: 100,
            vip_level: 0,
            is_banned,
            created_at: Utc::now() - Duration::seconds(offset_secs),
        }
    }

    /// 构建带预置用户数据的 App
    fn user_app(users: Vec<AdminUserListRowUser>) -> axum::Router {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        let user_repo = Arc::new(FakeAdminUserRepository::default());
        for user in users {
            user_repo.seed(user);
        }
        build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo,
            user_repo,
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ))
    }

    /// 发起认证 GET /api/v1/admin/users 请求
    async fn get_users(
        app: axum::Router,
        token: &str,
        query: &str,
    ) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/users{query}"))
                .header("Authorization", format!("Bearer {token}"))
                .header("x-request-id", "test-req-users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // ── U-01: super_admin JWT + 无过滤 → 200，total 正确，按 created_at DESC 排序 ──
    #[tokio::test]
    async fn u01_super_admin_no_filter_returns_all_sorted_desc() {
        let token = make_jwt("test-secret", "super_admin");
        let app = user_app(vec![
            make_user_row("13800000001", "Alice", false, 30),
            make_user_row("13800000002", "Bob", false, 10),
            make_user_row("13800000003", "Charlie", false, 20),
        ]);

        let resp = get_users(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::OK, "U-01: super_admin 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0);
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 3, "U-01: total 应为 3");

        // 验证按 created_at DESC：Bob(-10s) > Charlie(-20s) > Alice(-30s)
        let items = json["data"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 3, "U-01: items 应有 3 条");
        assert_eq!(
            items[0]["nickname"].as_str().unwrap(),
            "Bob",
            "U-01: 第一条应是最新的 Bob"
        );
        assert_eq!(
            items[2]["nickname"].as_str().unwrap(),
            "Alice",
            "U-01: 最后一条应是最旧的 Alice"
        );
    }

    // ── U-02: phone 精确搜索 → 200，只返回手机号完全匹配的用户 ──────────────
    #[tokio::test]
    async fn u02_phone_exact_search_returns_matching_user() {
        let token = make_jwt("test-secret", "operator");
        let app = user_app(vec![
            make_user_row("13800000001", "Alice", false, 10),
            make_user_row("13900000002", "Bob", false, 10),
        ]);

        let resp = get_users(app, &token, "?phone=13800000001").await;
        assert_eq!(resp.status(), StatusCode::OK, "U-02: 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 1, "U-02: 精确匹配应只有 1 个");
        assert_eq!(
            json["data"]["items"][0]["phone"].as_str().unwrap(),
            "13800000001",
            "U-02: 返回的手机号应匹配"
        );
    }

    // ── U-03: nickname 模糊搜索 → 200，只返回昵称包含子串的用户 ─────────────
    #[tokio::test]
    async fn u03_nickname_fuzzy_search_returns_matching_users() {
        let token = make_jwt("test-secret", "operator");
        let app = user_app(vec![
            make_user_row("111", "Alice Music", false, 10),
            make_user_row("222", "Music Bob", false, 10),
            make_user_row("333", "Charlie Games", false, 10),
        ]);

        let resp = get_users(app, &token, "?nickname=music").await;
        assert_eq!(resp.status(), StatusCode::OK, "U-03: 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(
            json["data"]["total"].as_i64().unwrap(),
            2,
            "U-03: 模糊搜索 'music' 应匹配 2 个（大小写不敏感）"
        );
    }

    // ── U-04: status=banned → 200，只返回 is_banned=true 的用户 ─────────────
    #[tokio::test]
    async fn u04_status_banned_returns_only_banned_users() {
        let token = make_jwt("test-secret", "operator");
        let app = user_app(vec![
            make_user_row("111", "Normal User", false, 10),
            make_user_row("222", "Banned User 1", true, 10),
            make_user_row("333", "Banned User 2", true, 10),
        ]);

        let resp = get_users(app, &token, "?status=banned").await;
        assert_eq!(resp.status(), StatusCode::OK, "U-04: 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(
            json["data"]["total"].as_i64().unwrap(),
            2,
            "U-04: status=banned 应只返回 2 个封禁用户"
        );
        let items = json["data"]["items"].as_array().unwrap();
        for item in items {
            assert_eq!(
                item["status"].as_str().unwrap(),
                "banned",
                "U-04: items[*].status 均应为 banned"
            );
        }
    }

    // ── U-05: page=2&size=5 → 200，返回第 6-10 条 ───────────────────────────
    #[tokio::test]
    async fn u05_pagination_page2_size5_returns_correct_slice() {
        let token = make_jwt("test-secret", "operator");
        // 预置 10 条数据（offset 递减确保顺序可预测）
        let users: Vec<_> = (0..10)
            .map(|i| make_user_row(
                &format!("138000000{:02}", i),
                &format!("User{:02}", i),
                false,
                i * 10,
            ))
            .collect();
        let app = user_app(users);

        let resp = get_users(app, &token, "?page=2&size=5").await;
        assert_eq!(resp.status(), StatusCode::OK, "U-05: 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 10, "U-05: total 应为 10");
        assert_eq!(json["data"]["page"].as_i64().unwrap(), 2, "U-05: page 应为 2");
        assert_eq!(json["data"]["size"].as_i64().unwrap(), 5, "U-05: size 应为 5");
        assert_eq!(
            json["data"]["items"].as_array().unwrap().len(),
            5,
            "U-05: 第 2 页应有 5 条"
        );
    }

    // ── U-06: 无匹配条件时 → 200，total=0，items=[] ──────────────────────────
    #[tokio::test]
    async fn u06_no_match_returns_empty_result() {
        let token = make_jwt("test-secret", "operator");
        let app = user_app(vec![make_user_row("111", "Alice", false, 10)]);

        let resp = get_users(app, &token, "?phone=00000000000").await;
        assert_eq!(resp.status(), StatusCode::OK, "U-06: 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["data"]["total"].as_i64().unwrap(), 0, "U-06: 无匹配 total 应为 0");
        assert!(
            json["data"]["items"].as_array().unwrap().is_empty(),
            "U-06: 无匹配 items 应为 []"
        );
    }

    // ── U-07: 无 Authorization 头 → 401，code=40101 ──────────────────────────
    #[tokio::test]
    async fn u07_no_auth_returns_401_40101() {
        let app = user_app(vec![]);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "U-07: 无 token 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "U-07: 错误码应为 40101");
    }

    // ── U-08: finance 角色 JWT → 403，code=40301 ─────────────────────────────
    #[tokio::test]
    async fn u08_finance_role_returns_403_40301() {
        let token = make_jwt("test-secret", "finance");
        let app = user_app(vec![]);

        let resp = get_users(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "U-08: finance 角色应返回 403");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40301, "U-08: 错误码应为 40301");
    }

    // ── U-09: cs 角色 JWT → 200（cs 有 UserRead 权限）────────────────────────
    #[tokio::test]
    async fn u09_cs_role_returns_200() {
        let token = make_jwt("test-secret", "cs");
        let app = user_app(vec![
            make_user_row("111", "Normal", false, 10),
        ]);

        let resp = get_users(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::OK, "U-09: cs 角色应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "U-09: code 应为 0");
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10008 集成测试：用户详情接口 GET /api/v1/admin/users/:id
    // ════════════════════════════════════════════════════════════════════════

    // ── 用户详情测试辅助 ───────────────────────────────────────────────────

    fn make_user_row_with_id(
        id: Uuid,
        phone: &str,
        nickname: &str,
        is_banned: bool,
    ) -> AdminUserListRowUser {
        AdminUserListRowUser {
            id,
            phone: phone.to_string(),
            nickname: nickname.to_string(),
            avatar: Some("https://cdn.example.com/avatar.jpg".to_string()),
            coin_balance: 500,
            vip_level: 1,
            is_banned,
            created_at: Utc::now(),
        }
    }

    /// 构建带预置用户数据的 App（支持软删除），返回 user_repo 和 app
    fn user_detail_app(
        users: Vec<AdminUserListRowUser>,
        deleted_users: Vec<AdminUserListRowUser>,
    ) -> axum::Router {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        let user_repo = Arc::new(FakeAdminUserRepository::default());
        for user in users {
            user_repo.seed(user);
        }
        for user in deleted_users {
            user_repo.seed_deleted(user);
        }
        build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo,
            user_repo,
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ))
    }

    /// 发起认证 GET /api/v1/admin/users/:id 请求
    async fn get_user_detail(
        app: axum::Router,
        token: &str,
        id: &str,
    ) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/users/{id}"))
                .header("Authorization", format!("Bearer {token}"))
                .header("x-request-id", "test-req-user-detail")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // ── UD-01: super_admin JWT + 有效 UUID → 200，data.id 与路径参数一致，含 status ─
    #[tokio::test]
    async fn ud01_super_admin_valid_uuid_returns_200_with_correct_data() {
        let token = make_jwt("test-secret", "super_admin");
        let user_id = Uuid::new_v4();
        let row = make_user_row_with_id(user_id, "+8613800138001", "TestUser", false);
        let app = user_detail_app(vec![row], vec![]);

        let resp = get_user_detail(app, &token, &user_id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::OK, "UD-01: super_admin 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "UD-01: code 应为 0");
        assert_eq!(
            json["data"]["id"].as_str().unwrap(),
            user_id.to_string(),
            "UD-01: data.id 应与路径参数一致"
        );
        assert!(
            json["data"]["status"].as_str().is_some(),
            "UD-01: data.status 字段必须存在"
        );
        assert_eq!(
            json["data"]["status"].as_str().unwrap(),
            "normal",
            "UD-01: is_banned=false → status='normal'"
        );
        // 验证 MVP 空数组字段
        assert!(
            json["data"]["recharge_records"].as_array().unwrap().is_empty(),
            "UD-01: recharge_records 应为空数组"
        );
        assert!(
            json["data"]["consume_records"].as_array().unwrap().is_empty(),
            "UD-01: consume_records 应为空数组"
        );
        assert!(
            json["data"]["devices"].as_array().unwrap().is_empty(),
            "UD-01: devices 应为空数组"
        );
    }

    // ── UD-02: 有效 UUID 但用户不存在 → 404，code=40401 ──────────────────────
    #[tokio::test]
    async fn ud02_nonexistent_user_returns_404_40401() {
        let token = make_jwt("test-secret", "operator");
        let app = user_detail_app(vec![], vec![]);
        let nonexistent_id = Uuid::new_v4().to_string();

        let resp = get_user_detail(app, &token, &nonexistent_id).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "UD-02: 用户不存在应返回 404");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40401, "UD-02: 错误码应为 40401");
    }

    // ── UD-03: finance 角色 JWT + 有效 UUID → 403，code=40301 ────────────────
    #[tokio::test]
    async fn ud03_finance_role_returns_403_40301() {
        let token = make_jwt("test-secret", "finance");
        let user_id = Uuid::new_v4();
        let row = make_user_row_with_id(user_id, "+8613800138003", "FinUser", false);
        let app = user_detail_app(vec![row], vec![]);

        let resp = get_user_detail(app, &token, &user_id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "UD-03: finance 角色应返回 403");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40301, "UD-03: 错误码应为 40301");
    }

    // ── UD-04: 路径参数为非法字符串 → 400/40003，非 500 ──────────────────────
    #[tokio::test]
    async fn ud04_invalid_uuid_format_returns_400_not_500() {
        let token = make_jwt("test-secret", "operator");
        let app = user_detail_app(vec![], vec![]);

        let resp = get_user_detail(app, &token, "not-a-uuid").await;
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "UD-04: 非法 UUID 应返回 400"
        );
        let json = body_json(resp).await;
        assert_ne!(
            json["code"].as_i64().unwrap_or(-1),
            50000,
            "UD-04: 非法 UUID 不应返回 500"
        );
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "UD-04: 错误码应为 40003");
    }

    // ── UD-05: 无 Authorization 头 → 401，code=40101 ─────────────────────────
    #[tokio::test]
    async fn ud05_no_auth_returns_401_40101() {
        let user_id = Uuid::new_v4();
        let row = make_user_row_with_id(user_id, "+8613800138005", "NoAuthUser", false);
        let app = user_detail_app(vec![row], vec![]);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/admin/users/{user_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "UD-05: 无 token 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "UD-05: 错误码应为 40101");
    }

    // ── UD-06: 有效 UUID 但用户已软删除 → 404，code=40401 ────────────────────
    #[tokio::test]
    async fn ud06_soft_deleted_user_returns_404_40401() {
        let token = make_jwt("test-secret", "operator");
        let user_id = Uuid::new_v4();
        let deleted_row = make_user_row_with_id(user_id, "+8613800138006", "DeletedUser", false);
        let app = user_detail_app(vec![], vec![deleted_row]);

        let resp = get_user_detail(app, &token, &user_id.to_string()).await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "UD-06: 已软删除用户应返回 404"
        );
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40401, "UD-06: 错误码应为 40401");
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10009 集成测试：封禁/解封接口 POST /api/v1/admin/users/{id}/ban
    // ════════════════════════════════════════════════════════════════════════

    /// 构建带预置用户数据的 App，同时返回 user_repo（用于多步联动测试）
    fn ban_app(
        users: Vec<AdminUserListRowUser>,
    ) -> (axum::Router, Arc<FakeAdminUserRepository>) {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        let user_repo = Arc::new(FakeAdminUserRepository::default());
        for user in users {
            user_repo.seed(user);
        }
        let router = build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo,
            user_repo.clone(),
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ));
        (router, user_repo)
    }

    /// 构建带预置用户数据的 App，同时返回 user_repo 和 audit_repo（用于 CA 审计验证测试）
    fn ban_app_with_audit(
        users: Vec<AdminUserListRowUser>,
    ) -> (axum::Router, Arc<FakeAdminUserRepository>, Arc<FakeAuditRepository>) {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        let user_repo = Arc::new(FakeAdminUserRepository::default());
        for user in users {
            user_repo.seed(user);
        }
        let audit_repo = Arc::new(FakeAuditRepository::default());
        let router = build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo,
            user_repo.clone(),
            Arc::new(crate::modules::stats::FakeAdminStatsRepository::default()),
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            audit_repo.clone(),
        ));
        (router, user_repo, audit_repo)
    }

    /// 发起 POST /api/v1/admin/users/{id}/ban 请求
    async fn post_ban(
        app: axum::Router,
        token: &str,
        id: &str,
        body: &str,
    ) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/users/{id}/ban"))
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("x-request-id", "test-req-ban")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // ── UB-01: super_admin JWT + 正常用户 + action=ban → 200，data.status="banned" ─
    #[tokio::test]
    async fn ub01_super_admin_ban_normal_user_returns_200_banned() {
        let token = make_jwt("test-secret", "super_admin");
        let user_id = Uuid::new_v4();
        let (app, _) = ban_app(vec![make_user_row_with_id(
            user_id,
            "+8613800001001",
            "UB01User",
            false,
        )]);

        let resp = post_ban(app, &token, &user_id.to_string(), r#"{"action":"ban"}"#).await;
        assert_eq!(resp.status(), StatusCode::OK, "UB-01: 封禁应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "UB-01: code 应为 0");
        assert_eq!(
            json["data"]["status"].as_str().unwrap(),
            "banned",
            "UB-01: data.status 应为 banned"
        );
        assert_eq!(
            json["data"]["id"].as_str().unwrap(),
            user_id.to_string(),
            "UB-01: data.id 应与路径参数一致"
        );
    }

    // ── UB-02: super_admin JWT + 已封禁用户 + action=unban → 200，data.status="normal" ─
    #[tokio::test]
    async fn ub02_super_admin_unban_banned_user_returns_200_normal() {
        let token = make_jwt("test-secret", "super_admin");
        let user_id = Uuid::new_v4();
        let (app, _) = ban_app(vec![make_user_row_with_id(
            user_id,
            "+8613800001002",
            "UB02User",
            true, // 已封禁
        )]);

        let resp = post_ban(app, &token, &user_id.to_string(), r#"{"action":"unban"}"#).await;
        assert_eq!(resp.status(), StatusCode::OK, "UB-02: 解封应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "UB-02: code 应为 0");
        assert_eq!(
            json["data"]["status"].as_str().unwrap(),
            "normal",
            "UB-02: data.status 应为 normal"
        );
    }

    // ── UB-03: 已封禁用户重复 ban → 409，code=40900 ──────────────────────────
    #[tokio::test]
    async fn ub03_ban_already_banned_user_returns_409_40900() {
        let token = make_jwt("test-secret", "super_admin");
        let user_id = Uuid::new_v4();
        let (app, _) = ban_app(vec![make_user_row_with_id(
            user_id,
            "+8613800001003",
            "UB03User",
            true, // 已封禁
        )]);

        let resp = post_ban(app, &token, &user_id.to_string(), r#"{"action":"ban"}"#).await;
        assert_eq!(resp.status(), StatusCode::CONFLICT, "UB-03: 重复 ban 应返回 409");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40900, "UB-03: 错误码应为 40900");
    }

    // ── UB-04: 不存在的 UUID → 404，code=40401 ───────────────────────────────
    #[tokio::test]
    async fn ub04_nonexistent_user_returns_404_40401() {
        let token = make_jwt("test-secret", "super_admin");
        let nonexistent_id = Uuid::new_v4();
        let (app, _) = ban_app(vec![]);

        let resp = post_ban(app, &token, &nonexistent_id.to_string(), r#"{"action":"ban"}"#).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "UB-04: 用户不存在应返回 404");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40401, "UB-04: 错误码应为 40401");
    }

    // ── UB-05: cs 角色 JWT + action=ban → 403，code=40301（无 UserWrite 权限）──
    #[tokio::test]
    async fn ub05_cs_role_returns_403_40301() {
        let token = make_jwt("test-secret", "cs");
        let user_id = Uuid::new_v4();
        let (app, _) = ban_app(vec![make_user_row_with_id(
            user_id,
            "+8613800001005",
            "UB05User",
            false,
        )]);

        let resp = post_ban(app, &token, &user_id.to_string(), r#"{"action":"ban"}"#).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "UB-05: cs 角色应返回 403");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40301, "UB-05: 错误码应为 40301");
    }

    // ── UB-06: 无 Authorization 头 → 401，code=40101 ─────────────────────────
    #[tokio::test]
    async fn ub06_no_auth_returns_401_40101() {
        let user_id = Uuid::new_v4();
        let (app, _) = ban_app(vec![make_user_row_with_id(
            user_id,
            "+8613800001006",
            "UB06User",
            false,
        )]);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/admin/users/{user_id}/ban"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"action":"ban"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "UB-06: 无 token 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "UB-06: 错误码应为 40101");
    }

    // ── UB-07: 无效 action（如 "kick"）→ 400，code=40003 ─────────────────────
    #[tokio::test]
    async fn ub07_invalid_action_returns_400_40003() {
        let token = make_jwt("test-secret", "super_admin");
        let user_id = Uuid::new_v4();
        let (app, _) = ban_app(vec![make_user_row_with_id(
            user_id,
            "+8613800001007",
            "UB07User",
            false,
        )]);

        let resp = post_ban(app, &token, &user_id.to_string(), r#"{"action":"kick"}"#).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "UB-07: 无效 action 应返回 400");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "UB-07: 错误码应为 40003");
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10010 集成测试：数据统计接口 GET /api/v1/admin/stats/overview
    // ════════════════════════════════════════════════════════════════════════

    use crate::modules::stats::FakeAdminStatsRepository;

    /// 构建带预置统计数据的 App
    fn stats_app(stats_repo: Arc<FakeAdminStatsRepository>) -> axum::Router {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let room_repo = Arc::new(FakeAdminRoomRepository::default());
        let user_repo = Arc::new(FakeAdminUserRepository::default());
        build_app(AppState::new(
            admin_repo,
            log_repo,
            room_repo,
            user_repo,
            stats_repo,
            "test-secret".to_string(),
            Arc::new(NoopEventPublisher::default()),
            Arc::new(FakeAuditRepository::default()),
        ))
    }

    /// 发起认证 GET /api/v1/admin/stats/overview 请求
    async fn get_stats_overview(
        app: axum::Router,
        token: &str,
        query: &str,
    ) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/stats/overview{query}"))
                .header("Authorization", format!("Bearer {token}"))
                .header("x-request-id", "test-req-stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // ── US-01: super_admin JWT + 无日期参数 → 200，响应结构完整 ───────────────
    #[tokio::test]
    async fn us01_super_admin_no_date_params_returns_200_with_full_structure() {
        let token = make_jwt("test-secret", "super_admin");
        let app = stats_app(Arc::new(FakeAdminStatsRepository::default()));

        let resp = get_stats_overview(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::OK, "US-01: super_admin 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "US-01: code 应为 0");

        let data = &json["data"];
        assert!(data["dau"].is_number(), "US-01: dau 字段必须存在");
        assert!(data["new_users"].is_number(), "US-01: new_users 字段必须存在");
        assert_eq!(data["active_rooms"].as_i64().unwrap(), 0, "US-01: active_rooms MVP 值应为 0");
        assert_eq!(data["online_users"].as_i64().unwrap(), 0, "US-01: online_users MVP 值应为 0");
        assert!(data["date_range"]["start"].as_str().is_some(), "US-01: date_range.start 必须存在");
        assert!(data["date_range"]["end"].as_str().is_some(), "US-01: date_range.end 必须存在");
        // 无参数时 start == end（均为今天）
        assert_eq!(
            data["date_range"]["start"],
            data["date_range"]["end"],
            "US-01: 无参数时 date_range.start 应等于 date_range.end"
        );
    }

    // ── US-02: super_admin JWT + start_date & end_date → 200，回显正确 ────────
    #[tokio::test]
    async fn us02_super_admin_with_date_range_returns_200_with_echoed_dates() {
        let token = make_jwt("test-secret", "super_admin");
        let app = stats_app(Arc::new(FakeAdminStatsRepository::default()));

        let resp = get_stats_overview(app, &token, "?start_date=2024-01-01&end_date=2024-01-31").await;
        assert_eq!(resp.status(), StatusCode::OK, "US-02: 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0);
        assert_eq!(
            json["data"]["date_range"]["start"].as_str().unwrap(),
            "2024-01-01",
            "US-02: date_range.start 应回显"
        );
        assert_eq!(
            json["data"]["date_range"]["end"].as_str().unwrap(),
            "2024-01-31",
            "US-02: date_range.end 应回显"
        );
    }

    // ── US-03: cs 角色 JWT → 403，code=40301（cs 无 StatsRead 权限）──────────
    #[tokio::test]
    async fn us03_cs_role_returns_403_40301() {
        let token = make_jwt("test-secret", "cs");
        let app = stats_app(Arc::new(FakeAdminStatsRepository::default()));

        let resp = get_stats_overview(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "US-03: cs 角色应返回 403");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40301, "US-03: 错误码应为 40301");
    }

    // ── US-04: 无 Authorization 头 → 401，code=40101 ─────────────────────────
    #[tokio::test]
    async fn us04_no_auth_returns_401_40101() {
        let app = stats_app(Arc::new(FakeAdminStatsRepository::default()));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/stats/overview")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "US-04: 无 token 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40101, "US-04: 错误码应为 40101");
    }

    // ── US-05: super_admin JWT + start_date=invalid → 400，code=40003 ─────────
    #[tokio::test]
    async fn us05_invalid_start_date_returns_400_40003() {
        let token = make_jwt("test-secret", "super_admin");
        let app = stats_app(Arc::new(FakeAdminStatsRepository::default()));

        let resp = get_stats_overview(app, &token, "?start_date=invalid").await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "US-05: 无效日期应返回 400");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 40003, "US-05: 错误码应为 40003");
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10011 集成测试（EI-01~02）：引入 event_publisher 后接口行为不退化
    // ════════════════════════════════════════════════════════════════════════

    // ── EI-01: POST /api/v1/admin/users/:id/ban（action=ban）→ 200 OK ────────
    // 事件发布为 Noop，不影响响应
    #[tokio::test]
    async fn ei01_post_ban_with_noop_publisher_returns_200() {
        let token = make_jwt("test-secret", "operator");
        let user_id = Uuid::new_v4();
        let (app, _) = ban_app(vec![make_user_row_with_id(
            user_id,
            "+8613800011001",
            "EI01User",
            false,
        )]);

        let resp = post_ban(app, &token, &user_id.to_string(), r#"{"action":"ban"}"#).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "EI-01: ban 接口引入 event_publisher 后应仍返回 200"
        );
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "EI-01: code 应为 0");
        assert_eq!(
            json["data"]["status"].as_str().unwrap(),
            "banned",
            "EI-01: data.status 应为 banned"
        );
    }

    // ── EI-02: DELETE /api/v1/admin/rooms/:id → 200 OK ────────────────────────
    // 事件发布为 Noop，不影响响应
    #[tokio::test]
    async fn ei02_delete_room_with_noop_publisher_returns_200() {
        let token = make_jwt("test-secret", "operator");
        let row = make_detail_row_for_test("EI02 Room", "active");
        let id = row.id;
        let (app, _) = force_close_app(vec![row]);

        let resp = delete_room(app, &token, &id.to_string()).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "EI-02: force_close_room 接口引入 event_publisher 后应仍返回 200"
        );
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "EI-02: code 应为 0");
        assert!(json["data"].is_null(), "EI-02: data 应为 null");
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10012 集成测试（UL-01~05）：审计日志查询接口 GET /api/v1/admin/logs
    // ════════════════════════════════════════════════════════════════════════

    /// 发起认证 GET /api/v1/admin/logs 请求
    async fn get_logs(
        app: axum::Router,
        token: &str,
        query: &str,
    ) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/admin/logs{query}"))
                .header("Authorization", format!("Bearer {token}"))
                .header("x-request-id", "test-req-logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    /// 发起带 X-Forwarded-For 的 POST /ban 请求
    async fn post_ban_with_xff(
        app: axum::Router,
        token: &str,
        id: &str,
        body: &str,
        xff: &str,
    ) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/admin/users/{id}/ban"))
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .header("x-forwarded-for", xff)
                .header("x-request-id", "test-req-ban-xff")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // ── UL-01: super_admin GET /api/v1/admin/logs → 200 + items 数组（可为空）──
    #[tokio::test]
    async fn ul01_super_admin_get_logs_returns_200_with_items_array() {
        let token = make_jwt("test-secret", "super_admin");
        let app = build_app(AppState::for_test());

        let resp = get_logs(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::OK, "UL-01: super_admin 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "UL-01: code 应为 0");
        assert!(
            json["data"]["items"].as_array().is_some(),
            "UL-01: data.items 应为数组"
        );
        assert_eq!(
            json["data"]["total"].as_i64().unwrap(),
            0,
            "UL-01: 空仓库 total 应为 0"
        );
    }

    // ── UL-02: operator GET /api/v1/admin/logs?action=ban_user&page=1&size=10 → 200 ──
    #[tokio::test]
    async fn ul02_operator_get_logs_with_action_filter_returns_200() {
        let token = make_jwt("test-secret", "operator");
        let app = build_app(AppState::for_test());

        let resp = get_logs(app, &token, "?action=ban_user&page=1&size=10").await;
        assert_eq!(resp.status(), StatusCode::OK, "UL-02: operator 应返回 200");
        let json = body_json(resp).await;
        assert_eq!(json["code"].as_i64().unwrap(), 0, "UL-02: code 应为 0");
        assert_eq!(
            json["data"]["total"].as_i64().unwrap(),
            0,
            "UL-02: 空仓库 total=0"
        );
    }

    // ── UL-03: size=101 → 400，参数校验错误 ───────────────────────────────────
    #[tokio::test]
    async fn ul03_size_over_100_returns_400() {
        let token = make_jwt("test-secret", "super_admin");
        let app = build_app(AppState::for_test());

        let resp = get_logs(app, &token, "?page=1&size=101").await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "UL-03: size=101 应返回 400");
        let json = body_json(resp).await;
        assert_eq!(
            json["code"].as_i64().unwrap(),
            40003,
            "UL-03: 错误码应为 40003"
        );
    }

    // ── UL-04: cs 角色无 LogRead 权限 → 403/40301 ─────────────────────────────
    #[tokio::test]
    async fn ul04_cs_role_lacks_log_read_returns_403() {
        let token = make_jwt("test-secret", "cs");
        let app = build_app(AppState::for_test());

        let resp = get_logs(app, &token, "").await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN, "UL-04: cs 角色应返回 403");
        let json = body_json(resp).await;
        assert_eq!(
            json["code"].as_i64().unwrap(),
            40301,
            "UL-04: 错误码应为 40301"
        );
    }

    // ── UL-05: 无 Authorization 头 → 401/40101 ────────────────────────────────
    #[tokio::test]
    async fn ul05_no_token_returns_401() {
        let app = build_app(AppState::for_test());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/admin/logs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "UL-05: 无 token 应返回 401");
        let json = body_json(resp).await;
        assert_eq!(
            json["code"].as_i64().unwrap(),
            40101,
            "UL-05: 错误码应为 40101"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    // T-10012 Controller 审计验证（CA-01~05）
    // ════════════════════════════════════════════════════════════════════════

    // ── CA-01: POST /ban action=ban 成功 → audit_repo 有 1 条，action="ban_user" ─
    #[tokio::test]
    async fn ca01_ban_success_writes_ban_user_audit_log() {
        let token = make_jwt("test-secret", "operator");
        let user_id = Uuid::new_v4();
        let (app, _, audit_repo) = ban_app_with_audit(vec![make_user_row_with_id(
            user_id,
            "+8613900001001",
            "CA01User",
            false,
        )]);

        let resp = post_ban(app, &token, &user_id.to_string(), r#"{"action":"ban"}"#).await;
        assert_eq!(resp.status(), StatusCode::OK, "CA-01: ban 应成功");

        let logs = audit_repo.get_logs();
        assert_eq!(logs.len(), 1, "CA-01: audit_repo 应有 1 条记录");
        assert_eq!(logs[0].action, "ban_user", "CA-01: action 应为 ban_user");
        assert_eq!(
            logs[0].target_id,
            Some(user_id),
            "CA-01: target_id 应为 user_id"
        );
    }

    // ── CA-02: POST /ban action=unban 成功 → logs[0].action="unban_user" ────────
    #[tokio::test]
    async fn ca02_unban_success_writes_unban_user_audit_log() {
        let token = make_jwt("test-secret", "operator");
        let user_id = Uuid::new_v4();
        let (app, _, audit_repo) = ban_app_with_audit(vec![make_user_row_with_id(
            user_id,
            "+8613900001002",
            "CA02User",
            true, // 已封禁，可以 unban
        )]);

        let resp = post_ban(app, &token, &user_id.to_string(), r#"{"action":"unban"}"#).await;
        assert_eq!(resp.status(), StatusCode::OK, "CA-02: unban 应成功");

        let logs = audit_repo.get_logs();
        assert_eq!(logs.len(), 1, "CA-02: audit_repo 应有 1 条记录");
        assert_eq!(logs[0].action, "unban_user", "CA-02: action 应为 unban_user");
    }

    // ── CA-03: DELETE /rooms/:id 成功 → logs[0].action="close_room"，target_type="room" ─
    #[tokio::test]
    async fn ca03_force_close_room_writes_close_room_audit_log() {
        let token = make_jwt("test-secret", "operator");
        let row = make_detail_row_for_test("CA03 Room", "active");
        let room_id = row.id;
        let (app, _, audit_repo) = force_close_app_with_audit(vec![row]);

        let resp = delete_room(app, &token, &room_id.to_string()).await;
        assert_eq!(resp.status(), StatusCode::OK, "CA-03: close_room 应成功");

        let logs = audit_repo.get_logs();
        assert_eq!(logs.len(), 1, "CA-03: audit_repo 应有 1 条记录");
        assert_eq!(logs[0].action, "close_room", "CA-03: action 应为 close_room");
        assert_eq!(
            logs[0].target_type,
            Some("room".to_string()),
            "CA-03: target_type 应为 room"
        );
        assert_eq!(
            logs[0].target_id,
            Some(room_id),
            "CA-03: target_id 应为 room_id"
        );
    }

    // ── CA-04: POST /ban 携带 X-Forwarded-For → logs[0].ip_address 为首段 IP ───
    #[tokio::test]
    async fn ca04_ban_with_xff_writes_ip_to_audit_log() {
        let token = make_jwt("test-secret", "operator");
        let user_id = Uuid::new_v4();
        let (app, _, audit_repo) = ban_app_with_audit(vec![make_user_row_with_id(
            user_id,
            "+8613900001004",
            "CA04User",
            false,
        )]);

        let resp = post_ban_with_xff(
            app,
            &token,
            &user_id.to_string(),
            r#"{"action":"ban"}"#,
            "1.2.3.4, 5.6.7.8",
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK, "CA-04: ban 应成功");

        let logs = audit_repo.get_logs();
        assert_eq!(logs.len(), 1, "CA-04: audit_repo 应有 1 条记录");
        assert_eq!(
            logs[0].ip_address,
            Some("1.2.3.4".to_string()),
            "CA-04: ip_address 应取 X-Forwarded-For 的第一段"
        );
    }

    // ── CA-05: POST /ban → 用户不存在（404）→ audit_repo 无新记录 ───────────────
    #[tokio::test]
    async fn ca05_ban_failure_does_not_write_audit_log() {
        let token = make_jwt("test-secret", "operator");
        let nonexistent_id = Uuid::new_v4();
        let (app, _, audit_repo) = ban_app_with_audit(vec![]); // 空用户仓库

        let resp = post_ban(
            app,
            &token,
            &nonexistent_id.to_string(),
            r#"{"action":"ban"}"#,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND, "CA-05: 用户不存在应返回 404");

        let logs = audit_repo.get_logs();
        assert_eq!(
            logs.len(),
            0,
            "CA-05: 业务失败时 audit_repo 不应有新记录"
        );
    }
}
