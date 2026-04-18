use async_trait::async_trait;

use crate::common::error::AppError;

use super::SmsProvider;

/// 测试用 Mock SMS Provider，固定返回成功，验证码回写到日志。
#[derive(Debug, Default)]
pub struct MockSmsProvider;

#[async_trait]
impl SmsProvider for MockSmsProvider {
    async fn send_verification_code(&self, phone: &str, code: &str) -> Result<(), AppError> {
        tracing::info!(%phone, %code, "MockSmsProvider: send_verification_code (no-op)");
        Ok(())
    }
}

/// 发送失败的 Mock，用于测试 SMS 异常路径。
#[derive(Debug, Default)]
pub struct FailingSmsProvider;

#[async_trait]
impl SmsProvider for FailingSmsProvider {
    async fn send_verification_code(&self, _phone: &str, _code: &str) -> Result<(), AppError> {
        Err(AppError::SmsSendFailed("mock failure".into()))
    }
}
