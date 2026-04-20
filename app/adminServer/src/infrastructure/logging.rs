use axum::{
    body::Body,
    http::{header::HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use tracing::{field::Empty, Instrument};
use uuid::Uuid;

use crate::common::RequestContext;

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// 请求上下文中间件：提取/生成 X-Request-Id，注入 RequestContext Extension，
/// 绑定 tracing span，并在响应头回传 X-Request-Id。
pub async fn request_context_middleware(mut request: Request<Body>, next: Next) -> Response {
    let request_id = extract_request_id(&request).unwrap_or_else(generate_request_id);

    request
        .extensions_mut()
        .insert(RequestContext::new(request_id.clone()));

    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method    = %request.method(),
        uri       = %request.uri(),
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
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

/// 从请求头中提取客户端 IP 地址（X-Forwarded-For 优先）。
pub fn extract_client_ip(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
}
