//! T-00025 字段校验器
//!
//! - `validate_cover_url`    — 白名单前缀检查
//! - `validate_password`     — 6 位数字正则
//! - `validate_announcement` — ≤200 Unicode 字符
//! - `validate_category`     — 6 类枚举

use crate::common::error::AppError;

/// 封面 URL 允许前缀白名单
pub const COVER_PREFIX_ALLOW: &[&str] = &[
    "/assets/covers/",
    "https://cdn.voiceroom.example/covers/",
];

/// 允许的房间分类枚举（6 类）
pub const VALID_CATEGORIES: &[&str] = &[
    "chat",
    "emotion",
    "music",
    "game",
    "matchmaking",
    "other",
];

/// 校验封面 URL：空串合法（无封面），否则必须匹配白名单前缀
pub fn validate_cover_url(url: &str) -> Result<(), AppError> {
    if url.is_empty() {
        return Ok(());
    }
    if COVER_PREFIX_ALLOW.iter().any(|p| url.starts_with(p)) {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "invalid cover_url: must start with one of {:?}",
            COVER_PREFIX_ALLOW
        )))
    }
}

/// 校验密码：必须是 6 位纯数字（`^\d{6}$`）
pub fn validate_password(pw: &str) -> Result<(), AppError> {
    if pw.len() == 6 && pw.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(AppError::ValidationError(
            "password must be exactly 6 digits (0-9)".to_string(),
        ))
    }
}

/// 校验公告：≤200 个 Unicode 字符
///
/// 空串合法（表示无公告；在 PATCH 操作中代表清空）
pub fn validate_announcement(text: &str) -> Result<(), AppError> {
    let len = text.chars().count();
    if len > 200 {
        Err(AppError::ValidationError(format!(
            "announcement must be at most 200 characters, got {len}"
        )))
    } else {
        Ok(())
    }
}

/// 校验分类：必须是 6 类枚举之一
pub fn validate_category(cat: &str) -> Result<(), AppError> {
    if VALID_CATEGORIES.contains(&cat) {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "category must be one of {:?}, got {:?}",
            VALID_CATEGORIES, cat
        )))
    }
}

// ─── 单元测试（T-00025 验收）─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_cover_url ───────────────────────────────────────────────────

    /// V-01: 空串合法（无封面）
    #[test]
    fn v01_empty_cover_url_is_valid() {
        assert!(validate_cover_url("").is_ok());
    }

    /// V-02: /assets/covers/ 前缀合法
    #[test]
    fn v02_assets_cover_prefix_is_valid() {
        assert!(validate_cover_url("/assets/covers/desert.png").is_ok());
        assert!(validate_cover_url("/assets/covers/night.jpg").is_ok());
    }

    /// V-03: CDN 白名单前缀合法
    #[test]
    fn v03_cdn_cover_prefix_is_valid() {
        assert!(
            validate_cover_url("https://cdn.voiceroom.example/covers/room1.jpg").is_ok()
        );
    }

    /// V-04: 非白名单 URL 返回 ValidationError（CR25-05）
    #[test]
    fn v04_invalid_cover_url_returns_error() {
        let err = validate_cover_url("https://evil.com/hack.png").unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    /// V-05: http:// 非白名单 → 错误
    #[test]
    fn v05_http_non_whitelist_returns_error() {
        assert!(validate_cover_url("http://example.com/cover.jpg").is_err());
    }

    /// V-05b: /assets/ 前缀但不匹配 covers/ → 错误
    #[test]
    fn v05b_assets_non_covers_path_returns_error() {
        assert!(validate_cover_url("/assets/other/image.png").is_err());
    }

    // ── validate_password ────────────────────────────────────────────────────

    /// V-06: 6 位纯数字合法（CR25-01, C-09 新要求）
    #[test]
    fn v06_six_digits_is_valid() {
        assert!(validate_password("123456").is_ok());
        assert!(validate_password("000000").is_ok());
        assert!(validate_password("999999").is_ok());
    }

    /// V-07: 5 位数字不合法（CR25-02）
    #[test]
    fn v07_five_digits_is_invalid() {
        let err = validate_password("12345").unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    /// V-08: 7 位数字不合法
    #[test]
    fn v08_seven_digits_is_invalid() {
        assert!(validate_password("1234567").is_err());
    }

    /// V-09: 6 位含字母不合法
    #[test]
    fn v09_six_chars_with_letter_is_invalid() {
        assert!(validate_password("12345a").is_err());
        assert!(validate_password("a12345").is_err());
    }

    /// V-10: 空串不合法
    #[test]
    fn v10_empty_password_is_invalid() {
        assert!(validate_password("").is_err());
    }

    /// V-11: 含特殊字符不合法
    #[test]
    fn v11_special_chars_is_invalid() {
        assert!(validate_password("12!456").is_err());
        assert!(validate_password("12 456").is_err());
    }

    // ── validate_announcement ────────────────────────────────────────────────

    /// V-12: 空串合法（表示无公告 / PATCH 清空）
    #[test]
    fn v12_empty_announcement_is_valid() {
        assert!(validate_announcement("").is_ok());
    }

    /// V-13: 200 个 Unicode 字符合法（边界 CR25-04 的反面）
    #[test]
    fn v13_200_unicode_chars_is_valid() {
        let text = "音".repeat(200);
        assert_eq!(text.chars().count(), 200);
        assert!(validate_announcement(&text).is_ok());
    }

    /// V-14: 201 个字符不合法（CR25-04）
    #[test]
    fn v14_201_unicode_chars_is_invalid() {
        let text = "音".repeat(201);
        let err = validate_announcement(&text).unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    /// V-15: 普通 ASCII 文本合法
    #[test]
    fn v15_ascii_text_is_valid() {
        assert!(validate_announcement("Hello World! Welcome to the room.").is_ok());
    }

    /// V-15b: 含 emoji 字符按 Unicode 标量值计算，≤200 合法
    #[test]
    fn v15b_emoji_announcement_is_valid() {
        let text = "🎵".repeat(50); // 50 个 emoji = 50 chars
        assert!(validate_announcement(&text).is_ok());
    }

    // ── validate_category ────────────────────────────────────────────────────

    /// V-16: 所有 6 个合法分类（CR25-03 的正向用例）
    #[test]
    fn v16_all_valid_categories() {
        for cat in &["chat", "emotion", "music", "game", "matchmaking", "other"] {
            assert!(
                validate_category(cat).is_ok(),
                "category {:?} should be valid",
                cat
            );
        }
    }

    /// V-17: 未知分类返回 ValidationError（CR25-03）
    #[test]
    fn v17_unknown_category_returns_error() {
        let err = validate_category("unknown").unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
    }

    /// V-18: 空串分类不合法
    #[test]
    fn v18_empty_category_returns_error() {
        assert!(validate_category("").is_err());
    }

    /// V-19: 大小写敏感 — "Chat"（首字母大写）不合法
    #[test]
    fn v19_case_sensitive_category() {
        assert!(validate_category("Chat").is_err());
        assert!(validate_category("MUSIC").is_err());
    }
}
