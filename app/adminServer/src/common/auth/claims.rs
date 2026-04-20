/// AdminClaims 直接从 shared crate 重导出，避免重复定义。
/// JWT payload: { sub: admin_id, role, iss: "voiceroom-admin", exp, iat }
pub use voice_room_shared::jwt::token::AdminClaims;
