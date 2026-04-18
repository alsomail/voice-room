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
