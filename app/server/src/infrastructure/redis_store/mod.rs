use std::{
    collections::HashMap,
    sync::Mutex,
};

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
    return redis.error_reply('COOLDOWN')
end

local cnt = tonumber(redis.call('GET', daily_key) or '0') or 0
if cnt >= max_daily then
    return redis.error_reply('DAILY_LIMIT')
end

redis.call('HSET', code_key, 'code', code, 'attempts', '0')
redis.call('EXPIRE', code_key, ttl_code)
redis.call('SET', cooldown_key, '1', 'EX', ttl_cool)
redis.call('INCR', daily_key)
redis.call('EXPIRE', daily_key, ttl_daily)
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
                if msg.contains("COOLDOWN") {
                    Err(AppError::VerificationCodeCooldown)
                } else if msg.contains("DAILY_LIMIT") {
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

        let map: HashMap<String, String> = conn.hgetall(&code_key).await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if map.is_empty() {
            return Err(AppError::VerificationCodeExpired);
        }

        let stored = map.get("code").ok_or(AppError::VerificationCodeExpired)?;
        let attempts: u32 = map.get("attempts").and_then(|v| v.parse().ok()).unwrap_or(0);

        if attempts >= MAX_ATTEMPTS {
            let _: () = conn.del(&code_key).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            return Err(AppError::VerificationCodeMaxAttempts);
        }

        if stored != input {
            let _: () = conn.hset(&code_key, "attempts", attempts + 1).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            return Err(AppError::InvalidVerificationCode);
        }

        let _: () = conn.del(&code_key).await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn is_in_cooldown(&self, phone: &str) -> Result<bool, AppError> {
        let mut conn = self.conn.clone();
        let key = format!("{SMS_COOLDOWN_KEY}{phone}");
        conn.exists(&key).await
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    async fn daily_count(&self, phone: &str, today: &str) -> Result<u64, AppError> {
        let mut conn = self.conn.clone();
        let key = format!("{SMS_DAILY_KEY_PREFIX}{phone}:{today}");
        Ok(conn.get(&key).await.unwrap_or(0))
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
        let stored = inner.codes.get(phone).cloned()
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
        Ok(*self.inner.lock().unwrap().cooldowns.get(phone).unwrap_or(&false))
    }

    async fn daily_count(&self, phone: &str, today: &str) -> Result<u64, AppError> {
        let key = format!("{phone}:{today}");
        Ok(self.inner.lock().unwrap().daily.get(&key).copied().unwrap_or(0))
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
        self.inner.lock().unwrap().cooldowns.insert(phone.to_string(), value);
    }

    /// 测试辅助：设置当日次数
    pub fn set_daily_count(&self, phone: &str, today: &str, count: u64) {
        let key = format!("{phone}:{today}");
        self.inner.lock().unwrap().daily.insert(key, count);
    }
}
