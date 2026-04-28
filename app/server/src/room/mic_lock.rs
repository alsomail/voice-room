//! 抢麦分布式锁（T-00014 验收 #4 / P2-12）
//!
//! 通过 `MicLock::try_acquire` 在抢麦前申请短期排他锁（默认 TTL=3s），
//! 防止水平扩展场景下两个 Pod 同时把同一麦位分配给不同用户。
//!
//! - 单 Pod 部署时与 `RoomState` 内 `RwLock<Vec>` 形成"双重保护"，
//!   分布式锁仍然可以确保跨进程并发抢麦只有一个胜出。
//! - 锁是 best-effort：如果 Redis 暂时不可用（错误），`try_acquire`
//!   返回 `Err`，调用方应回退到原有 RoomState 锁（在 `handle_take_mic`
//!   中按 fail-open 策略：Err 时 warn 后继续走 RoomState 原子操作）。
//!
//! ## Redis Key 设计
//! - key = `mic_lock:{room_id}:{slot_index}`
//! - value = 任意（这里写当前调用者 user_id 字符串，便于排查）
//! - 命令：`SET key value NX EX ttl_secs`
//!
//! ## 测试 vs 生产实现
//! - `FakeMicLock`：内存 HashMap + Instant 过期，方便单元/集成测试
//! - `RealMicLock`：真实 redis::Client（生产 wiring）

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use uuid::Uuid;

use crate::common::error::AppError;

/// 抢麦锁 TTL（秒）— 与协议 §6 抢麦超时一致，3s 足以覆盖 take_mic_slot + 广播路径。
pub const MIC_LOCK_TTL_SECS: u64 = 3;

// ─── MicLock Trait ───────────────────────────────────────────────────────────

/// 抢麦分布式锁抽象。
#[async_trait]
pub trait MicLock: Send + Sync {
    /// 尝试为 `(room_id, slot_index)` 申请独占锁。
    ///
    /// - 返回 `Ok(true)`：当前调用者获锁成功，可继续抢麦
    /// - 返回 `Ok(false)`：锁被他人持有，调用方应返回 SLOT_OCCUPIED
    /// - 返回 `Err(_)`：Redis 暂时不可用，调用方按 fail-open 处理
    async fn try_acquire(
        &self,
        room_id: Uuid,
        slot_index: usize,
        owner: Uuid,
        ttl_secs: u64,
    ) -> Result<bool, AppError>;

    /// 主动释放 `(room_id, slot_index)` 的锁（LeaveMic 时调用）。
    ///
    /// 释放是 best-effort：若锁已过期或不存在，忽略错误。
    async fn release(&self, room_id: Uuid, slot_index: usize) -> Result<(), AppError>;
}

/// Blanket impl：允许 `Arc<T: MicLock>` 透明用作 `&dyn MicLock`。
#[async_trait]
impl<T: MicLock + ?Sized> MicLock for Arc<T> {
    async fn try_acquire(
        &self,
        room_id: Uuid,
        slot_index: usize,
        owner: Uuid,
        ttl_secs: u64,
    ) -> Result<bool, AppError> {
        (**self)
            .try_acquire(room_id, slot_index, owner, ttl_secs)
            .await
    }

    async fn release(&self, room_id: Uuid, slot_index: usize) -> Result<(), AppError> {
        (**self).release(room_id, slot_index).await
    }
}

fn lock_key(room_id: Uuid, slot_index: usize) -> String {
    format!("mic_lock:{room_id}:{slot_index}")
}

// ─── FakeMicLock（测试 / 默认）────────────────────────────────────────────────

struct FakeEntry {
    #[allow(dead_code)]
    owner: Uuid,
    expires_at: Instant,
}

/// 内存抢麦锁（测试 + AppState 默认）— 进程内 SET NX EX 等价语义。
pub struct FakeMicLock {
    data: Mutex<HashMap<String, FakeEntry>>,
}

impl Default for FakeMicLock {
    fn default() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl FakeMicLock {
    /// 测试辅助：将所有持有的锁立即过期。
    pub fn expire_all(&self) {
        let mut guard = self.data.lock().unwrap();
        let past = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now);
        for entry in guard.values_mut() {
            entry.expires_at = past;
        }
    }

    /// 测试辅助：判断指定 (room, slot) 锁是否仍持有（未过期）。
    pub fn is_locked(&self, room_id: Uuid, slot_index: usize) -> bool {
        let guard = self.data.lock().unwrap();
        guard
            .get(&lock_key(room_id, slot_index))
            .map(|e| Instant::now() < e.expires_at)
            .unwrap_or(false)
    }
}

#[async_trait]
impl MicLock for FakeMicLock {
    async fn try_acquire(
        &self,
        room_id: Uuid,
        slot_index: usize,
        owner: Uuid,
        ttl_secs: u64,
    ) -> Result<bool, AppError> {
        let key = lock_key(room_id, slot_index);
        let mut guard = self.data.lock().unwrap();
        let now = Instant::now();
        if let Some(entry) = guard.get(&key) {
            if now < entry.expires_at {
                return Ok(false);
            }
        }
        guard.insert(
            key,
            FakeEntry {
                owner,
                expires_at: now + Duration::from_secs(ttl_secs.max(1)),
            },
        );
        Ok(true)
    }

    async fn release(&self, room_id: Uuid, slot_index: usize) -> Result<(), AppError> {
        let key = lock_key(room_id, slot_index);
        self.data.lock().unwrap().remove(&key);
        Ok(())
    }
}

// ─── RealMicLock（生产 Redis 实现）────────────────────────────────────────────

/// 真实 Redis 抢麦锁 — 通过 `SET NX EX` 实现跨进程原子性。
pub struct RealMicLock {
    client: redis::Client,
}

impl RealMicLock {
    pub fn new(redis_url: &str) -> Result<Self, AppError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| AppError::RedisError(format!("redis client open: {e}")))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl MicLock for RealMicLock {
    async fn try_acquire(
        &self,
        room_id: Uuid,
        slot_index: usize,
        owner: Uuid,
        ttl_secs: u64,
    ) -> Result<bool, AppError> {
        let key = lock_key(room_id, slot_index);
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        // SET key owner NX EX ttl —— 直接走 SET options，避免 redis crate 版本差异
        let res: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(owner.to_string())
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs.max(1))
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        // SET NX：成功返回 "OK"，失败（已存在）返回 nil
        Ok(res.is_some())
    }

    async fn release(&self, room_id: Uuid, slot_index: usize) -> Result<(), AppError> {
        let key = lock_key(room_id, slot_index);
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(())
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// ML-01：首次 try_acquire 返回 Ok(true)
    #[tokio::test]
    async fn ml01_first_acquire_returns_ok_true() {
        let lock = FakeMicLock::default();
        let room = Uuid::new_v4();
        let user = Uuid::new_v4();
        let ok = lock
            .try_acquire(room, 0, user, MIC_LOCK_TTL_SECS)
            .await
            .unwrap();
        assert!(ok, "ML-01: first acquire should succeed");
        assert!(lock.is_locked(room, 0));
    }

    /// ML-02：同一 (room, slot) 第二次 try_acquire 在 TTL 内返回 Ok(false)
    #[tokio::test]
    async fn ml02_second_acquire_within_ttl_returns_false() {
        let lock = FakeMicLock::default();
        let room = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        let first = lock
            .try_acquire(room, 0, user_a, MIC_LOCK_TTL_SECS)
            .await
            .unwrap();
        let second = lock
            .try_acquire(room, 0, user_b, MIC_LOCK_TTL_SECS)
            .await
            .unwrap();
        assert!(first, "ML-02: first acquire should succeed");
        assert!(!second, "ML-02: second acquire should fail (locked)");
    }

    /// ML-03：不同 slot 互不干扰
    #[tokio::test]
    async fn ml03_different_slots_are_independent() {
        let lock = FakeMicLock::default();
        let room = Uuid::new_v4();
        let user = Uuid::new_v4();
        assert!(lock
            .try_acquire(room, 0, user, MIC_LOCK_TTL_SECS)
            .await
            .unwrap());
        assert!(lock
            .try_acquire(room, 1, user, MIC_LOCK_TTL_SECS)
            .await
            .unwrap());
    }

    /// ML-04：不同 room 互不干扰
    #[tokio::test]
    async fn ml04_different_rooms_are_independent() {
        let lock = FakeMicLock::default();
        let user = Uuid::new_v4();
        let room_a = Uuid::new_v4();
        let room_b = Uuid::new_v4();
        assert!(lock
            .try_acquire(room_a, 0, user, MIC_LOCK_TTL_SECS)
            .await
            .unwrap());
        assert!(lock
            .try_acquire(room_b, 0, user, MIC_LOCK_TTL_SECS)
            .await
            .unwrap());
    }

    /// ML-05：TTL 过期后允许重新获取
    #[tokio::test]
    async fn ml05_expired_lock_can_be_reacquired() {
        let lock = FakeMicLock::default();
        let room = Uuid::new_v4();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        assert!(lock
            .try_acquire(room, 0, user_a, MIC_LOCK_TTL_SECS)
            .await
            .unwrap());
        lock.expire_all();
        assert!(lock
            .try_acquire(room, 0, user_b, MIC_LOCK_TTL_SECS)
            .await
            .unwrap());
    }

    /// ML-06：高并发场景下恰好一个调用者获锁
    #[tokio::test]
    async fn ml06_concurrent_acquire_only_one_succeeds() {
        let lock = Arc::new(FakeMicLock::default());
        let room = Uuid::new_v4();

        let mut handles = Vec::new();
        for _ in 0..32 {
            let lock = lock.clone();
            handles.push(tokio::spawn(async move {
                lock.try_acquire(room, 0, Uuid::new_v4(), MIC_LOCK_TTL_SECS)
                    .await
                    .unwrap()
            }));
        }
        let mut wins = 0usize;
        for h in handles {
            if h.await.unwrap() {
                wins += 1;
            }
        }
        assert_eq!(wins, 1, "ML-06: exactly one concurrent acquire should win");
    }
}
