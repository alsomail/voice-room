use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
};
use jsonwebtoken::errors::ErrorKind;
use uuid::Uuid;
use voice_room_shared::jwt::token::{decode_token, AppClaims};

use crate::{bootstrap::AppState, common::error::AppError, common::RequestContext};

use crate::common::auth::AuthContext;

/// 从 `Authorization: Bearer <token>` 头提取 JWT 并注入 AuthContext。
/// 参见 doc/tds/server/T-00004.md 验收用例。
impl FromRequestParts<AppState> for AuthContext {
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // request_context_middleware 已在 extensions 中注入 RequestContext，
        // 从此处提取真实 request_id，满足 protocol §1.3 要求。
        let request_id = parts
            .extensions
            .get::<RequestContext>()
            .map(|rc| rc.request_id().to_string())
            .unwrap_or_default();
        extract_auth_context(&parts.headers, &state.jwt_secret)
            .map_err(|e| e.into_rejection_with_id(&request_id))
    }
}

/// 纯函数：从 HeaderMap + secret 解析 AuthContext，方便单元测试。
pub fn extract_auth_context(
    headers: &HeaderMap,
    jwt_secret: &str,
) -> Result<AuthContext, AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let claims: AppClaims =
        decode_token(token, jwt_secret.as_bytes(), "voiceroom").map_err(|e| {
            if e.kind() == &ErrorKind::ExpiredSignature {
                AppError::TokenExpired
            } else {
                AppError::Unauthorized
            }
        })?;

    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;

    Ok(AuthContext { user_id })
}

impl AppError {
    pub(crate) fn into_rejection_with_id(
        self,
        request_id: &str,
    ) -> (StatusCode, axum::Json<serde_json::Value>) {
        use voice_room_shared::error::code::ErrorCode;
        let (status, code, message) = match &self {
            AppError::TokenExpired => (
                StatusCode::UNAUTHORIZED,
                ErrorCode::TokenExpired as i32,
                self.to_string(),
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized as i32,
                self.to_string(),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::InternalError as i32,
                self.to_string(),
            ),
        };
        (
            status,
            axum::Json(serde_json::json!({
                "code": code,
                "message": message,
                "request_id": request_id
            })),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::time::{SystemTime, UNIX_EPOCH};
    use voice_room_shared::jwt::token::{encode_token, AppClaims};

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn make_valid_token(secret: &[u8]) -> String {
        let claims = AppClaims {
            sub: "550e8400-e29b-41d4-a716-446655440000".into(),
            iss: "voiceroom".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        encode_token(&claims, secret).unwrap()
    }

    fn bearer_headers(token: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        h
    }

    // T-00004 验收用例：正向 - 合法 token 注入 user_id
    #[test]
    fn valid_token_injects_user_id() {
        let secret = b"test-secret";
        let token = make_valid_token(secret);
        let headers = bearer_headers(&token);
        let ctx = extract_auth_context(&headers, "test-secret").unwrap();
        assert_eq!(
            ctx.user_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    // T-00004 验收用例：异常 - 无 Authorization Header → 401 40101
    #[test]
    fn missing_auth_header_returns_unauthorized() {
        let headers = HeaderMap::new();
        let err = extract_auth_context(&headers, "secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
        assert_eq!(
            voice_room_shared::error::code::ErrorCode::Unauthorized as i32,
            40101
        );
    }

    // T-00004 验收用例：异常 - 非 "Bearer xxx" 格式 → 401
    #[test]
    fn invalid_bearer_format_returns_unauthorized() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Token abc123"),
        );
        let err = extract_auth_context(&headers, "secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // T-00004 验收用例：异常 - 签名无效 → 401
    #[test]
    fn invalid_signature_returns_unauthorized() {
        let token = make_valid_token(b"correct-secret");
        let headers = bearer_headers(&token);
        let err = extract_auth_context(&headers, "wrong-secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // T-00004 验收用例：异常 - token 过期 → 401 40102
    #[test]
    fn expired_token_returns_token_expired() {
        let secret = b"test-secret";
        let claims = AppClaims {
            sub: "user-123".into(),
            iss: "voiceroom".into(),
            exp: now_secs() - 120, // 已过期 (超过 jsonwebtoken 60s leeway)
            iat: now_secs() - 180,
        };
        let token = encode_token(&claims, secret).unwrap();
        let headers = bearer_headers(&token);
        let err = extract_auth_context(&headers, "test-secret").unwrap_err();
        assert!(matches!(err, AppError::TokenExpired));
    }

    // T-00004 验收用例：异常 - iss 不是 "voiceroom" → 401
    #[test]
    fn wrong_iss_returns_unauthorized() {
        use voice_room_shared::jwt::token::{encode_token, AdminClaims};
        let secret = b"test-secret";
        let claims = AdminClaims {
            sub: "admin-001".into(),
            role: "super_admin".into(),
            iss: "voiceroom-admin".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        let token = encode_token(&claims, secret).unwrap();
        let headers = bearer_headers(&token);
        let err = extract_auth_context(&headers, "test-secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }
}
