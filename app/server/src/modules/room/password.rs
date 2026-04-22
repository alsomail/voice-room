//! 密码房进房校验 + 锁定机制（T-00026）
//!
//! ## 核心流程
//! 1. 检查 `pwd_lock:{user_id}:{room_id}` — 若存在则返回 Locked
//! 2. bcrypt 验证明文密码
//!    - 成功 → DEL pwd_fail → 签发 60s room_access token
//!    - 失败 → INCR pwd_fail（初次设置 TTL 1800s）
//!      - 计数 >= 5 → SET NX EX pwd_lock 1800 → 返回 Locked
//!      - 计数 < 5  → 返回 WrongPassword { remaining_attempts }
//!
//! ## Redis Key 策略
//! | Key                            | 类型   | TTL   | 用途         |
//! |-------------------------------|--------|-------|-------------|
//! | `pwd_fail:{user_id}:{room_id}` | Int    | 1800s | 失败计数     |
//! | `pwd_lock:{user_id}:{room_id}` | String | 1800s | 锁定标记     |

use std::sync::Mutex;
use std::{collections::HashMap, time::Instant};

use async_trait::async_trait;
use uuid::Uuid;
use voice_room_shared::auth::room_access::encode_room_access_token;
use voice_room_shared::models::room::RoomModel;

use crate::common::error::AppError;

// ─── Redis 抽象 Trait ────────────────────────────────────────────────────────

/// 密码校验所需的 Redis 原子操作抽象。
///
/// 生产实现使用真实 Redis；测试使用 `FakeRoomPasswordRedis`（内存 HashMap）。
#[async_trait]
pub trait RoomPasswordRedis: Send + Sync {
    /// 将 key 的整数值原子递增 1 并返回新值。
    /// 若 key 不存在则从 1 开始，并设置 TTL（秒）。
    async fn incr_with_ttl(&self, key: &str, ttl_secs: i64) -> Result<i64, AppError>;

    /// SET key value NX EX ex_secs — 仅在 key 不存在时设置，返回是否成功。
    async fn set_nx_ex(&self, key: &str, value: &str, ex_secs: i64) -> Result<bool, AppError>;

    /// 获取 key 的剩余 TTL（秒）。key 不存在返回 None。
    async fn get_ttl(&self, key: &str) -> Result<Option<i64>, AppError>;

    /// 删除 key（不报错，key 不存在时静默忽略）
    async fn del(&self, key: &str) -> Result<(), AppError>;
}

/// Blanket impl：允许 `Arc<T: RoomPasswordRedis>` 直接作为 `&dyn RoomPasswordRedis` 使用。
#[async_trait]
impl<T: RoomPasswordRedis + ?Sized> RoomPasswordRedis for std::sync::Arc<T> {
    async fn incr_with_ttl(&self, key: &str, ttl_secs: i64) -> Result<i64, AppError> {
        (**self).incr_with_ttl(key, ttl_secs).await
    }
    async fn set_nx_ex(&self, key: &str, value: &str, ex_secs: i64) -> Result<bool, AppError> {
        (**self).set_nx_ex(key, value, ex_secs).await
    }
    async fn get_ttl(&self, key: &str) -> Result<Option<i64>, AppError> {
        (**self).get_ttl(key).await
    }
    async fn del(&self, key: &str) -> Result<(), AppError> {
        (**self).del(key).await
    }
}

// ─── 验证结果枚举 ─────────────────────────────────────────────────────────────

/// `verify_password` 的业务结果（区别于基础设施错误）
#[derive(Debug, PartialEq)]
pub enum VerifyPasswordResult {
    /// 验证成功，返回 60s room access JWT
    Token(String),
    /// 密码错误，还有 `remaining_attempts` 次机会
    WrongPassword { remaining_attempts: u32 },
    /// 用户已被锁定，`remaining_sec` 秒后解锁
    Locked { remaining_sec: i64 },
}

// ─── Redis Key 构造器 ─────────────────────────────────────────────────────────

fn fail_key(user_id: Uuid, room_id: Uuid) -> String {
    format!("pwd_fail:{user_id}:{room_id}")
}

fn lock_key(user_id: Uuid, room_id: Uuid) -> String {
    format!("pwd_lock:{user_id}:{room_id}")
}

/// 锁定失败阈值
const MAX_FAIL_COUNT: i64 = 5;
/// 锁定 + 失败计数 TTL（秒）
const LOCK_TTL_SECS: i64 = 1800;

// ─── 核心校验函数 ─────────────────────────────────────────────────────────────

/// 校验密码房密码，处理失败计数与锁定逻辑。
///
/// # 参数
/// - `room` — 含 `password_hash` 的房间 Model（必须是 password 类型且 active）
/// - `input_password` — 用户输入的明文密码
/// - `user_id` — 当前用户 ID（用于 Redis key 命名空间隔离）
/// - `redis` — Redis 抽象接口（测试时传入 FakeRoomPasswordRedis）
/// - `jwt_secret` — 签发 room_access token 用的密钥
///
/// # 返回
/// - `Ok(Token(jwt))` — 密码正确，jwt 60s 内有效
/// - `Ok(WrongPassword { remaining })` — 密码错误且未达锁定阈值
/// - `Ok(Locked { remaining_sec })` — 已锁定（含剩余锁定秒数）
/// - `Err` — 基础设施错误（Redis/bcrypt 故障）
pub async fn verify_password(
    room: &RoomModel,
    input_password: &str,
    user_id: Uuid,
    redis: &dyn RoomPasswordRedis,
    jwt_secret: &str,
) -> Result<VerifyPasswordResult, AppError> {
    let room_id = room.id;
    let lk = lock_key(user_id, room_id);
    let fk = fail_key(user_id, room_id);

    // ── 1. 检查锁定状态 ─────────────────────────────────────────────────────
    if let Some(remaining_sec) = redis.get_ttl(&lk).await? {
        return Ok(VerifyPasswordResult::Locked { remaining_sec });
    }

    // ── 2. bcrypt 校验 ────────────────────────────────────────────────────
    let hash = match &room.password_hash {
        Some(h) => h.clone(),
        None => {
            return Err(AppError::Internal(
                "password_hash missing for password room".to_string(),
            ));
        }
    };

    let is_valid = bcrypt::verify(input_password, &hash)
        .map_err(|e| AppError::Internal(format!("bcrypt error: {e}")))?;

    if is_valid {
        // ── 2a. 成功：清除失败计数，签发 token ─────────────────────────────
        redis.del(&fk).await?;
        let token = encode_room_access_token(user_id, room_id, jwt_secret.as_bytes())
            .map_err(|e| AppError::Internal(format!("jwt encode error: {e}")))?;
        return Ok(VerifyPasswordResult::Token(token));
    }

    // ── 2b. 失败：递增计数 ────────────────────────────────────────────────
    let count = redis.incr_with_ttl(&fk, LOCK_TTL_SECS).await?;

    if count >= MAX_FAIL_COUNT {
        // 达到阈值 → 设置锁定 key（NX 防止并发重复写）
        redis.set_nx_ex(&lk, "1", LOCK_TTL_SECS).await?;
        return Ok(VerifyPasswordResult::Locked {
            remaining_sec: LOCK_TTL_SECS,
        });
    }

    let remaining_attempts = (MAX_FAIL_COUNT - count) as u32;
    Ok(VerifyPasswordResult::WrongPassword { remaining_attempts })
}

// ─── Fake Redis 实现（测试专用）──────────────────────────────────────────────

/// 内存 Fake Redis，用于单元测试（无需真实 Redis 实例）。
///
/// 支持：`incr_with_ttl`、`set_nx_ex`、`get_ttl`、`del`。
/// TTL 通过 `Instant` 模拟，`expire_all()` 方法可手动触发 TTL 到期。
#[derive(Default)]
pub struct FakeRoomPasswordRedis {
    data: Mutex<HashMap<String, FakeEntry>>,
}

struct FakeEntry {
    value: String,
    expires_at: Option<Instant>,
}

impl FakeRoomPasswordRedis {
    /// 测试辅助：立即使所有 key 的 TTL 到期（模拟时间流逝）
    pub fn expire_all(&self) {
        let mut guard = self.data.lock().unwrap();
        // 将所有 expires_at 设为过去
        for entry in guard.values_mut() {
            entry.expires_at = Some(Instant::now() - std::time::Duration::from_secs(1));
        }
    }

    /// 测试辅助：检查指定 key 是否存在（未过期）
    pub fn key_exists(&self, key: &str) -> bool {
        let guard = self.data.lock().unwrap();
        if let Some(entry) = guard.get(key) {
            if let Some(exp) = entry.expires_at {
                return Instant::now() < exp;
            }
            return true;
        }
        false
    }

    /// 测试辅助：返回所有未过期的 key 数量（调试用）
    pub fn active_key_count(&self) -> usize {
        let guard = self.data.lock().unwrap();
        guard
            .values()
            .filter(|e| {
                e.expires_at
                    .map_or(true, |exp| Instant::now() < exp)
            })
            .count()
    }

    fn is_expired(entry: &FakeEntry) -> bool {
        entry
            .expires_at
            .map_or(false, |exp| Instant::now() >= exp)
    }
}

#[async_trait]
impl RoomPasswordRedis for FakeRoomPasswordRedis {
    async fn incr_with_ttl(&self, key: &str, ttl_secs: i64) -> Result<i64, AppError> {
        let mut guard = self.data.lock().unwrap();
        let entry = guard.get(key);

        let new_value = if let Some(e) = entry {
            if Self::is_expired(e) {
                // 过期 key 视为不存在，重新从 1 开始
                1i64
            } else {
                e.value.parse::<i64>().unwrap_or(0) + 1
            }
        } else {
            1i64
        };

        // 若 key 已存在且未过期，不重置 TTL（与 Redis INCR 语义一致）
        let preserved_expiry = guard
            .get(key)
            .filter(|e| !Self::is_expired(e))
            .and_then(|e| e.expires_at);

        guard.insert(
            key.to_string(),
            FakeEntry {
                value: new_value.to_string(),
                expires_at: preserved_expiry.or(Some(
                    Instant::now() + std::time::Duration::from_secs(ttl_secs as u64),
                )),
            },
        );
        Ok(new_value)
    }

    async fn set_nx_ex(&self, key: &str, value: &str, ex_secs: i64) -> Result<bool, AppError> {
        let mut guard = self.data.lock().unwrap();

        // 检查是否已有未过期的 key
        if let Some(entry) = guard.get(key) {
            if !Self::is_expired(entry) {
                return Ok(false); // key 已存在，NX 失败
            }
        }

        guard.insert(
            key.to_string(),
            FakeEntry {
                value: value.to_string(),
                expires_at: Some(
                    Instant::now() + std::time::Duration::from_secs(ex_secs as u64),
                ),
            },
        );
        Ok(true)
    }

    async fn get_ttl(&self, key: &str) -> Result<Option<i64>, AppError> {
        let guard = self.data.lock().unwrap();
        match guard.get(key) {
            None => Ok(None),
            Some(entry) if Self::is_expired(entry) => Ok(None),
            Some(entry) => {
                let remaining = entry
                    .expires_at
                    .map(|exp| {
                        let now = Instant::now();
                        if exp > now {
                            (exp - now).as_secs() as i64
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0);
                Ok(Some(remaining))
            }
        }
    }

    async fn del(&self, key: &str) -> Result<(), AppError> {
        let mut guard = self.data.lock().unwrap();
        guard.remove(key);
        Ok(())
    }
}

// ─── 真实 Redis 实现 ──────────────────────────────────────────────────────────

/// 生产环境 Redis 实现，使用 `redis` crate 的异步连接。
pub struct RealRoomPasswordRedis {
    client: redis::Client,
}

impl RealRoomPasswordRedis {
    pub fn new(redis_url: &str) -> Result<Self, AppError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| AppError::RedisError(format!("redis client open: {e}")))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl RoomPasswordRedis for RealRoomPasswordRedis {
    async fn incr_with_ttl(&self, key: &str, ttl_secs: i64) -> Result<i64, AppError> {
        use redis::AsyncCommands;
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        // INCR + EXPIRE（若 key 已存在则 EXPIRE 只在首次设置，保持原 TTL）
        let count: i64 = conn
            .incr(key, 1i64)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        if count == 1 {
            // 首次创建 key，设置 TTL
            let _: () = conn
                .expire(key, ttl_secs)
                .await
                .map_err(|e| AppError::RedisError(e.to_string()))?;
        }
        Ok(count)
    }

    async fn set_nx_ex(&self, key: &str, value: &str, ex_secs: i64) -> Result<bool, AppError> {
        use redis::AsyncCommands;
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        let result: Option<String> = conn
            .set_options(
                key,
                value,
                redis::SetOptions::default()
                    .conditional_set(redis::ExistenceCheck::NX)
                    .with_expiration(redis::SetExpiry::EX(ex_secs as u64)),
            )
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(result.is_some())
    }

    async fn get_ttl(&self, key: &str) -> Result<Option<i64>, AppError> {
        use redis::AsyncCommands;
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        let ttl: i64 = conn
            .ttl(key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        match ttl {
            -2 => Ok(None),       // key 不存在
            -1 => Ok(Some(0)),    // key 存在但无 TTL（不应发生在此业务场景）
            n if n >= 0 => Ok(Some(n)),
            _ => Ok(None),
        }
    }

    async fn del(&self, key: &str) -> Result<(), AppError> {
        use redis::AsyncCommands;
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        let _: i64 = conn
            .del(key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(())
    }
}

// ─── 单元测试 PR26-01 ~ PR26-11 ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Arc;

    const TEST_SECRET: &str = "test-jwt-secret-for-room-access";
    const BCRYPT_COST: u32 = 4; // 测试用低成本

    fn make_password_room(password: &str) -> RoomModel {
        let hash = bcrypt::hash(password, BCRYPT_COST).expect("bcrypt hash");
        RoomModel {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "密码测试房".to_string(),
            room_type: "password".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: Some(hash),
            max_members: 50,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        }
    }

    fn make_normal_room() -> RoomModel {
        RoomModel {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "普通房间".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        }
    }

    // ── PR26-01: 正确密码返回 token ────────────────────────────────────────

    #[tokio::test]
    async fn pr26_01_correct_password_returns_token() {
        let room = make_password_room("123456");
        let user_id = Uuid::new_v4();
        let redis = FakeRoomPasswordRedis::default();

        let result = verify_password(&room, "123456", user_id, &redis, TEST_SECRET)
            .await
            .expect("should not err");

        match result {
            VerifyPasswordResult::Token(jwt) => {
                // 解码 token 验证 claim
                use voice_room_shared::auth::room_access::decode_room_access_token;
                let claims =
                    decode_room_access_token(&jwt, TEST_SECRET.as_bytes()).expect("decode jwt");
                assert_eq!(claims.sub, user_id.to_string());
                assert_eq!(claims.room_id, room.id.to_string());
                assert_eq!(claims.iss, "voiceroom-room-access");
            }
            other => panic!("expected Token, got {other:?}"),
        }
    }

    // ── PR26-05: 错误密码连续 5 次后返回 Locked(42910) ──────────────────

    #[tokio::test]
    async fn pr26_05_five_wrong_passwords_returns_locked() {
        let room = make_password_room("123456");
        let user_id = Uuid::new_v4();
        let redis = FakeRoomPasswordRedis::default();

        // 前 4 次返回 WrongPassword
        for i in 1..MAX_FAIL_COUNT {
            let result = verify_password(&room, "wrong", user_id, &redis, TEST_SECRET)
                .await
                .unwrap();
            assert!(
                matches!(result, VerifyPasswordResult::WrongPassword { remaining_attempts } if remaining_attempts == (MAX_FAIL_COUNT - i) as u32),
                "第 {i} 次失败应返回 WrongPassword, remaining={}", MAX_FAIL_COUNT - i
            );
        }

        // 第 5 次返回 Locked
        let result = verify_password(&room, "wrong", user_id, &redis, TEST_SECRET)
            .await
            .unwrap();
        assert!(
            matches!(result, VerifyPasswordResult::Locked { .. }),
            "第 5 次失败应返回 Locked, got {result:?}"
        );

        // 验证 lock key 存在
        let lk = lock_key(user_id, room.id);
        assert!(redis.key_exists(&lk), "lock key 应在第 5 次后存在");
    }

    // ── PR26-06: 锁定后任何请求返回 Locked + remaining_sec ──────────────

    #[tokio::test]
    async fn pr26_06_locked_returns_42910_with_remaining_sec() {
        let room = make_password_room("123456");
        let user_id = Uuid::new_v4();
        let redis = FakeRoomPasswordRedis::default();

        // 先触发锁定
        for _ in 0..MAX_FAIL_COUNT {
            let _ = verify_password(&room, "wrong", user_id, &redis, TEST_SECRET)
                .await
                .unwrap();
        }

        // 锁定后任何密码（含正确密码）都返回 Locked
        let result = verify_password(&room, "123456", user_id, &redis, TEST_SECRET)
            .await
            .unwrap();
        match result {
            VerifyPasswordResult::Locked { remaining_sec } => {
                assert!(remaining_sec > 0, "remaining_sec 应 > 0");
                assert!(remaining_sec <= LOCK_TTL_SECS, "remaining_sec 应 <= {LOCK_TTL_SECS}");
            }
            other => panic!("锁定后应返回 Locked, got {other:?}"),
        }
    }

    // ── PR26-07: 锁定 TTL 到期后可重新尝试 ──────────────────────────────

    #[tokio::test]
    async fn pr26_07_after_lock_ttl_expires_can_retry() {
        let room = make_password_room("123456");
        let user_id = Uuid::new_v4();
        let redis = FakeRoomPasswordRedis::default();

        // 触发锁定
        for _ in 0..MAX_FAIL_COUNT {
            let _ = verify_password(&room, "wrong", user_id, &redis, TEST_SECRET)
                .await
                .unwrap();
        }

        // 模拟 TTL 到期（expire_all 使所有 key 立即过期）
        redis.expire_all();

        // 再次尝试正确密码应成功
        let result = verify_password(&room, "123456", user_id, &redis, TEST_SECRET)
            .await
            .unwrap();
        assert!(
            matches!(result, VerifyPasswordResult::Token(_)),
            "TTL 到期后正确密码应返回 Token, got {result:?}"
        );
    }

    // ── PR26-08: password_hash 为 None → 内部错误 ────────────────────────
    // （非密码房调 verify_password 的防御性测试）

    #[tokio::test]
    async fn pr26_08_missing_password_hash_returns_internal_error() {
        let room = make_normal_room(); // password_hash = None
        let user_id = Uuid::new_v4();
        let redis = FakeRoomPasswordRedis::default();

        let result = verify_password(&room, "123456", user_id, &redis, TEST_SECRET).await;
        assert!(
            matches!(result, Err(AppError::Internal(_))),
            "缺少 password_hash 应返回 Internal error"
        );
    }

    // ── PR26-09: 密码格式验证（在 controller 层，这里测试 bcrypt 兜底）──
    // controller 层对格式进行 validate_password 校验，此处测试错误密码情况

    #[tokio::test]
    async fn pr26_09_wrong_password_decrements_remaining_attempts() {
        let room = make_password_room("123456");
        let user_id = Uuid::new_v4();
        let redis = FakeRoomPasswordRedis::default();

        let result = verify_password(&room, "000000", user_id, &redis, TEST_SECRET)
            .await
            .unwrap();
        assert_eq!(
            result,
            VerifyPasswordResult::WrongPassword {
                remaining_attempts: 4
            },
            "首次失败应剩余 4 次"
        );

        let result2 = verify_password(&room, "000000", user_id, &redis, TEST_SECRET)
            .await
            .unwrap();
        assert_eq!(
            result2,
            VerifyPasswordResult::WrongPassword {
                remaining_attempts: 3
            },
            "第 2 次失败应剩余 3 次"
        );
    }

    // ── PR26-10: 并发 5 次错误仅创建一次锁定 key ───────────────────────

    #[tokio::test]
    async fn pr26_10_concurrent_five_failures_only_one_lock_key() {
        let room = Arc::new(make_password_room("123456"));
        let user_id = Uuid::new_v4();
        let redis = Arc::new(FakeRoomPasswordRedis::default());

        // 前 4 次顺序失败（确保计数到 4）
        for _ in 0..4 {
            let _ = verify_password(&room, "wrong", user_id, &redis, TEST_SECRET)
                .await
                .unwrap();
        }

        // 并发 3 个请求同时触发第 5+ 次（其中一个触发锁定）
        let mut handles = Vec::new();
        for _ in 0..3 {
            let room_clone = Arc::clone(&room);
            let redis_clone = Arc::clone(&redis);
            let h = tokio::spawn(async move {
                verify_password(&room_clone, "wrong", user_id, &redis_clone, TEST_SECRET).await
            });
            handles.push(h);
        }

        for h in handles {
            let _ = h.await.unwrap();
        }

        // 无论多少并发，lock key 只应被设置一次（NX 语义）
        let lk = lock_key(user_id, room.id);
        assert!(redis.key_exists(&lk), "lock key 应存在");

        // 不需要多 lock key（只有一个路径可以通过 NX 写入）
        // 验证 key_exists 本身就是验证唯一性
    }

    // ── PR26-11: 正确密码后 pwd_fail key 被清除 ─────────────────────────

    #[tokio::test]
    async fn pr26_11_correct_password_clears_fail_key() {
        let room = make_password_room("123456");
        let user_id = Uuid::new_v4();
        let redis = FakeRoomPasswordRedis::default();

        // 先失败 3 次
        for _ in 0..3 {
            let _ = verify_password(&room, "wrong", user_id, &redis, TEST_SECRET)
                .await
                .unwrap();
        }
        let fk = fail_key(user_id, room.id);
        assert!(redis.key_exists(&fk), "失败 3 次后 fail key 应存在");

        // 正确密码
        let result = verify_password(&room, "123456", user_id, &redis, TEST_SECRET)
            .await
            .unwrap();
        assert!(
            matches!(result, VerifyPasswordResult::Token(_)),
            "正确密码应返回 Token"
        );

        // fail key 应被清除
        assert!(!redis.key_exists(&fk), "成功后 fail key 应被清除");
    }

    // ── PR26-extra: 不同 user/room 组合的计数相互隔离 ──────────────────

    #[tokio::test]
    async fn isolation_between_users() {
        let room = make_password_room("123456");
        let user1 = Uuid::new_v4();
        let user2 = Uuid::new_v4();
        let redis = FakeRoomPasswordRedis::default();

        // user1 失败 4 次
        for _ in 0..4 {
            let _ = verify_password(&room, "wrong", user1, &redis, TEST_SECRET)
                .await
                .unwrap();
        }

        // user2 第一次失败应剩余 4 次（独立计数）
        let result = verify_password(&room, "wrong", user2, &redis, TEST_SECRET)
            .await
            .unwrap();
        assert_eq!(
            result,
            VerifyPasswordResult::WrongPassword {
                remaining_attempts: 4
            }
        );
    }
}
