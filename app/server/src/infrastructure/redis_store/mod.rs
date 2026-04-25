use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client as RedisClient, Script};

use crate::common::error::AppError;

/// Redis key 前缀，来自 protocol.md §6.2
const SMS_CODE_KEY: &str = "sms:code:";
const SMS_COOLDOWN_KEY: &str = "sms:cooldown:";
const SMS_DAILY_KEY_PREFIX: &str = "sms:daily:";
const CODE_TTL_SECS: u64 = 300;
const COOLDOWN_TTL_SECS: u64 = 60;
const DAILY_TTL_SECS: u64 = 86400;
const MAX_ATTEMPTS: u32 = 5;
const MAX_DAILY: u64 = 10;

/// Lua 原子脚本：检查冷却/日限并写入验证码（H-03）
/// KEYS[1]=cooldown_key KEYS[2]=daily_key KEYS[3]=code_key
/// ARGV[1]=code ARGV[2]=max_daily ARGV[3]=ttl_code ARGV[4]=ttl_cool ARGV[5]=ttl_daily
/// 错误前缀 VR: 确保不与 Redis 内部错误混淆（M-01）
const SAVE_CODE_LUA: &str = r#"
local cooldown_key = KEYS[1]
local daily_key    = KEYS[2]
local code_key     = KEYS[3]
local code         = ARGV[1]
local max_daily    = tonumber(ARGV[2])
local ttl_code     = tonumber(ARGV[3])
local ttl_cool     = tonumber(ARGV[4])
local ttl_daily    = tonumber(ARGV[5])

if redis.call('EXISTS', cooldown_key) == 1 then
    return redis.error_reply('VR:COOLDOWN')
end

local cnt = tonumber(redis.call('GET', daily_key) or '0') or 0
if cnt >= max_daily then
    return redis.error_reply('VR:DAILY_LIMIT')
end

redis.call('HSET', code_key, 'code', code, 'attempts', '0')
redis.call('EXPIRE', code_key, ttl_code)
redis.call('SET', cooldown_key, '1', 'EX', ttl_cool)
redis.call('INCR', daily_key)
redis.call('EXPIRE', daily_key, ttl_daily)
return redis.status_reply('OK')
"#;

/// Lua 原子脚本：校验并消费验证码（H-01）
/// KEYS[1]=code_key ARGV[1]=input_code ARGV[2]=max_attempts
/// 原子化三步：HGETALL → 判断 → HSET/DEL，消除并发双重消费风险
const VERIFY_CODE_LUA: &str = r#"
local code_key    = KEYS[1]
local input       = ARGV[1]
local max_attempts = tonumber(ARGV[2])

local map = redis.call('HGETALL', code_key)
if #map == 0 then
    return redis.error_reply('VR:EXPIRED')
end

local data = {}
for i = 1, #map, 2 do
    data[map[i]] = map[i + 1]
end

local attempts = tonumber(data['attempts'] or '0') or 0
if attempts >= max_attempts then
    redis.call('DEL', code_key)
    return redis.error_reply('VR:MAX_ATTEMPTS')
end

if data['code'] ~= input then
    redis.call('HSET', code_key, 'attempts', attempts + 1)
    return redis.error_reply('VR:INVALID')
end

redis.call('DEL', code_key)
return redis.status_reply('OK')
"#;

/// 短信验证码存储 trait
#[async_trait]
pub trait SmsCodeStore: Send + Sync {
    /// 保存验证码；若处于冷却期则返回 Err(TooManyRequests)
    async fn save_code(&self, phone: &str, code: &str, today: &str) -> Result<(), AppError>;
    /// 校验验证码；次数用尽或不匹配返回 Err；成功后删除 key
    async fn verify_and_consume(&self, phone: &str, input: &str) -> Result<(), AppError>;
    /// 是否处于冷却期
    async fn is_in_cooldown(&self, phone: &str) -> Result<bool, AppError>;
    /// 当日发送次数
    async fn daily_count(&self, phone: &str, today: &str) -> Result<u64, AppError>;
    /// SMS 发送失败时撤销预留：删除 code_key + cooldown_key；daily count 保留（防滥用）
    async fn revoke_code(&self, phone: &str) -> Result<(), AppError>;
}

// ─── Redis 实现 ───────────────────────────────────────────────────────────────

/// `MultiplexedConnection` 是 Clone 的，内部共享同一 TCP 连接，每次操作 clone 避免 &mut 竞争。
pub struct RedisCodeStore {
    conn: MultiplexedConnection,
}

impl RedisCodeStore {
    pub async fn new(redis_url: &str) -> Result<Self, AppError> {
        let client = RedisClient::open(redis_url)
            .map_err(|e| AppError::Internal(format!("redis open: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::Internal(format!("redis conn: {e}")))?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl SmsCodeStore for RedisCodeStore {
    async fn save_code(&self, phone: &str, code: &str, today: &str) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        let cooldown_key = format!("{SMS_COOLDOWN_KEY}{phone}");
        let daily_key = format!("{SMS_DAILY_KEY_PREFIX}{phone}:{today}");
        let code_key = format!("{SMS_CODE_KEY}{phone}");

        let result: redis::RedisResult<redis::Value> = Script::new(SAVE_CODE_LUA)
            .key(&cooldown_key)
            .key(&daily_key)
            .key(&code_key)
            .arg(code)
            .arg(MAX_DAILY.to_string())
            .arg(CODE_TTL_SECS.to_string())
            .arg(COOLDOWN_TTL_SECS.to_string())
            .arg(DAILY_TTL_SECS.to_string())
            .invoke_async(&mut conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("VR:COOLDOWN") {
                    Err(AppError::VerificationCodeCooldown)
                } else if msg.contains("VR:DAILY_LIMIT") {
                    Err(AppError::VerificationCodeDailyLimit)
                } else {
                    Err(AppError::RedisError(msg))
                }
            }
        }
    }

    async fn verify_and_consume(&self, phone: &str, input: &str) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        let code_key = format!("{SMS_CODE_KEY}{phone}");

        let result: redis::RedisResult<redis::Value> = Script::new(VERIFY_CODE_LUA)
            .key(&code_key)
            .arg(input)
            .arg(MAX_ATTEMPTS.to_string())
            .invoke_async(&mut conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("VR:EXPIRED") {
                    Err(AppError::VerificationCodeExpired)
                } else if msg.contains("VR:MAX_ATTEMPTS") {
                    Err(AppError::VerificationCodeMaxAttempts)
                } else if msg.contains("VR:INVALID") {
                    Err(AppError::InvalidVerificationCode)
                } else {
                    Err(AppError::RedisError(msg))
                }
            }
        }
    }

    async fn is_in_cooldown(&self, phone: &str) -> Result<bool, AppError> {
        let mut conn = self.conn.clone();
        let key = format!("{SMS_COOLDOWN_KEY}{phone}");
        conn.exists(&key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }

    async fn daily_count(&self, phone: &str, today: &str) -> Result<u64, AppError> {
        let mut conn = self.conn.clone();
        let key = format!("{SMS_DAILY_KEY_PREFIX}{phone}:{today}");
        let val: Option<u64> = conn
            .get(&key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(val.unwrap_or(0))
    }

    async fn revoke_code(&self, phone: &str) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        let code_key = format!("{SMS_CODE_KEY}{phone}");
        let cooldown_key = format!("{SMS_COOLDOWN_KEY}{phone}");
        conn.del(&[code_key, cooldown_key])
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))
    }
}

// ─── Fake 实现（内存，用于单元测试）─────────────────────────────────────────

#[derive(Default)]
pub struct FakeCodeStore {
    inner: Mutex<FakeStoreInner>,
}

#[derive(Default)]
struct FakeStoreInner {
    codes: HashMap<String, String>,
    attempts: HashMap<String, u32>,
    cooldowns: HashMap<String, bool>,
    daily: HashMap<String, u64>,
}

#[async_trait]
impl SmsCodeStore for FakeCodeStore {
    async fn save_code(&self, phone: &str, code: &str, today: &str) -> Result<(), AppError> {
        let mut inner = self.inner.lock().unwrap();
        if *inner.cooldowns.get(phone).unwrap_or(&false) {
            return Err(AppError::VerificationCodeCooldown);
        }
        let daily_key = format!("{phone}:{today}");
        let count = inner.daily.get(&daily_key).copied().unwrap_or(0);
        if count >= MAX_DAILY {
            return Err(AppError::VerificationCodeDailyLimit);
        }
        inner.codes.insert(phone.to_string(), code.to_string());
        inner.attempts.insert(phone.to_string(), 0);
        inner.cooldowns.insert(phone.to_string(), true);
        *inner.daily.entry(daily_key).or_insert(0) += 1;
        Ok(())
    }

    async fn verify_and_consume(&self, phone: &str, input: &str) -> Result<(), AppError> {
        let mut inner = self.inner.lock().unwrap();
        let stored = inner
            .codes
            .get(phone)
            .cloned()
            .ok_or(AppError::VerificationCodeExpired)?;
        let attempts = inner.attempts.entry(phone.to_string()).or_insert(0);
        if *attempts >= MAX_ATTEMPTS {
            inner.codes.remove(phone);
            return Err(AppError::VerificationCodeMaxAttempts);
        }
        if stored != input {
            *attempts += 1;
            return Err(AppError::InvalidVerificationCode);
        }
        inner.codes.remove(phone);
        inner.attempts.remove(phone);
        Ok(())
    }

    async fn is_in_cooldown(&self, phone: &str) -> Result<bool, AppError> {
        Ok(*self
            .inner
            .lock()
            .unwrap()
            .cooldowns
            .get(phone)
            .unwrap_or(&false))
    }

    async fn daily_count(&self, phone: &str, today: &str) -> Result<u64, AppError> {
        let key = format!("{phone}:{today}");
        Ok(self
            .inner
            .lock()
            .unwrap()
            .daily
            .get(&key)
            .copied()
            .unwrap_or(0))
    }

    async fn revoke_code(&self, phone: &str) -> Result<(), AppError> {
        let mut inner = self.inner.lock().unwrap();
        inner.codes.remove(phone);
        inner.attempts.remove(phone);
        inner.cooldowns.insert(phone.to_string(), false);
        Ok(())
    }
}

impl FakeCodeStore {
    /// 测试辅助：直接写入一个验证码
    pub fn seed_code(&self, phone: &str, code: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.codes.insert(phone.to_string(), code.to_string());
        inner.attempts.insert(phone.to_string(), 0);
    }

    /// 测试辅助：设置冷却状态
    pub fn set_cooldown(&self, phone: &str, value: bool) {
        self.inner
            .lock()
            .unwrap()
            .cooldowns
            .insert(phone.to_string(), value);
    }

    /// 测试辅助：设置当日次数
    pub fn set_daily_count(&self, phone: &str, today: &str, count: u64) {
        let key = format!("{phone}:{today}");
        self.inner.lock().unwrap().daily.insert(key, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> FakeCodeStore {
        FakeCodeStore::default()
    }

    /// H-01 行为契约：verify_and_consume 消费后 key 不存在，第二次调用返回 Expired
    #[tokio::test]
    async fn verify_and_consume_reuse_code_returns_expired() {
        let store = make_store();
        store.seed_code("+8613800138000", "123456");

        // 第一次：成功消费
        store
            .verify_and_consume("+8613800138000", "123456")
            .await
            .unwrap();

        // 第二次：同一 OTP 必须拒绝（验证原子性契约）
        let err = store
            .verify_and_consume("+8613800138000", "123456")
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::VerificationCodeExpired),
            "second call with same code must return Expired, got: {err:?}"
        );
    }

    /// H-01 行为契约：错误码不消耗 key，正确码才消耗
    #[tokio::test]
    async fn verify_and_consume_wrong_then_right_works() {
        let store = make_store();
        store.seed_code("+8613800138001", "999999");

        // 先错后对：错误的不消耗
        let err = store
            .verify_and_consume("+8613800138001", "000000")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::InvalidVerificationCode));

        // 正确的可以消耗
        store
            .verify_and_consume("+8613800138001", "999999")
            .await
            .unwrap();

        // 此后不可再用
        let err2 = store
            .verify_and_consume("+8613800138001", "999999")
            .await
            .unwrap_err();
        assert!(matches!(err2, AppError::VerificationCodeExpired));
    }

    /// H-02 契约：daily_count 应该对有效 key 返回正确计数
    #[tokio::test]
    async fn daily_count_returns_correct_value() {
        let store = make_store();
        let today = "2026-01-01";
        let phone = "+8613800138002";

        // 初始为 0
        assert_eq!(store.daily_count(phone, today).await.unwrap(), 0);

        // 直接设置计数后验证读取正确
        store.seed_code(phone, "111111");
        store.set_daily_count(phone, today, 3);
        assert_eq!(store.daily_count(phone, today).await.unwrap(), 3);
    }

    /// M-02 契约：revoke_code 清除 code + cooldown，不清除 daily count
    #[tokio::test]
    async fn revoke_code_clears_code_and_cooldown_keeps_daily() {
        let store = make_store();
        let today = "2026-01-01";
        let phone = "+8613800138003";

        store.seed_code(phone, "999999");
        store.set_cooldown(phone, true);
        store.set_daily_count(phone, today, 1);

        store.revoke_code(phone).await.unwrap();

        // code 已撤销
        let err = store.verify_and_consume(phone, "999999").await.unwrap_err();
        assert!(
            matches!(err, AppError::VerificationCodeExpired),
            "code must be gone after revoke"
        );

        // cooldown 已清除
        assert!(
            !store.is_in_cooldown(phone).await.unwrap(),
            "cooldown must be cleared after revoke"
        );

        // daily count 保留（防滥用）
        assert_eq!(
            store.daily_count(phone, today).await.unwrap(),
            1,
            "daily count must remain after revoke"
        );
    }
}
