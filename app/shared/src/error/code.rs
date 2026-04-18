use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    #[error("INVALID_PARAM")]
    InvalidParam = 40000,
    #[error("UNAUTHORIZED")]
    Unauthorized = 40100,
    #[error("TOKEN_EXPIRED")]
    TokenExpired = 40101,
    #[error("FORBIDDEN")]
    Forbidden = 40300,
    #[error("NOT_FOUND")]
    NotFound = 40400,
    #[error("RATE_LIMITED")]
    RateLimited = 42900,
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
    }

    #[test]
    fn error_code_numeric_values() {
        assert_eq!(ErrorCode::Unauthorized as i32, 40100);
        assert_eq!(ErrorCode::InternalError as i32, 50000);
    }
}
