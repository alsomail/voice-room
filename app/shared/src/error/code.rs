use thiserror::Error;

/// 全局错误码，数值与 doc/protocol.md §1.4 保持一致。
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // 400 - 参数错误
    #[error("INVALID_PHONE_NUMBER")]
    InvalidPhoneNumber = 40001,
    #[error("PARAMETER_MISSING")]
    ParameterMissing = 40002,
    #[error("VALIDATION_ERROR")]
    ValidationError = 40003,
    #[error("INVALID_COUNT")]
    InvalidCount = 40004,
    #[error("NOT_PASSWORD_ROOM")]
    NotPasswordRoom = 40014,
    #[error("INSUFFICIENT_BALANCE")]
    InsufficientBalance = 40290,

    // 404 - 资源不存在（续）
    #[error("GIFT_NOT_AVAILABLE")]
    GiftNotAvailable = 40402,
    #[error("RECEIVER_UNAVAILABLE")]
    ReceiverUnavailable = 40403,

    // 401 - 认证错误
    #[error("UNAUTHORIZED")]
    Unauthorized = 40101,
    #[error("TOKEN_EXPIRED")]
    TokenExpired = 40102,
    #[error("INVALID_VERIFICATION_CODE")]
    InvalidVerificationCode = 40103,
    #[error("VERIFICATION_CODE_EXPIRED")]
    VerificationCodeExpired = 40104,
    #[error("VERIFICATION_CODE_MAX_ATTEMPTS")]
    VerificationCodeMaxAttempts = 40105,
    #[error("INVALID_ADMIN_CREDENTIALS")]
    InvalidAdminCredentials = 40106,

    // 403 - 权限错误
    #[error("FORBIDDEN")]
    Forbidden = 40301,
    #[error("ACCOUNT_DISABLED")]
    AccountDisabled = 40302,

    // 404 - 资源不存在
    #[error("NOT_FOUND")]
    NotFound = 40400,
    #[error("USER_NOT_FOUND")]
    UserNotFound = 40401,

    // 409 - 冲突
    #[error("CONFLICT")]
    Conflict = 40900,
    #[error("ROOM_ALREADY_CLOSED")]
    RoomAlreadyClosed = 40901,

    // 409 / 404 / 422 - E-09 贵族体系错误码 (§10.6)
    #[error("DOWNGRADE_NOT_ALLOWED")]
    DowngradeNotAllowed = 40911,
    #[error("INSUFFICIENT_NOBLE_BALANCE")]
    InsufficientNobleBalance = 40912,
    #[error("SAME_TIER_RENEWAL_OVERLAP")]
    SameTierRenewalOverlap = 40913,
    #[error("TIER_INACTIVE")]
    TierInactive = 40914,
    #[error("NOBLE_PRIVILEGE_BLOCKED")]
    NoblePrivilegeBlocked = 40915,
    #[error("RENEW_REMINDER_ACK_INVALID")]
    RenewReminderAckInvalid = 40916,
    #[error("PRIVILEGES_SCHEMA_INVALID")]
    PrivilegesSchemaInvalid = 40917,

    // 429 - 频率限制
    #[error("VERIFICATION_CODE_COOLDOWN")]
    VerificationCodeCooldown = 42901,
    #[error("VERIFICATION_CODE_DAILY_LIMIT")]
    VerificationCodeDailyLimit = 42902,
    #[error("PASSWORD_ROOM_LOCKED")]
    PasswordRoomLocked = 42910,

    // 500 - 服务端错误
    #[error("INTERNAL_ERROR")]
    InternalError = 50000,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_display() {
        assert_eq!(ErrorCode::Unauthorized.to_string(), "UNAUTHORIZED");
        assert_eq!(ErrorCode::TokenExpired.to_string(), "TOKEN_EXPIRED");
        assert_eq!(ErrorCode::NotFound.to_string(), "NOT_FOUND");
        assert_eq!(
            ErrorCode::InvalidVerificationCode.to_string(),
            "INVALID_VERIFICATION_CODE"
        );
    }

    #[test]
    fn error_code_numeric_values_match_protocol() {
        assert_eq!(ErrorCode::InvalidPhoneNumber as i32, 40001);
        assert_eq!(ErrorCode::Unauthorized as i32, 40101);
        assert_eq!(ErrorCode::TokenExpired as i32, 40102);
        assert_eq!(ErrorCode::InvalidVerificationCode as i32, 40103);
        assert_eq!(ErrorCode::VerificationCodeExpired as i32, 40104);
        assert_eq!(ErrorCode::VerificationCodeMaxAttempts as i32, 40105);
        assert_eq!(ErrorCode::VerificationCodeCooldown as i32, 42901);
        assert_eq!(ErrorCode::VerificationCodeDailyLimit as i32, 42902);
        assert_eq!(ErrorCode::InternalError as i32, 50000);
        // E-03: RoomAlreadyClosed 数值必须是 40901
        assert_eq!(ErrorCode::RoomAlreadyClosed as i32, 40901);
    }
}
