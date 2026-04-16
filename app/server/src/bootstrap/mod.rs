use axum::{extract::Extension, middleware, routing::get, Json, Router};
use serde::Serialize;

use crate::{common::RequestContext, infrastructure::logging::request_context_middleware};

pub fn build_app() -> Router {
    Router::new()
        .route("/ping", get(ping))
        .layer(middleware::from_fn(request_context_middleware))
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
