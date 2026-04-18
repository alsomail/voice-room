use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use voice_room_shared::error::code::ErrorCode;

use crate::common::response::ApiResponse;

/// 服务端错误类型，可直接作为 Axum handler 的 Err 返回值。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // 400
    #[error("Invalid phone number format")]
    InvalidPhoneNumber,

    // 401
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid verification code")]
    InvalidVerificationCode,
    #[error("Verification code expired")]
    VerificationCodeExpired,
    #[error("Verification code max attempts exceeded")]
    VerificationCodeMaxAttempts,

    // 429
    #[error("Verification code sent too frequently")]
    VerificationCodeCooldown,
    #[error("Daily verification code limit exceeded")]
    VerificationCodeDailyLimit,

    // 404
    #[error("Resource not found: {0}")]
    NotFound(String),

    // 500
    #[error("SMS send failed: {0}")]
    SmsSendFailed(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Redis error: {0}")]
    RedisError(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    fn error_code(&self) -> ErrorCode {
        match self {
            AppError::InvalidPhoneNumber => ErrorCode::InvalidPhoneNumber,
            AppError::Unauthorized => ErrorCode::Unauthorized,
            AppError::TokenExpired => ErrorCode::TokenExpired,
            AppError::InvalidVerificationCode => ErrorCode::InvalidVerificationCode,
            AppError::VerificationCodeExpired => ErrorCode::VerificationCodeExpired,
            AppError::VerificationCodeMaxAttempts => ErrorCode::VerificationCodeMaxAttempts,
            AppError::VerificationCodeCooldown => ErrorCode::VerificationCodeCooldown,
            AppError::VerificationCodeDailyLimit => ErrorCode::VerificationCodeDailyLimit,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::SmsSendFailed(_)
            | AppError::DatabaseError(_)
            | AppError::RedisError(_)
            | AppError::Internal(_) => ErrorCode::InternalError,
        }
    }

    fn http_status(&self) -> StatusCode {
        match self {
            AppError::InvalidPhoneNumber => StatusCode::BAD_REQUEST,
            AppError::Unauthorized
            | AppError::TokenExpired
            | AppError::InvalidVerificationCode
            | AppError::VerificationCodeExpired
            | AppError::VerificationCodeMaxAttempts => StatusCode::UNAUTHORIZED,
            AppError::VerificationCodeCooldown | AppError::VerificationCodeDailyLimit => {
                StatusCode::TOO_MANY_REQUESTS
            }
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::SmsSendFailed(_)
            | AppError::DatabaseError(_)
            | AppError::RedisError(_)
            | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::DatabaseError(e.to_string())
    }
}

impl From<redis::RedisError> for AppError {
    fn from(e: redis::RedisError) -> Self {
        AppError::RedisError(e.to_string())
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: i32,
    message: String,
    request_id: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = self.error_code() as i32;
        let message = self.to_string();
        let body = ErrorBody {
            code,
            message,
            request_id: String::new(), // middleware 会在外层注入
        };
        (self.http_status(), Json(body)).into_response()
    }
}

/// 将 AppError 包进统一响应 ApiResponse
pub fn err_response(err: AppError, request_id: &str) -> Response {
    let code = err.error_code() as i32;
    let message = err.to_string();
    let body = ApiResponse::<()> {
        code,
        message,
        data: None,
        request_id: request_id.to_string(),
    };
    (err.http_status(), Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_phone_maps_to_400() {
        let err = AppError::InvalidPhoneNumber;
        assert_eq!(err.http_status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code() as i32, 40001);
    }

    #[test]
    fn unauthorized_maps_to_401() {
        let err = AppError::Unauthorized;
        assert_eq!(err.http_status(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.error_code() as i32, 40101);
    }

    #[test]
    fn token_expired_maps_to_401_with_40102() {
        let err = AppError::TokenExpired;
        assert_eq!(err.http_status(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.error_code() as i32, 40102);
    }

    #[test]
    fn cooldown_maps_to_429_with_42901() {
        let err = AppError::VerificationCodeCooldown;
        assert_eq!(err.http_status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(err.error_code() as i32, 42901);
    }

    #[test]
    fn daily_limit_maps_to_429_with_42902() {
        let err = AppError::VerificationCodeDailyLimit;
        assert_eq!(err.http_status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(err.error_code() as i32, 42902);
    }

    #[test]
    fn not_found_maps_to_404() {
        let err = AppError::NotFound("user".into());
        assert_eq!(err.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_code() as i32, 40400);
    }
}
