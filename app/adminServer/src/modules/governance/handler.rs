use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    bootstrap::AppState,
    common::{
        auth::{AdminAuthContext, Permission},
        error::err_response,
        response::ApiResponse,
        RequestContext,
    },
    modules::audit::controller::extract_ip,
};

use super::service::GovernanceQueryParams;

// ─── HTTP Query 参数（Deserialize from query string）────────────────────────

/// GET query params（kicks 和 mutes 共用）
#[derive(Debug, Deserialize, Default)]
pub struct GovernanceHttpQuery {
    pub room_id: Option<Uuid>,
    pub target_user_id: Option<Uuid>,
    pub operator_user_id: Option<Uuid>,
    pub from: Option<String>,
    pub to: Option<String>,
    /// 仅 mutes 有效
    #[serde(rename = "type")]
    pub mute_type: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

impl From<GovernanceHttpQuery> for GovernanceQueryParams {
    fn from(q: GovernanceHttpQuery) -> Self {
        Self {
            room_id: q.room_id,
            target_user_id: q.target_user_id,
            operator_user_id: q.operator_user_id,
            from: q.from,
            to: q.to,
            mute_type: q.mute_type,
            page: q.page,
            limit: q.limit,
        }
    }
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// GET /api/v1/admin/governance/kicks
///
/// 查询踢人审计日志。
/// 权限：super_admin / operator / cs 可查；finance → 403。
pub async fn list_kicks_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Query(query): Query<GovernanceHttpQuery>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::GovernanceRead) {
        return err_response(e, rc.request_id());
    }

    let ip = extract_ip(&headers);
    let params: GovernanceQueryParams = query.into();

    match state
        .governance_service
        .query_kicks(params, ctx.admin_id, ip)
        .await
    {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

/// GET /api/v1/admin/governance/mutes
///
/// 查询禁言审计日志。
/// 权限：super_admin / operator / cs 可查；finance → 403。
pub async fn list_mutes_handler(
    ctx: AdminAuthContext,
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    headers: HeaderMap,
    Query(query): Query<GovernanceHttpQuery>,
) -> axum::response::Response {
    // RBAC 校验
    if let Err(e) = ctx.require_permission(Permission::GovernanceRead) {
        return err_response(e, rc.request_id());
    }

    let ip = extract_ip(&headers);
    let params: GovernanceQueryParams = query.into();

    match state
        .governance_service
        .query_mutes(params, ctx.admin_id, ip)
        .await
    {
        Ok(resp) => Json(ApiResponse::ok(resp, rc.request_id())).into_response(),
        Err(e) => err_response(e, rc.request_id()),
    }
}

// ─── 单元/集成测试（G16-06 handler 层）────────────────────────────────────────
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

    /// G16-06-handler: finance 角色访问 kicks → 403
    #[tokio::test]
    async fn h_g16_06_finance_kicks_forbidden() {
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
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// G16-06-handler: finance 角色访问 mutes → 403
    #[tokio::test]
    async fn h_g16_06_finance_mutes_forbidden() {
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
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    /// super_admin 访问 kicks → 200
    #[tokio::test]
    async fn h_super_admin_kicks_ok() {
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
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// cs 角色访问 mutes → 200
    #[tokio::test]
    async fn h_cs_mutes_ok() {
        let state = AppState::for_test();
        let app = build_app(state);
        let token = make_jwt("cs");

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/mutes")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// 无 JWT → 401
    #[tokio::test]
    async fn h_no_auth_returns_401() {
        let state = AppState::for_test();
        let app = build_app(state);

        let req = Request::builder()
            .method("GET")
            .uri("/api/v1/admin/governance/kicks")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
