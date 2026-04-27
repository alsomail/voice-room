//! T-0000N: AppServer 统一 `/health` 端点集成测试。
//!
//! 验收用例覆盖：
//! - U-1：`GET /health` → 200 + `{status:"ok", service:"app-server", version:<non-empty>}`
//! - U-3：免鉴权（无 Authorization / Cookie / 自定义 header 也能 200）
//! - N-1：`POST /health` → 405 Method Not Allowed
//! - R-1：通过独立 Router 挂载 health handler，不耦合 `/ping`，避免误改

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use tower::ServiceExt;
use voice_room_server::bootstrap::health;

fn build_health_router() -> Router {
    Router::new().route("/health", get(health))
}

#[tokio::test]
async fn get_health_returns_200_with_expected_json() {
    // U-1
    let app = build_health_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let json: serde_json::Value =
        serde_json::from_slice(&body_bytes).expect("body should be valid JSON");

    assert_eq!(json["status"], "ok", "status field must be 'ok'");
    assert_eq!(
        json["service"], "app-server",
        "service field must identify app-server"
    );
    let version = json["version"]
        .as_str()
        .expect("version must be a string");
    assert!(!version.is_empty(), "version must be non-empty");
}

#[tokio::test]
async fn get_health_works_without_any_auth_or_headers() {
    // U-3：纯净请求，无任何 header
    let app = build_health_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "/health must respond 200 without authentication"
    );
}

#[tokio::test]
async fn post_health_returns_405_method_not_allowed() {
    // N-1
    let app = build_health_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(
        response.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "POST /health must yield 405"
    );
}
