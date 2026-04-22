//! JWT encode/decode for room access tokens（T-00026）
//!
//! Claim 格式：
//! ```json
//! { "sub": "<user_id>", "room_id": "<room_id>", "iat": 1700000000, "exp": 1700000060, "iss": "voiceroom-room-access" }
//! ```
//! 有效期 60s，仅用于密码房进房二次鉴权。

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

pub const ROOM_ACCESS_ISS: &str = "voiceroom-room-access";
pub const ROOM_ACCESS_EXPIRY_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomAccessClaims {
    /// user_id（UUID 字符串）
    pub sub: String,
    /// room_id（UUID 字符串）
    pub room_id: String,
    /// 签发时间（Unix 秒）
    pub iat: u64,
    /// 过期时间（Unix 秒，= iat + 60）
    pub exp: u64,
    /// 固定值 "voiceroom-room-access"
    pub iss: String,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 签发 room access token（有效期 60s）
pub fn encode_room_access_token(
    user_id: uuid::Uuid,
    room_id: uuid::Uuid,
    secret: &[u8],
) -> Result<String, jsonwebtoken::errors::Error> {
    let iat = now_secs();
    let claims = RoomAccessClaims {
        sub: user_id.to_string(),
        room_id: room_id.to_string(),
        iat,
        exp: iat + ROOM_ACCESS_EXPIRY_SECS,
        iss: ROOM_ACCESS_ISS.to_string(),
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret))
}

/// 解码并验证 room access token
///
/// 校验：
/// - iss == "voiceroom-room-access"
/// - 未过期（exp > now，leeway = 0 严格校验）
pub fn decode_room_access_token(
    token: &str,
    secret: &[u8],
) -> Result<RoomAccessClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.set_issuer(&[ROOM_ACCESS_ISS]);
    validation.leeway = 0; // 严格过期校验，不允许任何宽松时间
    let data = decode::<RoomAccessClaims>(token, &DecodingKey::from_secret(secret), &validation)?;
    Ok(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    const TEST_SECRET: &[u8] = b"test-room-access-secret";

    // ── RA-01: 签发与解码往返 ──────────────────────────────────────────────

    #[test]
    fn ra01_encode_decode_roundtrip() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        let token =
            encode_room_access_token(user_id, room_id, TEST_SECRET).expect("encode should succeed");
        let claims =
            decode_room_access_token(&token, TEST_SECRET).expect("decode should succeed");

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.room_id, room_id.to_string());
        assert_eq!(claims.iss, ROOM_ACCESS_ISS);
        // exp should be ~60s after iat
        assert!(claims.exp > claims.iat);
        assert!(claims.exp - claims.iat <= ROOM_ACCESS_EXPIRY_SECS + 2);
    }

    // ── RA-02: 错误密钥解码失败 ───────────────────────────────────────────

    #[test]
    fn ra02_wrong_secret_returns_error() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        let token =
            encode_room_access_token(user_id, room_id, TEST_SECRET).expect("encode should succeed");
        let result = decode_room_access_token(&token, b"wrong-secret");

        assert!(result.is_err(), "wrong secret should fail decode");
    }

    // ── RA-03: 过期 token 解码失败 ───────────────────────────────────────

    #[test]
    fn ra03_expired_token_returns_error() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        // 手动构造一个已过期的 claims（exp = iat - 1）
        let iat = now_secs();
        let claims = RoomAccessClaims {
            sub: user_id.to_string(),
            room_id: room_id.to_string(),
            iat,
            exp: iat - 1, // 已过期
            iss: ROOM_ACCESS_ISS.to_string(),
        };
        use jsonwebtoken::{encode, Header, EncodingKey};
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(TEST_SECRET))
            .expect("encode");
        let result = decode_room_access_token(&token, TEST_SECRET);
        assert!(result.is_err(), "expired token should fail decode");
    }

    // ── RA-04: 错误 iss 解码失败 ─────────────────────────────────────────

    #[test]
    fn ra04_wrong_iss_returns_error() {
        let iat = now_secs();
        let claims = RoomAccessClaims {
            sub: Uuid::new_v4().to_string(),
            room_id: Uuid::new_v4().to_string(),
            iat,
            exp: iat + 3600,
            iss: "wrong-issuer".to_string(), // 错误 iss
        };
        use jsonwebtoken::{encode, Header, EncodingKey};
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(TEST_SECRET))
            .expect("encode");
        let result = decode_room_access_token(&token, TEST_SECRET);
        assert!(result.is_err(), "wrong iss should fail decode");
    }

    // ── RA-05: 无效 token 字符串解码失败 ─────────────────────────────────

    #[test]
    fn ra05_garbage_token_returns_error() {
        let result = decode_room_access_token("not-a-valid-jwt", TEST_SECRET);
        assert!(result.is_err());
    }

    // ── RA-06: 不同 user_id 不能伪造 ─────────────────────────────────────

    #[test]
    fn ra06_different_users_get_different_tokens() {
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        let t1 = encode_room_access_token(user1, room_id, TEST_SECRET).unwrap();
        let t2 = encode_room_access_token(user2, room_id, TEST_SECRET).unwrap();
        assert_ne!(t1, t2);
    }
}
