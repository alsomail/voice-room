use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension, Json,
};

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

// ─── Handler ─────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/users/:id/events
///
/// 查询指定用户的行为事件流，需要 UserRead 权限。
///
/// 权限规则：
/// - `super_admin`：全量可查（含 admin_* 事件）
/// - `operator` / `cs`：可查，admin_* 事件自动过滤
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

    // ── 用户存在性校验（不存在 → 404）────────────────────────────────────
    if let Err(e) = state.user_service.get_user_detail(user_id).await {
        return err_response(e, rc.request_id());
    }

    // ── 角色衍生：非 super_admin 过滤 admin_* 事件 ───────────────────────
    let filter_admin_events = ctx.role != "super_admin";

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
}
