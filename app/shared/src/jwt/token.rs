use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AppClaims {
    pub sub: String, // user_id
    pub iss: String, // "voiceroom"
    pub exp: u64,
    pub iat: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminClaims {
    pub sub: String,  // admin_id
    pub role: String, // "super_admin" | "operator" | "cs" | "finance"
    pub iss: String,  // "voiceroom-admin"
    pub exp: u64,
    pub iat: u64,
}

pub fn encode_token<T: Serialize>(
    claims: &T,
    secret: &[u8],
) -> Result<String, jsonwebtoken::errors::Error> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret),
    )
}

/// 解码并验证 JWT。
/// `expected_iss` 必须与 token 中的 `iss` 字段完全一致，防止跨角色 token 滥用。
pub fn decode_token<T: for<'de> Deserialize<'de>>(
    token: &str,
    secret: &[u8],
    expected_iss: &str,
) -> Result<T, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.set_issuer(&[expected_iss]);
    let data = decode::<T>(token, &DecodingKey::from_secret(secret), &validation)?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn app_claims_roundtrip() {
        let secret = b"test-secret-key";
        let claims = AppClaims {
            sub: "user-123".into(),
            iss: "voiceroom".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        let token = encode_token(&claims, secret).expect("encode should succeed");
        let decoded: AppClaims =
            decode_token(&token, secret, "voiceroom").expect("decode should succeed");
        assert_eq!(decoded.sub, "user-123");
        assert_eq!(decoded.iss, "voiceroom");
    }

    #[test]
    fn admin_claims_roundtrip() {
        let secret = b"admin-secret";
        let claims = AdminClaims {
            sub: "admin-456".into(),
            role: "super_admin".into(),
            iss: "voiceroom-admin".into(),
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        let token = encode_token(&claims, secret).expect("encode should succeed");
        let decoded: AdminClaims =
            decode_token(&token, secret, "voiceroom-admin").expect("decode should succeed");
        assert_eq!(decoded.sub, "admin-456");
        assert_eq!(decoded.role, "super_admin");
    }

    #[test]
    fn expired_token_returns_error() {
        let secret = b"test-secret-key";
        let claims = AppClaims {
            sub: "user-999".into(),
            iss: "voiceroom".into(),
            exp: now_secs() - 120, // 已过期，超过 jsonwebtoken 默认 60s leeway
            iat: now_secs() - 180,
        };
        let token = encode_token(&claims, secret).expect("encode should succeed");
        let result: Result<AppClaims, _> = decode_token(&token, secret, "voiceroom");
        assert!(result.is_err(), "expired token should return error");
    }

    #[test]
    fn wrong_iss_returns_error() {
        let secret = b"test-secret-key";
        let claims = AdminClaims {
            sub: "admin-001".into(),
            role: "super_admin".into(),
            iss: "voiceroom-admin".into(), // admin token
            exp: now_secs() + 3600,
            iat: now_secs(),
        };
        let token = encode_token(&claims, secret).expect("encode should succeed");
        // 用 app iss 解码 admin token，必须拒绝
        let result: Result<AdminClaims, _> = decode_token(&token, secret, "voiceroom");
        assert!(result.is_err(), "wrong iss should be rejected");
    }
}
