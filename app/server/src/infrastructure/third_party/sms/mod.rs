use async_trait::async_trait;

use crate::common::error::AppError;

/// 短信发送防腐层 trait，隔离 Twilio / 其他 SMS 平台。
/// 参见 doc/ARCHITECTURE.md §10 防腐层原则。
#[async_trait]
pub trait SmsProvider: Send + Sync {
    async fn send_verification_code(&self, phone: &str, code: &str) -> Result<(), AppError>;
}

pub mod mock;
pub mod twilio;

pub use mock::MockSmsProvider;
pub use twilio::TwilioSmsProvider;
