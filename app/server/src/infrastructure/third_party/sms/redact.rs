//! P1 修复 — SMS 防腐层敏感信息脱敏工具
//!
//! 缺陷背景（GlobalReview 第 1 轮 缺陷 3）：
//!   `TwilioSmsProvider::send_verification_code` 在 `tracing::warn!` 中输出了
//!   完整 `phone` 与 OTP `code`，违反"敏感信息泄露"红线。
//!
//! 本模块提供统一的脱敏函数，所有 `SmsProvider` 实现必须仅记录脱敏结果，
//! 严禁打印 OTP 明文。
//!
//! 约定：
//! - 手机号脱敏 → 仅保留末 4 位，前缀以 `****` 替换。例：`+8613812345678` → `****5678`。
//! - 验证码 OTP → 任何场景下都不记录明文；如需可观测性，仅记录长度（位数）。

/// 仅保留手机号末 4 个字符（按 `char` 计数，避免 emoji / 全角等多字节字符切割 panic），
/// 前缀以 `****` 表示。短于 4 位的输入直接全部脱敏为 `****`。
///
/// 不要把传入的 `phone` 直接落入日志 — 调用此函数的脱敏结果。
pub fn mask_phone(phone: &str) -> String {
    let chars: Vec<char> = phone.chars().collect();
    if chars.len() <= 4 {
        return "****".to_string();
    }
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("****{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_phone_keeps_last_four_digits() {
        assert_eq!(mask_phone("+8613812345678"), "****5678");
        assert_eq!(mask_phone("13800001111"), "****1111");
    }

    #[test]
    fn mask_phone_handles_short_inputs() {
        assert_eq!(mask_phone(""), "****");
        assert_eq!(mask_phone("12"), "****");
        assert_eq!(mask_phone("1234"), "****");
    }

    #[test]
    fn mask_phone_does_not_leak_full_number() {
        let original = "+8613812345678";
        let masked = mask_phone(original);
        // 关键不变量：脱敏结果中绝不能包含原始号码的前缀（即使是国家码 +86）
        assert!(
            !masked.contains("138"),
            "masked output must not contain leading digits, got {masked}"
        );
        assert!(!masked.contains("+86"));
    }

    #[test]
    fn mask_phone_handles_unicode_safely() {
        // 含 emoji / 中文不应 panic（按 char 边界切，而非 byte）
        let weird = "电话13812345678";
        let masked = mask_phone(weird);
        assert_eq!(masked, "****5678");
    }
}
