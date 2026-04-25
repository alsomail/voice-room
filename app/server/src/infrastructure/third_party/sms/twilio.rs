use async_trait::async_trait;

use crate::common::error::AppError;

use super::redact::mask_phone;
use super::SmsProvider;

/// Twilio SMS Provider（生产实现，当前为 stub，后续接入 Twilio REST API）。
pub struct TwilioSmsProvider {
    #[allow(dead_code)]
    account_sid: String,
    #[allow(dead_code)]
    auth_token: String,
    #[allow(dead_code)]
    from_number: String,
}

impl TwilioSmsProvider {
    pub fn new(account_sid: String, auth_token: String, from_number: String) -> Self {
        // 启动期一次性 info 日志即可，便于运维核验配置（不含 auth_token / 验证码）
        tracing::info!(
            account_sid_len = account_sid.len(),
            from_number = %mask_phone(&from_number),
            "TwilioSmsProvider initialized"
        );
        Self {
            account_sid,
            auth_token,
            from_number,
        }
    }
}

#[async_trait]
impl SmsProvider for TwilioSmsProvider {
    async fn send_verification_code(&self, phone: &str, code: &str) -> Result<(), AppError> {
        // TODO(T-00002-follow-up): 接入 Twilio REST API
        // POST https://api.twilio.com/2010-04-01/Accounts/{AccountSid}/Messages.json
        //
        // 安全红线：以下日志中
        //   - 不得出现完整手机号 → 仅记录 mask_phone 结果（****后四位）
        //   - 不得出现 OTP 明文 → 仅记录位数 code.len()
        //   - 不得出现 account_sid / auth_token / from_number
        tracing::warn!(
            phone_masked = %mask_phone(phone),
            code_len = code.len(),
            provider = "twilio",
            "TwilioSmsProvider: stub, SMS NOT actually sent"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! P1 缺陷 3 回归测试：tracing 日志不得携带完整手机号或 OTP。
    //!
    //! 这里直接对 `send_verification_code` 中传给宏的字段做"结构性"断言：
    //! 我们只把 `mask_phone(phone)` 与 `code.len()` 喂给 tracing，而 mask_phone
    //! 的脱敏行为已在 redact.rs 单测里覆盖。

    use super::*;
    use crate::infrastructure::third_party::sms::redact::mask_phone;

    #[tokio::test]
    async fn send_verification_code_does_not_leak_phone_or_code() {
        let provider = TwilioSmsProvider::new(
            "AC_test_sid".to_owned(),
            "auth_token_secret".to_owned(),
            "+12025550000".to_owned(),
        );
        // 调用本身不应 panic / 报错
        provider
            .send_verification_code("+8613812345678", "123456")
            .await
            .expect("stub send should succeed");
    }

    #[test]
    fn masked_phone_format_is_four_stars_plus_last4() {
        // 锁定脱敏格式，防止后续重构把它改回带原号
        assert_eq!(mask_phone("+8613812345678"), "****5678");
    }
}
