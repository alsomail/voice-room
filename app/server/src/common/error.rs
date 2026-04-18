use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use voice_room_shared::error::code::ErrorCode;

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
    /// 注意：此实现仅作为兜底保留。所有业务 handler 应通过 `err_response()` 传入 request_id。
    fn into_response(self) -> Response {
        err_response(self, "")
    }
}

/// 将 AppError 序列化为带 request_id 的 JSON 错误响应。
/// 所有 controller 的错误路径必须调用此函数以填充 request_id。
pub fn err_response(err: AppError, request_id: &str) -> Response {
    let code = err.error_code() as i32;
    // M-03: 5xx 错误对外暴露通用文本，内部细节通过 tracing 记录
    let message = err.safe_message();
    let body = ErrorBody {
        code,
        message,
        request_id: request_id.to_string(),
    };
    (err.http_status(), Json(body)).into_response()
}

impl AppError {
    /// 对外安全的错误消息：4xx 用原始描述，5xx 用通用文本（避免泄露内部信息）
    fn safe_message(&self) -> String {
        match self {
            AppError::DatabaseError(e) => {
                tracing::error!(detail = %e, "database error");
                "internal server error".to_string()
            }
            AppError::RedisError(e) => {
                tracing::error!(detail = %e, "redis error");
                "internal server error".to_string()
            }
            AppError::Internal(e) => {
                tracing::error!(detail = %e, "internal error");
                "internal server error".to_string()
            }
            AppError::SmsSendFailed(e) => {
                tracing::error!(detail = %e, "sms send failed");
                "failed to send verification code".to_string()
            }
            _ => self.to_string(),
        }
    }
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
