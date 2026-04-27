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
    #[error("Required parameter missing: {0}")]
    ParameterMissing(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Invalid count: {0}")]
    InvalidCount(String),
    #[error("Insufficient balance")]
    InsufficientBalance,

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

    // 403
    #[error("Forbidden: {0}")]
    Forbidden(String),

    // 409
    #[error("User already has an active room")]
    ActiveRoomExists,
    #[error("Room is already closed")]
    RoomAlreadyClosed,

    // Room password errors (T-00026)
    #[error("Room is not a password room")]
    NotPasswordRoom,

    // 429
    #[error("Verification code sent too frequently")]
    VerificationCodeCooldown,
    #[error("Daily verification code limit exceeded")]
    VerificationCodeDailyLimit,

    // 404
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Gift not available")]
    GiftNotAvailable,
    #[error("Receiver not available")]
    ReceiverUnavailable,

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
            AppError::ParameterMissing(_) => ErrorCode::ParameterMissing,
            AppError::ValidationError(_) => ErrorCode::ValidationError,
            AppError::InvalidCount(_) => ErrorCode::InvalidPhoneNumber, // 复用 40001
            AppError::InsufficientBalance => ErrorCode::InsufficientBalance,
            AppError::Unauthorized => ErrorCode::Unauthorized,
            AppError::TokenExpired => ErrorCode::TokenExpired,
            AppError::InvalidVerificationCode => ErrorCode::InvalidVerificationCode,
            AppError::VerificationCodeExpired => ErrorCode::VerificationCodeExpired,
            AppError::VerificationCodeMaxAttempts => ErrorCode::VerificationCodeMaxAttempts,
            AppError::Forbidden(_) => ErrorCode::Forbidden,
            AppError::ActiveRoomExists => ErrorCode::Conflict,
            AppError::RoomAlreadyClosed => ErrorCode::RoomAlreadyClosed,
            AppError::NotPasswordRoom => ErrorCode::NotPasswordRoom,
            AppError::VerificationCodeCooldown => ErrorCode::VerificationCodeCooldown,
            AppError::VerificationCodeDailyLimit => ErrorCode::VerificationCodeDailyLimit,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::GiftNotAvailable => ErrorCode::GiftNotAvailable,
            AppError::ReceiverUnavailable => ErrorCode::ReceiverUnavailable,
            AppError::SmsSendFailed(_)
            | AppError::DatabaseError(_)
            | AppError::RedisError(_)
            | AppError::Internal(_) => ErrorCode::InternalError,
        }
    }

    fn http_status(&self) -> StatusCode {
        match self {
            AppError::InvalidPhoneNumber
            | AppError::ParameterMissing(_)
            | AppError::ValidationError(_)
            | AppError::InvalidCount(_)
            | AppError::InsufficientBalance => StatusCode::BAD_REQUEST,
            AppError::Unauthorized
            | AppError::TokenExpired
            | AppError::InvalidVerificationCode
            | AppError::VerificationCodeExpired
            | AppError::VerificationCodeMaxAttempts => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::ActiveRoomExists | AppError::RoomAlreadyClosed => StatusCode::CONFLICT,
            AppError::NotPasswordRoom => StatusCode::BAD_REQUEST,
            AppError::VerificationCodeCooldown | AppError::VerificationCodeDailyLimit => {
                StatusCode::TOO_MANY_REQUESTS
            }
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::GiftNotAvailable | AppError::ReceiverUnavailable => StatusCode::NOT_FOUND,
            AppError::SmsSendFailed(_)
            | AppError::DatabaseError(_)
            | AppError::RedisError(_)
            | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        // H-01: PG unique violation (23505) → ActiveRoomExists (HTTP 409)
        // 并发 INSERT 被 idx_rooms_owner_active 拦截时，DB 返回 23505，
        // 必须转换为 409 而非 500。
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.code().as_deref() == Some("23505") {
                return AppError::ActiveRoomExists;
            }
        }
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

    // ── Mock DatabaseError（用于构造 sqlx::Error::Database 以测试 From impl）──

    struct MockDbError {
        pg_code: &'static str,
    }

    impl std::fmt::Display for MockDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "mock db error code={}", self.pg_code)
        }
    }

    impl std::fmt::Debug for MockDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "MockDbError({})", self.pg_code)
        }
    }

    impl std::error::Error for MockDbError {}

    impl sqlx::error::DatabaseError for MockDbError {
        fn message(&self) -> &str {
            "mock db error"
        }
        fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
            Some(std::borrow::Cow::Borrowed(self.pg_code))
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::UniqueViolation
        }
    }

    // ── H-01: DB 唯一约束 23505 → ActiveRoomExists ────────────────────────────

    /// H-01: sqlx 返回 PG 错误码 23505 时，From<sqlx::Error> 必须返回 ActiveRoomExists
    #[test]
    fn test_db_unique_violation_translates_to_active_room_exists() {
        let db_err = MockDbError { pg_code: "23505" };
        let sqlx_err = sqlx::Error::Database(Box::new(db_err));
        let app_err = AppError::from(sqlx_err);
        assert!(
            matches!(app_err, AppError::ActiveRoomExists),
            "PG error 23505 should map to ActiveRoomExists, got: {app_err:?}"
        );
    }

    /// H-01 补充: 其他 DB 错误码仍映射到 DatabaseError（不误伤）
    #[test]
    fn test_db_other_code_translates_to_database_error() {
        let db_err = MockDbError { pg_code: "42000" };
        let sqlx_err = sqlx::Error::Database(Box::new(db_err));
        let app_err = AppError::from(sqlx_err);
        assert!(
            matches!(app_err, AppError::DatabaseError(_)),
            "other DB error codes should map to DatabaseError, got: {app_err:?}"
        );
    }

    // ── M-02: ValidationError / ActiveRoomExists 错误码单元测试 ──────────────

    /// M-02: ValidationError → HTTP 400 / code 40003
    #[test]
    fn validation_error_maps_to_400_with_40003() {
        let err = AppError::ValidationError("test validation".into());
        assert_eq!(err.http_status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code() as i32, 40003);
    }

    /// M-02: ActiveRoomExists → HTTP 409 / code 40900
    #[test]
    fn active_room_exists_maps_to_409_with_40900() {
        let err = AppError::ActiveRoomExists;
        assert_eq!(err.http_status(), StatusCode::CONFLICT);
        assert_eq!(err.error_code() as i32, 40900);
    }

    // ── E-01: Forbidden → 403/40301 ────────────────────────────────────────

    /// E-01: Forbidden → HTTP 403 / code 40301
    #[test]
    fn forbidden_maps_to_403_with_40301() {
        let err = AppError::Forbidden("not the owner".into());
        assert_eq!(err.http_status(), StatusCode::FORBIDDEN);
        assert_eq!(err.error_code() as i32, 40301);
    }

    // ── E-02: RoomAlreadyClosed → 409/40901 ────────────────────────────────

    /// E-02: RoomAlreadyClosed → HTTP 409 / code 40901
    #[test]
    fn room_already_closed_maps_to_409_with_40901() {
        let err = AppError::RoomAlreadyClosed;
        assert_eq!(err.http_status(), StatusCode::CONFLICT);
        assert_eq!(err.error_code() as i32, 40901);
    }

    // ── 原有测试（保持不变）──────────────────────────────────────────────────

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
