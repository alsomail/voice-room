use async_trait::async_trait;

use crate::common::error::AppError;

use super::SmsProvider;

/// Twilio SMS Provider（生产实现，当前为 stub，后续接入 Twilio REST API）。
pub struct TwilioSmsProvider {
    account_sid: String,
    #[allow(dead_code)]
    auth_token: String,
    from_number: String,
}

impl TwilioSmsProvider {
    pub fn new(account_sid: String, auth_token: String, from_number: String) -> Self {
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
        tracing::warn!(
            %phone,
            %code,
            account_sid = %self.account_sid,
            from = %self.from_number,
            "TwilioSmsProvider: stub, SMS NOT actually sent"
        );
        Ok(())
    }
}
