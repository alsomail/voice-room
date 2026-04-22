pub mod bootstrap;
pub mod common;
pub mod core;
pub mod events;
pub mod infrastructure;
pub mod modules;
pub mod room;
pub mod stats;
pub mod ws;

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    /// H-01 (T-00005)：JWT 拒绝路径的 request_id 必须与 X-Request-Id header 一致，不能为空。
    /// 无 token 访问 /api/v1/users/me → 401，body.request_id == header X-Request-Id
    #[tokio::test]
    async fn get_me_no_token_401_request_id_matches_header() {
        let app = crate::bootstrap::build_app(crate::bootstrap::AppState::for_test());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/users/me")
                    .header("x-request-id", "test-req-id-jwt-reject")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // 响应头中的 X-Request-Id（由 request_context_middleware 注入）
        let header_request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .expect("x-request-id header must be present")
            .to_owned();

        assert_eq!(
            header_request_id, "test-req-id-jwt-reject",
            "X-Request-Id header should echo the sent value"
        );

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("body should be valid JSON");

        // 关键断言：body 中的 request_id 必须与 header 一致（不能为空字符串）
        assert_eq!(
            json["request_id"], "test-req-id-jwt-reject",
            "JWT rejection body.request_id must match X-Request-Id header, not be empty"
        );
        assert_eq!(json["code"], 40101, "error code should be 40101 (Unauthorized)");
    }

    #[tokio::test]
    async fn ping_returns_json_payload_and_request_id() {
        let app = crate::bootstrap::build_app(crate::bootstrap::AppState::for_test());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .expect("x-request-id header should be present")
            .to_owned();

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");

        assert_eq!(
            std::str::from_utf8(&body).expect("body should be utf8"),
            format!(r#"{{"status":"ok","request_id":"{request_id}"}}"#)
        );
    }
}
