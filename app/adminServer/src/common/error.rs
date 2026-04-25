use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use voice_room_shared::error::code::ErrorCode;

/// Admin Server 统一错误类型。
/// 错误码与 HTTP 状态码映射参见 doc/protocol.md §1.4。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // 401 - 管理员凭证无效（账号不存在 / 密码错误，统一返回，防止用户名枚举）
    #[error("Invalid admin credentials")]
    InvalidAdminCredentials,

    // 401 - JWT 无效（无 token / 签名非法 / iss 不匹配）
    #[error("Unauthorized")]
    Unauthorized,

    // 401 - JWT 已过期
    #[error("Token expired")]
    TokenExpired,

    // 403 - 账号被禁用
    #[error("Admin account disabled")]
    AccountDisabled,

    // 403 - RBAC 权限不足
    #[error("Forbidden: insufficient permissions")]
    Forbidden,

    // 404 - 资源不存在
    #[error("Not found: {0}")]
    NotFound(String),

    // 404 - 用户不存在或已软删除（T-10008，code=40401）
    #[error("User not found: {0}")]
    UserNotFound(String),

    // 409 - 房间已关闭（T-10006）
    #[error("Room already closed")]
    RoomAlreadyClosed,

    // 409 - 用户已是封禁状态（重复 ban，T-10009）
    #[error("User is already banned")]
    UserAlreadyBanned,

    // 409 - 用户已是正常状态（重复 unban，T-10009）
    #[error("User already in normal status")]
    UserAlreadyNormal,

    // 400 - 参数校验失败
    #[error("Validation error: {0}")]
    ValidationError(String),

    // 400 - 钱包余额不足（T-10013，code=40204）
    #[error("Insufficient balance")]
    InsufficientBalance,

    // 409 - 礼物 code 已存在（T-10014，code=40900）
    #[error("Duplicate code: {0}")]
    DuplicateCode(String),

    // 500
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    pub(crate) fn error_code(&self) -> ErrorCode {
        match self {
            AppError::InvalidAdminCredentials => ErrorCode::InvalidAdminCredentials,
            AppError::Unauthorized => ErrorCode::Unauthorized,
            AppError::TokenExpired => ErrorCode::TokenExpired,
            AppError::AccountDisabled => ErrorCode::AccountDisabled,
            AppError::Forbidden => ErrorCode::Forbidden,
            AppError::ValidationError(_) => ErrorCode::ValidationError,
            AppError::InsufficientBalance => ErrorCode::InsufficientBalance,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::UserNotFound(_) => ErrorCode::UserNotFound,
            AppError::RoomAlreadyClosed => ErrorCode::RoomAlreadyClosed,
            AppError::UserAlreadyBanned | AppError::UserAlreadyNormal => ErrorCode::Conflict,
            AppError::DuplicateCode(_) => ErrorCode::Conflict,
            AppError::DatabaseError(_) | AppError::Internal(_) => ErrorCode::InternalError,
        }
    }

    pub(crate) fn http_status(&self) -> StatusCode {
        match self {
            AppError::InvalidAdminCredentials | AppError::Unauthorized | AppError::TokenExpired => {
                StatusCode::UNAUTHORIZED
            }
            AppError::AccountDisabled | AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::UserNotFound(_) => StatusCode::NOT_FOUND,
            AppError::RoomAlreadyClosed => StatusCode::CONFLICT,
            AppError::UserAlreadyBanned | AppError::UserAlreadyNormal => StatusCode::CONFLICT,
            AppError::DuplicateCode(_) => StatusCode::CONFLICT,
            AppError::ValidationError(_) | AppError::InsufficientBalance => StatusCode::BAD_REQUEST,
            AppError::DatabaseError(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn safe_message(&self) -> String {
        match self {
            AppError::DatabaseError(e) => {
                tracing::error!(detail = %e, "database error");
                "internal server error".to_string()
            }
            AppError::Internal(e) => {
                tracing::error!(detail = %e, "internal error");
                "internal server error".to_string()
            }
            _ => self.to_string(),
        }
    }

    /// 构造携带 request_id 的 JSON 拒绝响应（用于 FromRequestParts）。
    pub fn into_rejection_with_id(
        self,
        request_id: &str,
    ) -> (StatusCode, axum::Json<serde_json::Value>) {
        let status = self.http_status();
        let code = self.error_code() as i32;
        let message = self.safe_message();
        (
            status,
            axum::Json(serde_json::json!({
                "code": code,
                "message": message,
                "request_id": request_id,
            })),
        )
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::DatabaseError(e.to_string())
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
        err_response(self, "")
    }
}

/// 将 AppError 序列化为带 request_id 的 JSON 错误响应。
/// 所有 controller 错误路径必须调用此函数以填充 request_id。
pub fn err_response(err: AppError, request_id: &str) -> Response {
    let code = err.error_code() as i32;
    let message = err.safe_message();
    let body = ErrorBody {
        code,
        message,
        request_id: request_id.to_string(),
    };
    (err.http_status(), Json(body)).into_response()
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// T-10002: 凭证无效 → HTTP 401，错误码 40106
    #[test]
    fn invalid_admin_credentials_maps_to_401_40106() {
        let err = AppError::InvalidAdminCredentials;
        assert_eq!(err.http_status(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.error_code() as i32, 40106);
    }

    /// T-10003: JWT 无效 → HTTP 401，错误码 40101
    #[test]
    fn unauthorized_maps_to_401_40101() {
        let err = AppError::Unauthorized;
        assert_eq!(err.http_status(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.error_code() as i32, 40101);
    }

    /// T-10003: JWT 过期 → HTTP 401，错误码 40102
    #[test]
    fn token_expired_maps_to_401_40102() {
        let err = AppError::TokenExpired;
        assert_eq!(err.http_status(), StatusCode::UNAUTHORIZED);
        assert_eq!(err.error_code() as i32, 40102);
    }

    /// T-10002: 账号禁用 → HTTP 403，错误码 40302
    #[test]
    fn account_disabled_maps_to_403_40302() {
        let err = AppError::AccountDisabled;
        assert_eq!(err.http_status(), StatusCode::FORBIDDEN);
        assert_eq!(err.error_code() as i32, 40302);
    }

    /// T-10003: RBAC 权限不足 → HTTP 403，错误码 40301
    #[test]
    fn forbidden_maps_to_403_40301() {
        let err = AppError::Forbidden;
        assert_eq!(err.http_status(), StatusCode::FORBIDDEN);
        assert_eq!(err.error_code() as i32, 40301);
    }

    /// 内部错误 → HTTP 500，错误码 50000
    #[test]
    fn internal_error_maps_to_500_50000() {
        let err = AppError::Internal("test".to_string());
        assert_eq!(err.http_status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_code() as i32, 50000);
    }

    /// 数据库错误 → HTTP 500，错误码 50000
    #[test]
    fn database_error_maps_to_500_50000() {
        let err = AppError::DatabaseError("connection refused".to_string());
        assert_eq!(err.http_status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.error_code() as i32, 50000);
    }

    /// T-10004: 参数校验失败 → HTTP 400，错误码 40003
    #[test]
    fn validation_error_maps_to_400_40003() {
        let err = AppError::ValidationError("page must be >= 1".to_string());
        assert_eq!(err.http_status(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code() as i32, 40003);
    }

    // ─────────────────────────── T-10005 新增测试 ─────────────────────────────

    /// E-01 (T-10005): NotFound → HTTP 404，错误码 40400
    #[test]
    fn e01_not_found_maps_to_404_40400() {
        let err = AppError::NotFound("room not found".to_string());
        assert_eq!(err.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_code() as i32, 40400);
    }

    // ─────────────────────────── T-10006 新增测试 ─────────────────────────────

    /// E-01 (T-10006): RoomAlreadyClosed → HTTP 409，错误码 40901
    #[test]
    fn e01_t10006_room_already_closed_maps_to_409_40901() {
        let err = AppError::RoomAlreadyClosed;
        assert_eq!(err.http_status(), StatusCode::CONFLICT);
        assert_eq!(err.error_code() as i32, 40901);
    }

    /// E-01 (T-10009): UserAlreadyBanned → HTTP 409，错误码 40900
    #[test]
    fn e01_t10009_user_already_banned_maps_to_409_40900() {
        let err = AppError::UserAlreadyBanned;
        assert_eq!(err.http_status(), StatusCode::CONFLICT);
        assert_eq!(err.error_code() as i32, 40900);
    }

    /// E-02 (T-10009): UserAlreadyNormal → HTTP 409，错误码 40900
    #[test]
    fn e02_t10009_user_already_normal_maps_to_409_40900() {
        let err = AppError::UserAlreadyNormal;
        assert_eq!(err.http_status(), StatusCode::CONFLICT);
        assert_eq!(err.error_code() as i32, 40900);
    }
}
