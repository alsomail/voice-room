//! 审计 Axum 中间件（T-10012 验收 #3 / P2-14）
//!
//! 自动拦截敏感操作并写入审计日志，避免控制器内手写 `audit_logger.log_action`。
//!
//! ## MVP 范围（高频 3 个端点）
//! 涉及 TDS 修订较大，本批先迁移以下三类高频操作；其余端点保持现状，
//! 留作 follow-up（见审查报告 P2-14 修复记录）。
//!
//! | Method | Path                                  | action 取值                  |
//! |--------|---------------------------------------|------------------------------|
//! | POST   | /api/v1/admin/users/{id}/ban          | `ban_user` 或 `unban_user`   |
//! | DELETE | /api/v1/admin/rooms/{id}              | `close_room`                 |
//!
//! ## 设计要点
//! - 命中白名单时，中间件**先消费再回填** request body（保留原 JSON 给 handler 解析），
//!   并在响应 status 2xx 时调用 `AuditLogger::log_action`；非 2xx 不写日志。
//! - 提取 admin_id 直接复用 `extract_admin_auth_context`（与 `FromRequestParts` 同源），
//!   无需在路由层叠加额外 layer。
//! - 仅当 path 命中白名单才解析 body，否则零开销直通。
//! - target_id 从 URL path 段中解析为 UUID（`/users/{id}/...` 或 `/rooms/{id}`）。
//!
//! ## 与控制器的协作
//! 控制器内的手写 `audit_logger.log_action` 调用同步删除，避免重复写入。

use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::{
    common::middleware::jwt_auth::extract_admin_auth_context,
    modules::audit::{controller::extract_ip, service::AuditLogger},
};

/// 审计中间件状态：仅依赖 logger + jwt_secret。
#[derive(Clone)]
pub struct AuditMiddlewareState {
    pub audit_logger: Arc<AuditLogger>,
    pub jwt_secret: String,
}

/// 白名单匹配结果，决定 action / target_type / 是否需要解析 body。
#[derive(Debug, Clone, PartialEq, Eq)]
struct AuditRoute {
    target_type: &'static str,
    /// 解析 body JSON 的 `action` 字段（适用 ban_user：值为 "ban" / "unban"）。
    /// None = 直接采用 `static_action`。
    action_from_body_field: Option<&'static str>,
    /// 当 `action_from_body_field` 为 None 时使用此固定 action。
    static_action: Option<&'static str>,
    /// body action="ban" → ban_user, action="unban" → unban_user 的映射。
    body_action_map: Option<&'static [(&'static str, &'static str)]>,
}

const BAN_BODY_MAP: &[(&str, &str)] = &[("ban", "ban_user"), ("unban", "unban_user")];

/// 解析 path/method → 命中的审计路由（None 表示未命中白名单）。
fn match_audit_route(method: &Method, path: &str) -> Option<AuditRoute> {
    // POST /api/v1/admin/users/{id}/ban
    if method == Method::POST
        && path.starts_with("/api/v1/admin/users/")
        && path.ends_with("/ban")
    {
        return Some(AuditRoute {
            target_type: "user",
            action_from_body_field: Some("action"),
            static_action: None,
            body_action_map: Some(BAN_BODY_MAP),
        });
    }
    // DELETE /api/v1/admin/rooms/{id}
    if method == Method::DELETE && path.starts_with("/api/v1/admin/rooms/") {
        // 排除子路径，确保是 .../rooms/{id} 末段
        let tail = &path["/api/v1/admin/rooms/".len()..];
        if !tail.is_empty() && !tail.contains('/') {
            return Some(AuditRoute {
                target_type: "room",
                action_from_body_field: None,
                static_action: Some("close_room"),
                body_action_map: None,
            });
        }
    }
    None
}

/// 从 path 中尝试解析 target UUID（path 段中第一个能解析为 UUID 的）。
fn extract_target_id(path: &str) -> Option<uuid::Uuid> {
    path.split('/')
        .find_map(|seg| uuid::Uuid::parse_str(seg).ok())
}

/// 体积上限：审计 body 解析最大 64 KiB（足以容纳所有合法管理操作 JSON）。
const MAX_AUDIT_BODY_BYTES: usize = 64 * 1024;

/// 审计 Axum middleware（`from_fn_with_state`）。
///
/// 工作流程：
/// 1. 路由白名单匹配；未命中 → 直通。
/// 2. 命中 → 抽取 admin_id（JWT）+ target_id（path）+ ip（header）+ body（JSON）。
/// 3. 调用下游 handler，捕获响应 status。
/// 4. 仅当 2xx 写入审计日志（fire-and-forget，失败仅 warn）。
pub async fn audit_middleware(
    State(state): State<AuditMiddlewareState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();

    let Some(route) = match_audit_route(&method, &path) else {
        // 未命中 → 完全透传，零开销
        return next.run(request).await;
    };

    // 命中白名单：解构以便消费 body；其他元数据先复制
    let (parts, body) = request.into_parts();
    let admin_id = match extract_admin_auth_context(&parts.headers, &state.jwt_secret) {
        Ok(ctx) => ctx.admin_id,
        Err(_) => {
            // 鉴权失败：交还给下游统一返回 401，无需审计
            let req = Request::from_parts(parts, body);
            return next.run(req).await;
        }
    };
    let ip = extract_ip(&parts.headers);
    let target_id = extract_target_id(&path);

    // 读取并回填 body
    let bytes = match to_bytes(body, MAX_AUDIT_BODY_BYTES).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error=?e, %path, "audit middleware: read body failed");
            // 无法读取 body：构造 413 响应
            return Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body(Body::empty())
                .unwrap();
        }
    };

    let body_json: Option<serde_json::Value> =
        if bytes.is_empty() { None } else { serde_json::from_slice(&bytes).ok() };

    // 解析 action 与 detail
    let (action_str, detail) = resolve_action_and_detail(&route, body_json.as_ref());

    // 重建 request 给下游 handler
    let req = Request::from_parts(parts, Body::from(bytes));
    let response = next.run(req).await;

    // 仅 2xx 写审计；4xx/5xx 不写
    if response.status().is_success() {
        if let Some(action) = action_str {
            state
                .audit_logger
                .log_action(
                    admin_id,
                    action,
                    Some(route.target_type),
                    target_id,
                    ip,
                    detail,
                )
                .await;
        }
    }
    response
}

/// 根据路由配置 + body 决定 audit action 与 detail JSON。
fn resolve_action_and_detail(
    route: &AuditRoute,
    body: Option<&serde_json::Value>,
) -> (Option<&'static str>, Option<serde_json::Value>) {
    // 静态 action 路由：detail 直接采用整个 body（如有）
    if let Some(static_action) = route.static_action {
        return (Some(static_action), body.cloned());
    }

    // body 字段驱动 action（ban/unban）
    if let (Some(field), Some(map), Some(body_json)) = (
        route.action_from_body_field,
        route.body_action_map,
        body,
    ) {
        let action_value = body_json.get(field).and_then(|v| v.as_str());
        if let Some(av) = action_value {
            for (k, v) in map {
                if av == *k {
                    return (Some(*v), Some(body_json.clone()));
                }
            }
        }
        // body 字段非法：让下游 handler 返回 400，此处不写审计
        return (None, None);
    }

    (None, None)
}

// ─── 单元测试 ────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;
    use serde_json::json;
    use uuid::Uuid;

    /// AM-01: ban 路径 + POST 命中白名单
    #[test]
    fn am01_match_ban_route() {
        let r = match_audit_route(
            &Method::POST,
            "/api/v1/admin/users/550e8400-e29b-41d4-a716-446655440000/ban",
        )
        .expect("AM-01: should match");
        assert_eq!(r.target_type, "user");
        assert_eq!(r.action_from_body_field, Some("action"));
    }

    /// AM-02: close_room（DELETE /rooms/:id）命中
    #[test]
    fn am02_match_close_room_route() {
        let r = match_audit_route(
            &Method::DELETE,
            "/api/v1/admin/rooms/550e8400-e29b-41d4-a716-446655440000",
        )
        .expect("AM-02: should match");
        assert_eq!(r.target_type, "room");
        assert_eq!(r.static_action, Some("close_room"));
    }

    /// AM-03: GET /rooms 不匹配
    #[test]
    fn am03_get_rooms_not_matched() {
        assert!(match_audit_route(&Method::GET, "/api/v1/admin/rooms").is_none());
    }

    /// AM-04: GET /rooms/:id 不匹配（仅 DELETE）
    #[test]
    fn am04_get_room_detail_not_matched() {
        assert!(
            match_audit_route(&Method::GET, "/api/v1/admin/rooms/abc-id").is_none(),
            "AM-04: GET 不应触发 close_room 审计"
        );
    }

    /// AM-05: extract_target_id 取出 path 中的 UUID
    #[test]
    fn am05_extract_target_id_from_path() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let got = extract_target_id(&format!("/api/v1/admin/users/{id}/ban")).unwrap();
        assert_eq!(got.to_string(), id);
    }

    /// AM-06: ban body action=ban → ban_user，detail 含完整 body
    #[test]
    fn am06_resolve_ban_action_and_detail() {
        let route = match_audit_route(
            &Method::POST,
            "/api/v1/admin/users/550e8400-e29b-41d4-a716-446655440000/ban",
        )
        .unwrap();
        let body = json!({"action": "ban", "ban_type": "permanent", "reason": "spam"});
        let (action, detail) = resolve_action_and_detail(&route, Some(&body));
        assert_eq!(action, Some("ban_user"));
        let d = detail.unwrap();
        assert_eq!(d["ban_type"], "permanent");
        assert_eq!(d["reason"], "spam");
    }

    /// AM-07: ban body action=unban → unban_user
    #[test]
    fn am07_resolve_unban_action() {
        let route = match_audit_route(
            &Method::POST,
            "/api/v1/admin/users/550e8400-e29b-41d4-a716-446655440000/ban",
        )
        .unwrap();
        let body = json!({"action": "unban"});
        let (action, _) = resolve_action_and_detail(&route, Some(&body));
        assert_eq!(action, Some("unban_user"));
    }

    /// AM-08: ban body action 非法（"foo"）→ 不写审计
    #[test]
    fn am08_invalid_ban_action_skips_audit() {
        let route = match_audit_route(
            &Method::POST,
            "/api/v1/admin/users/550e8400-e29b-41d4-a716-446655440000/ban",
        )
        .unwrap();
        let body = json!({"action": "foo"});
        let (action, _) = resolve_action_and_detail(&route, Some(&body));
        assert_eq!(action, None);
    }

    /// AM-09: close_room 静态 action，无 body 也成立
    #[test]
    fn am09_close_room_static_action() {
        let route = match_audit_route(
            &Method::DELETE,
            "/api/v1/admin/rooms/550e8400-e29b-41d4-a716-446655440000",
        )
        .unwrap();
        let (action, detail) = resolve_action_and_detail(&route, None);
        assert_eq!(action, Some("close_room"));
        assert!(detail.is_none(), "AM-09: 无 body 时 detail 为 None");
    }

    /// AM-10: extract_target_id 失败：path 无 UUID → None
    #[test]
    fn am10_extract_target_id_none_when_no_uuid() {
        assert!(extract_target_id("/api/v1/admin/users").is_none());
    }

    /// AM-11: 端到端 — 命中白名单且 handler 返回 200，写入一条审计日志（含 detail）
    #[tokio::test]
    async fn am11_end_to_end_success_writes_audit_with_detail() {
        use crate::modules::audit::repository::FakeAuditRepository;
        use axum::{routing::post, Router};
        use std::time::{SystemTime, UNIX_EPOCH};
        use tower::ServiceExt;
        use voice_room_shared::jwt::token::{encode_token, AdminClaims};

        let repo = Arc::new(FakeAuditRepository::default());
        let logger = Arc::new(AuditLogger::new(repo.clone()));
        let mw_state = AuditMiddlewareState {
            audit_logger: logger.clone(),
            jwt_secret: "test-secret".into(),
        };

        async fn fake_ban_handler() -> &'static str {
            "ok"
        }

        let app = Router::new()
            .route("/api/v1/admin/users/{id}/ban", post(fake_ban_handler))
            .layer(axum::middleware::from_fn_with_state(
                mw_state,
                audit_middleware,
            ));

        let admin_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let claims = AdminClaims {
            sub: admin_id.to_string(),
            role: "operator".into(),
            iss: "voiceroom-admin".into(),
            exp: now + 3600,
            iat: now,
        };
        let token = encode_token(&claims, b"test-secret").unwrap();

        let req = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/api/v1/admin/users/{target_id}/ban"))
            .header("Authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .header("x-real-ip", "1.2.3.4")
            .body(Body::from(
                serde_json::to_vec(&json!({"action": "ban", "ban_type": "permanent", "reason": "spam"}))
                    .unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let logs = repo.get_logs();
        assert_eq!(logs.len(), 1, "AM-11: 必须写入一条审计");
        assert_eq!(logs[0].action, "ban_user");
        assert_eq!(logs[0].admin_id, admin_id);
        assert_eq!(logs[0].target_id, Some(target_id));
        assert_eq!(logs[0].ip_address.as_deref(), Some("1.2.3.4"));
        assert_eq!(logs[0].target_type.as_deref(), Some("user"));
        let detail = logs[0].detail.as_ref().expect("AM-11: detail 必须存在");
        assert_eq!(detail["ban_type"], "permanent");
        assert_eq!(detail["reason"], "spam");
        assert_eq!(detail["action"], "ban");
    }

    /// AM-12: handler 返回 4xx 时不写审计
    #[tokio::test]
    async fn am12_non_2xx_response_skips_audit() {
        use crate::modules::audit::repository::FakeAuditRepository;
        use axum::{routing::post, Router};
        use std::time::{SystemTime, UNIX_EPOCH};
        use tower::ServiceExt;
        use voice_room_shared::jwt::token::{encode_token, AdminClaims};

        let repo = Arc::new(FakeAuditRepository::default());
        let logger = Arc::new(AuditLogger::new(repo.clone()));
        let mw_state = AuditMiddlewareState {
            audit_logger: logger.clone(),
            jwt_secret: "test-secret".into(),
        };

        async fn failing_handler() -> (StatusCode, &'static str) {
            (StatusCode::CONFLICT, "duplicate")
        }

        let app = Router::new()
            .route("/api/v1/admin/users/{id}/ban", post(failing_handler))
            .layer(axum::middleware::from_fn_with_state(
                mw_state,
                audit_middleware,
            ));

        let admin_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let claims = AdminClaims {
            sub: admin_id.to_string(),
            role: "operator".into(),
            iss: "voiceroom-admin".into(),
            exp: now + 3600,
            iat: now,
        };
        let token = encode_token(&claims, b"test-secret").unwrap();

        let req = axum::http::Request::builder()
            .method("POST")
            .uri(format!("/api/v1/admin/users/{target_id}/ban"))
            .header("Authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&json!({"action": "ban"})).unwrap(),
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let logs = repo.get_logs();
        assert_eq!(
            logs.len(),
            0,
            "AM-12: 4xx 响应不应写入审计（业务失败不审计）"
        );
    }

    /// AM-13: 非白名单路径完全透传，不写审计
    #[tokio::test]
    async fn am13_non_whitelisted_path_skips_audit() {
        use crate::modules::audit::repository::FakeAuditRepository;
        use axum::{routing::get, Router};
        use tower::ServiceExt;

        let repo = Arc::new(FakeAuditRepository::default());
        let logger = Arc::new(AuditLogger::new(repo.clone()));
        let mw_state = AuditMiddlewareState {
            audit_logger: logger.clone(),
            jwt_secret: "test-secret".into(),
        };

        async fn ok() -> &'static str {
            "ok"
        }

        let app = Router::new()
            .route("/api/v1/admin/users", get(ok))
            .layer(axum::middleware::from_fn_with_state(
                mw_state,
                audit_middleware,
            ));

        let req = axum::http::Request::builder()
            .uri("/api/v1/admin/users")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(repo.get_logs().len(), 0);
    }

    /// AM-14: close_room（DELETE /rooms/:id）端到端写入 close_room 审计
    #[tokio::test]
    async fn am14_close_room_end_to_end_writes_audit() {
        use crate::modules::audit::repository::FakeAuditRepository;
        use axum::{routing::delete, Router};
        use std::time::{SystemTime, UNIX_EPOCH};
        use tower::ServiceExt;
        use voice_room_shared::jwt::token::{encode_token, AdminClaims};

        let repo = Arc::new(FakeAuditRepository::default());
        let logger = Arc::new(AuditLogger::new(repo.clone()));
        let mw_state = AuditMiddlewareState {
            audit_logger: logger.clone(),
            jwt_secret: "test-secret".into(),
        };

        async fn close_handler() -> &'static str {
            "ok"
        }

        let app = Router::new()
            .route("/api/v1/admin/rooms/{id}", delete(close_handler))
            .layer(axum::middleware::from_fn_with_state(
                mw_state,
                audit_middleware,
            ));

        let admin_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let claims = AdminClaims {
            sub: admin_id.to_string(),
            role: "operator".into(),
            iss: "voiceroom-admin".into(),
            exp: now + 3600,
            iat: now,
        };
        let token = encode_token(&claims, b"test-secret").unwrap();

        let req = axum::http::Request::builder()
            .method("DELETE")
            .uri(format!("/api/v1/admin/rooms/{target_id}"))
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let logs = repo.get_logs();
        assert_eq!(logs.len(), 1, "AM-14: close_room 端到端应写入审计");
        assert_eq!(logs[0].action, "close_room");
        assert_eq!(logs[0].admin_id, admin_id);
        assert_eq!(logs[0].target_id, Some(target_id));
        assert_eq!(logs[0].target_type.as_deref(), Some("room"));
    }
}
