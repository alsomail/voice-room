//! Payment 错误类型
//!
//! 与通用 AppError 分离，直接承载 payment_api.md §9.6 定义的错误码。
//! 避免与现有 ErrorCode 枚举中的数值冲突（如 40901 已被 RoomAlreadyClosed 占用）。

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// Payment 专用错误类型（错误码严格匹配 payment_api.md §9.6）
#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    /// 40901 — Google 验签失败 / token 伪造 / obfuscatedAccountId 不一致
    #[error("INVALID_PURCHASE")]
    InvalidPurchase,

    /// 40902 — SKU 不存在或已下架
    #[error("SKU_DISABLED")]
    SkuDisabled,

    /// 40903 — 风控：日失败次数 > 10 / 设备黑名单
    #[error("ORDER_RISK_BLOCKED")]
    OrderRiskBlocked,

    /// 40904 — 订单不存在或不属于当前用户
    #[error("ORDER_NOT_FOUND")]
    OrderNotFound,

    /// 40905 — 订单已终态，无法继续操作
    #[error("ORDER_ALREADY_FINALIZED")]
    OrderAlreadyFinalized,

    /// 40906 — 同一 purchase_token 已被其它订单消费（per payment_api.md §9.6）
    #[error("TOKEN_REPLAY")]
    TokenReplay,

    /// 40907 — Google 返回金额与 SKU 配置严重不符（fraud 信号）
    #[error("AMOUNT_MISMATCH")]
    AmountMismatch,

    /// 40908 — 订单已过期（30min 未支付）
    #[error("ORDER_EXPIRED")]
    OrderExpired,

    /// 40909 — Google API 不可用
    #[error("GOOGLE_API_UNAVAILABLE")]
    GoogleApiUnavailable,

    /// 40910 — 生产环境调用 Dev Mock 通道（仅运行时检查）
    #[error("MOCK_NOT_ALLOWED")]
    MockNotAllowed,

    /// 40911 — RTDN Bearer OIDC token 无效（内部 webhook 鉴权码，不在公开 REST 错误码表内）
    #[error("RTDN_SIGNATURE_INVALID")]
    RtdnSignatureInvalid,

    /// 通用内部错误（向外暴露为 5xx）
    #[error("Internal error: {0}")]
    Internal(String),

    /// 数据库错误
    #[error("Database error: {0}")]
    Database(String),
}

#[derive(Serialize)]
struct PaymentErrorBody {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    request_id: String,
}

impl PaymentError {
    /// 错误码数值（payment_api.md §9.6）
    pub fn code(&self) -> i32 {
        match self {
            PaymentError::InvalidPurchase => 40901,
            PaymentError::SkuDisabled => 40902,
            PaymentError::OrderRiskBlocked => 40903,
            PaymentError::OrderNotFound => 40904,
            PaymentError::OrderAlreadyFinalized => 40905,
            PaymentError::TokenReplay => 40906,
            PaymentError::AmountMismatch => 40907,
            PaymentError::OrderExpired => 40908,
            PaymentError::GoogleApiUnavailable => 40909,
            PaymentError::MockNotAllowed => 40910,
            PaymentError::RtdnSignatureInvalid => 40911,
            PaymentError::Internal(_) | PaymentError::Database(_) => 50000,
        }
    }

    /// 对应 HTTP 状态码
    pub fn http_status(&self) -> StatusCode {
        match self {
            PaymentError::InvalidPurchase => StatusCode::CONFLICT,   // 409
            PaymentError::SkuDisabled => StatusCode::NOT_FOUND,      // 404
            PaymentError::OrderRiskBlocked => StatusCode::TOO_MANY_REQUESTS, // 429
            PaymentError::OrderNotFound => StatusCode::NOT_FOUND,    // 404
            PaymentError::OrderAlreadyFinalized => StatusCode::CONFLICT, // 409
            PaymentError::TokenReplay => StatusCode::CONFLICT,       // 409
            PaymentError::AmountMismatch => StatusCode::UNPROCESSABLE_ENTITY, // 422
            PaymentError::OrderExpired => StatusCode::CONFLICT,      // 409
            PaymentError::GoogleApiUnavailable => StatusCode::BAD_GATEWAY, // 502
            PaymentError::MockNotAllowed => StatusCode::FORBIDDEN,   // 403
            PaymentError::RtdnSignatureInvalid => StatusCode::UNAUTHORIZED, // 401
            PaymentError::Internal(_) | PaymentError::Database(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    pub fn with_request_id(self, request_id: &str) -> PaymentErrorWithId {
        PaymentErrorWithId {
            error: self,
            request_id: request_id.to_string(),
        }
    }
}

/// 带 request_id 的 PaymentError，实现 IntoResponse
pub struct PaymentErrorWithId {
    pub error: PaymentError,
    pub request_id: String,
}

impl IntoResponse for PaymentErrorWithId {
    fn into_response(self) -> Response {
        let safe_msg = match &self.error {
            PaymentError::Internal(e) => {
                tracing::error!(detail = %e, "payment internal error");
                "internal server error".to_string()
            }
            PaymentError::Database(e) => {
                tracing::error!(detail = %e, "payment database error");
                "internal server error".to_string()
            }
            other => other.to_string(),
        };
        let body = PaymentErrorBody {
            code: self.error.code(),
            message: safe_msg,
            request_id: self.request_id,
        };
        (self.error.http_status(), Json(body)).into_response()
    }
}

impl From<sqlx::Error> for PaymentError {
    fn from(e: sqlx::Error) -> Self {
        PaymentError::Database(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // PE01: 错误码数值正确（对齐 payment_api.md §9.6）
    #[test]
    fn pe01_error_codes_match_protocol() {
        assert_eq!(PaymentError::InvalidPurchase.code(), 40901);
        assert_eq!(PaymentError::SkuDisabled.code(), 40902);
        assert_eq!(PaymentError::OrderRiskBlocked.code(), 40903);
        assert_eq!(PaymentError::OrderNotFound.code(), 40904);
        assert_eq!(PaymentError::OrderAlreadyFinalized.code(), 40905);
        // 40906 = TOKEN_REPLAY (per §9.6 — purchase_token 已被其它订单消费)
        assert_eq!(PaymentError::TokenReplay.code(), 40906);
        // 40907 = AMOUNT_MISMATCH (per §9.6 — 金额与 SKU 不符)
        assert_eq!(PaymentError::AmountMismatch.code(), 40907);
        assert_eq!(PaymentError::OrderExpired.code(), 40908);
        assert_eq!(PaymentError::GoogleApiUnavailable.code(), 40909);
        assert_eq!(PaymentError::MockNotAllowed.code(), 40910);
        // 40911 = RTDN_SIGNATURE_INVALID (内部 webhook 鉴权，不在公开 REST 表内)
        assert_eq!(PaymentError::RtdnSignatureInvalid.code(), 40911);
    }

    // PE02: HTTP 状态码正确
    #[test]
    fn pe02_http_status_codes() {
        assert_eq!(PaymentError::SkuDisabled.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(PaymentError::OrderRiskBlocked.http_status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(PaymentError::OrderNotFound.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(PaymentError::InvalidPurchase.http_status(), StatusCode::CONFLICT);
        // TOKEN_REPLAY → 409 per protocol
        assert_eq!(PaymentError::TokenReplay.http_status(), StatusCode::CONFLICT);
        // AMOUNT_MISMATCH → 422 per protocol
        assert_eq!(PaymentError::AmountMismatch.http_status(), StatusCode::UNPROCESSABLE_ENTITY);
        // RTDN_SIGNATURE_INVALID → 401 Unauthorized (webhook OIDC auth)
        assert_eq!(
            PaymentError::RtdnSignatureInvalid.http_status(),
            StatusCode::UNAUTHORIZED
        );
    }

    // PE04 (RED→GREEN): TokenReplay 与协议完全对齐
    #[test]
    fn pe04_token_replay_aligns_with_protocol() {
        // §9.6: 40906 | 409 | TOKEN_REPLAY
        assert_eq!(PaymentError::TokenReplay.code(), 40906);
        assert_eq!(PaymentError::TokenReplay.http_status(), StatusCode::CONFLICT);
        assert_eq!(PaymentError::TokenReplay.to_string(), "TOKEN_REPLAY");
    }

    // PE05 (RED→GREEN): AmountMismatch 与协议完全对齐
    #[test]
    fn pe05_amount_mismatch_aligns_with_protocol() {
        // §9.6: 40907 | 422 | AMOUNT_MISMATCH
        assert_eq!(PaymentError::AmountMismatch.code(), 40907);
        assert_eq!(PaymentError::AmountMismatch.http_status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(PaymentError::AmountMismatch.to_string(), "AMOUNT_MISMATCH");
    }

    // PE06 (RED→GREEN): RtdnSignatureInvalid → 401 Unauthorized (OIDC webhook auth)
    #[test]
    fn pe06_rtdn_sig_invalid_is_401_unauthorized() {
        assert_eq!(PaymentError::RtdnSignatureInvalid.code(), 40911);
        assert_eq!(PaymentError::RtdnSignatureInvalid.http_status(), StatusCode::UNAUTHORIZED);
    }

    // PE03: Internal 错误有安全消息
    #[test]
    fn pe03_internal_error_display() {
        let err = PaymentError::Internal("secret db details".to_string());
        assert!(err.to_string().contains("Internal error"));
    }
}
