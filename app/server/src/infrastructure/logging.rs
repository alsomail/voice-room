use axum::{
    body::Body,
    http::Request,
    http::{header::HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::{field::Empty, Instrument};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

use crate::{common::RequestContext, infrastructure::config::LogSettings};

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

pub fn init_tracing(settings: &LogSettings) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(settings.level.as_str()))?;

    if settings.format == "json" {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true)
                    .with_target(true),
            )
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_target(true))
            .try_init()?;
    }

    Ok(())
}

pub async fn request_context_middleware(mut request: Request<Body>, next: Next) -> Response {
    let request_id = extract_request_id(&request).unwrap_or_else(generate_request_id);

    request
        .extensions_mut()
        .insert(RequestContext::new(request_id.clone()));

    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
        status_code = Empty
    );

    let mut response = next.run(request).instrument(span.clone()).await;
    span.record("status_code", response.status().as_u16());

    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER.clone(), header_value);
    }

    response
}

fn extract_request_id(request: &Request<Body>) -> Option<String> {
    request
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}
