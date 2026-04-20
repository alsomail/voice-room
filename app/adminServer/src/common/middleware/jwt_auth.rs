use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, StatusCode},
};
use jsonwebtoken::errors::ErrorKind;
use uuid::Uuid;
use voice_room_shared::jwt::token::{decode_token, AdminClaims};

use crate::{
    bootstrap::AppState,
    common::{
        auth::AdminAuthContext,
        error::AppError,
        RequestContext,
    },
};

/// Axum `FromRequestParts` 实现：从 `Authorization: Bearer <token>` 头提取并校验
/// Admin JWT，成功后注入 `AdminAuthContext`（含 admin_id 和 role）。
///
/// 错误路径：
/// - 无 header / 格式非 Bearer → 401 (40101)
/// - 签名无效 / iss 不是 "voiceroom-admin" → 401 (40101)
/// - token 已过期 → 401 (40102)
impl FromRequestParts<AppState> for AdminAuthContext {
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

        extract_admin_auth_context(&parts.headers, &state.jwt_secret)
            .map_err(|e| e.into_rejection_with_id(&request_id))
    }
}

/// 纯函数：从 HeaderMap + secret 解析 AdminAuthContext，方便单元测试。
pub fn extract_admin_auth_context(
    headers: &HeaderMap,
    jwt_secret: &str,
) -> Result<AdminAuthContext, AppError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let claims: AdminClaims =
        decode_token(token, jwt_secret.as_bytes(), "voiceroom-admin").map_err(|e| {
            if e.kind() == &ErrorKind::ExpiredSignature {
                AppError::TokenExpired
            } else {
                AppError::Unauthorized
            }
        })?;

    let admin_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;

    Ok(AdminAuthContext::new(admin_id, claims.role))
}

// ─── 单元测试（TDD T-10003 验收用例）────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::time::{SystemTime, UNIX_EPOCH};
    use voice_room_shared::jwt::token::{encode_token, AdminClaims, AppClaims};

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn make_admin_token(secret: &[u8], role: &str) -> String {
        let claims = AdminClaims {
            sub: "550e8400-e29b-41d4-a716-446655440001".into(),
            role: role.into(),
            iss: "voiceroom-admin".into(),
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

    // T-10003-J01: 无 Authorization Header → 401 (40101)
    #[test]
    fn missing_auth_header_returns_unauthorized() {
        let headers = HeaderMap::new();
        let err = extract_admin_auth_context(&headers, "secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // T-10003-J02: 非 "Bearer xxx" 格式 → 401
    #[test]
    fn invalid_bearer_format_returns_unauthorized() {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Token abc123"),
        );
        let err = extract_admin_auth_context(&headers, "secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // T-10003-J03: 签名无效（wrong secret）→ 401
    #[test]
    fn invalid_signature_returns_unauthorized() {
        let token = make_admin_token(b"correct-secret", "operator");
        let headers = bearer_headers(&token);
        let err = extract_admin_auth_context(&headers, "wrong-secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // T-10003-J04: token 过期 → 401 (40102)
    #[test]
    fn expired_token_returns_token_expired() {
        let secret = b"test-secret";
        let claims = AdminClaims {
            sub: "550e8400-e29b-41d4-a716-446655440001".into(),
            role: "operator".into(),
            iss: "voiceroom-admin".into(),
            exp: now_secs() - 120, // 已过期（超过 jsonwebtoken 60s leeway）
            iat: now_secs() - 180,
        };
        let token = encode_token(&claims, secret).unwrap();
        let headers = bearer_headers(&token);
        let err = extract_admin_auth_context(&headers, "test-secret").unwrap_err();
        assert!(matches!(err, AppError::TokenExpired));
    }

    // T-10003-J05: 合法 Admin token → 注入 admin_id 和 role
    #[test]
    fn valid_admin_token_injects_admin_id_and_role() {
        let secret = b"test-secret";
        let token = make_admin_token(secret, "super_admin");
        let headers = bearer_headers(&token);
        let ctx = extract_admin_auth_context(&headers, "test-secret").unwrap();
        assert_eq!(
            ctx.admin_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440001"
        );
        assert_eq!(ctx.role, "super_admin");
    }

    // T-10003-J06: C 端 JWT（iss="voiceroom"）尝试访问 Admin 接口 → 401
    #[test]
    fn app_token_with_wrong_iss_returns_unauthorized() {
        let secret = b"test-secret";
        let claims = AppClaims {
            sub: "user-001".into(),
            iss: "voiceroom".into(), // C 端 iss
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        let token = encode_token(&claims, secret).unwrap();
        let headers = bearer_headers(&token);
        let err = extract_admin_auth_context(&headers, "test-secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // T-10003-J07: sub 非 UUID 格式 → 401
    #[test]
    fn non_uuid_sub_returns_unauthorized() {
        let secret = b"test-secret";
        let claims = AdminClaims {
            sub: "not-a-uuid".into(),
            role: "operator".into(),
            iss: "voiceroom-admin".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        let token = encode_token(&claims, secret).unwrap();
        let headers = bearer_headers(&token);
        let err = extract_admin_auth_context(&headers, "test-secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // T-10003-J08: operator role 的 token 也能正确注入
    #[test]
    fn operator_token_injects_correct_role() {
        let secret = b"test-secret";
        let token = make_admin_token(secret, "operator");
        let headers = bearer_headers(&token);
        let ctx = extract_admin_auth_context(&headers, "test-secret").unwrap();
        assert_eq!(ctx.role, "operator");
    }
}
