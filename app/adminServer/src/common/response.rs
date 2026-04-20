use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

/// 统一 JSON 响应包装，参见 doc/protocol.md §1.3
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    pub request_id: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T, request_id: impl Into<String>) -> Self {
        Self {
            code: 0,
            message: "ok".to_string(),
            data: Some(data),
            request_id: request_id.into(),
        }
    }
}

impl<T: Serialize + Send> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Dummy {
        val: i32,
    }

    #[test]
    fn ok_response_has_code_zero_and_data() {
        let resp = ApiResponse::ok(Dummy { val: 42 }, "req-1");
        assert_eq!(resp.code, 0);
        assert_eq!(resp.message, "ok");
        assert_eq!(resp.request_id, "req-1");
        assert!(resp.data.is_some());
    }
}
