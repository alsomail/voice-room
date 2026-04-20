# 八、Provider 配置模型

## 8.1 SMS Provider

通过环境变量 / `config/*.toml` 注入：

```toml
[sms]
provider = "twilio"           # twilio | aws_sns | mock
# Twilio 专属
twilio_account_sid = "${TWILIO_ACCOUNT_SID}"
twilio_auth_token = "${TWILIO_AUTH_TOKEN}"
twilio_from_number = "${TWILIO_FROM_NUMBER}"
```

- `mock` 模式在本地开发/测试环境使用，不实际发送短信，验证码固定为 `000000`
- Provider 实现在 `infrastructure/third_party/sms/` 目录下

## 8.2 RTC Provider（预留）

```toml
[rtc]
provider = "agora"            # agora | zego | mock
app_id = "${RTC_APP_ID}"
app_certificate = "${RTC_APP_CERTIFICATE}"
```

- Provider 实现在 `infrastructure/third_party/rtc/` 目录下
- `mock` 模式返回固定 token 字符串，供开发调试
