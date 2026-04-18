use std::sync::Arc;

use axum::{extract::Extension, middleware, routing::get, Json, Router};
use serde::Serialize;

use crate::{
    common::RequestContext,
    infrastructure::{logging::request_context_middleware, redis_store::SmsCodeStore, third_party::sms::SmsProvider},
    modules::auth::{auth_routes, repository::UserRepository, service::AuthService},
};

#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub jwt_secret: String,
}

impl AppState {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        code_store: Arc<dyn SmsCodeStore>,
        sms: Arc<dyn SmsProvider>,
        jwt_secret: String,
    ) -> Self {
        let auth_service = Arc::new(AuthService::new(
            user_repo,
            code_store,
            sms,
            jwt_secret.clone(),
        ));
        Self {
            auth_service,
            jwt_secret,
        }
    }

    #[cfg(test)]
    pub fn for_test() -> Self {
        use crate::infrastructure::{
            redis_store::FakeCodeStore, third_party::sms::MockSmsProvider,
        };
        use crate::modules::auth::repository::FakeUserRepository;
        Self::new(
            Arc::new(FakeUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider::default()),
            "test-secret".to_string(),
        )
    }
}

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/ping", get(ping))
        .merge(auth_routes())
        .layer(middleware::from_fn(request_context_middleware))
        .with_state(state)
}

#[derive(Serialize)]
struct PingResponse {
    status: &'static str,
    request_id: String,
}

async fn ping(Extension(request_context): Extension<RequestContext>) -> Json<PingResponse> {
    tracing::info!(request_id = %request_context.request_id(), "handled ping request");

    Json(PingResponse {
        status: "ok",
        request_id: request_context.request_id().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// H-01: 错误响应 body 中的 request_id 应与 X-Request-Id header 一致
    #[tokio::test]
    async fn error_response_body_contains_request_id() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/verification-codes")
                    .header("content-type", "application/json")
                    .header("x-request-id", "test-req-id-42")
                    .body(Body::from(r#"{"phone":"invalid"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = body_json(response).await;
        assert_eq!(
            json["request_id"], "test-req-id-42",
            "error body must echo the X-Request-Id"
        );
    }

    /// 成功响应中 request_id 已正确（回归保护）
    #[tokio::test]
    async fn success_response_body_contains_request_id() {
        // 直接构造一个 send-code 请求（MockSmsProvider 不报错）
        let response = build_app(AppState::for_test())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/verification-codes")
                    .header("content-type", "application/json")
                    .header("x-request-id", "req-ok-1")
                    .body(Body::from(r#"{"phone":"+8613800138000"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["request_id"], "req-ok-1");
    }
}
