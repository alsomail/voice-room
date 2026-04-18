pub mod bootstrap;
pub mod common;
pub mod infrastructure;
pub mod modules;

#[cfg(test)]
mod tests {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

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
