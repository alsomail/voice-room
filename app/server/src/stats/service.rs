//! 在線統計服務模組
//!
//! - `StatsPort`：業務 trait（async_trait），定義統計操作接口
//! - `StatsService`：真實 Redis 實現（HyperLogLog + Set）
//! - `FakeStatsService`：內存 Fake 實現，供單元測試注入
//!
//! Redis key 設計（protocol.md §6.2）：
//! - `stats:online_users`         — HyperLogLog，記錄在線 user_id
//! - `stats:active_rooms`         — Set，記錄活躍 room_id
//! - `stats:snapshot:{date}:{mm}` — Hash，每分鐘快照；TTL 7 天

use async_trait::async_trait;
use redis::{aio::MultiplexedConnection, AsyncCommands, Client as RedisClient};

use crate::common::error::AppError;

// ─── StatsPort trait ──────────────────────────────────────────────────────────

/// 在線統計抽象接口
///
/// 所有方法均返回 `Result<_, AppError>`，方便統一錯誤處理。
#[async_trait]
pub trait StatsPort: Send + Sync {
    /// 用戶上線：`PFADD stats:online_users {user_id}`
    async fn user_online(&self, user_id: uuid::Uuid) -> Result<(), AppError>;

    /// 用戶下線：HyperLogLog append-only，no-op + debug log
    async fn user_offline(&self, user_id: uuid::Uuid) -> Result<(), AppError>;

    /// 用戶進入房間：`SADD stats:active_rooms {room_id}`
    async fn user_join_room(&self, room_id: uuid::Uuid) -> Result<(), AppError>;

    /// 用戶離開房間：僅當房間人數變為 0 時才 `SREM stats:active_rooms {room_id}`
    ///
    /// `remaining_members`：用戶離開後房間剩餘人數。
    /// - `0`：執行 SREM，房間從活躍集合中移除
    /// - `>0`：no-op（其他用戶仍在房間，房間繼續活躍）
    async fn user_leave_room(
        &self,
        room_id: uuid::Uuid,
        remaining_members: usize,
    ) -> Result<(), AppError>;

    /// 取得在線用戶估計數：`PFCOUNT stats:online_users`
    async fn get_online_count(&self) -> Result<u64, AppError>;

    /// 取得活躍房間數：`SCARD stats:active_rooms`
    async fn get_active_room_count(&self) -> Result<u64, AppError>;

    /// 執行快照：讀取兩項計數 → `HSET stats:snapshot:{date}:{mm}` + EXPIRE 604800
    async fn take_snapshot(&self) -> Result<(), AppError>;
}

// ─── StatsService（真實 Redis 實現）──────────────────────────────────────────

const ONLINE_USERS_KEY: &str = "stats:online_users";
const ACTIVE_ROOMS_KEY: &str = "stats:active_rooms";
const SNAPSHOT_TTL_SECS: i64 = 604_800; // 7 天

pub struct StatsService {
    conn: MultiplexedConnection,
}

impl StatsService {
    pub async fn new(redis_url: &str) -> Result<Self, AppError> {
        let client = RedisClient::open(redis_url)
            .map_err(|e| AppError::Internal(format!("stats redis open: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::Internal(format!("stats redis conn: {e}")))?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl StatsPort for StatsService {
    async fn user_online(&self, user_id: uuid::Uuid) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        let _: i64 = conn
            .pfadd(ONLINE_USERS_KEY, user_id.to_string())
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        tracing::debug!(%user_id, "stats: user online");
        Ok(())
    }

    async fn user_offline(&self, user_id: uuid::Uuid) -> Result<(), AppError> {
        // HyperLogLog 是 append-only 結構，無法刪除個別元素，故 offline 為 no-op
        tracing::debug!(%user_id, "stats: user offline (no-op, HLL append-only)");
        Ok(())
    }

    async fn user_join_room(&self, room_id: uuid::Uuid) -> Result<(), AppError> {
        let mut conn = self.conn.clone();
        let _: i64 = conn
            .sadd(ACTIVE_ROOMS_KEY, room_id.to_string())
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        tracing::debug!(%room_id, "stats: room active");
        Ok(())
    }

    async fn user_leave_room(
        &self,
        room_id: uuid::Uuid,
        remaining_members: usize,
    ) -> Result<(), AppError> {
        if remaining_members > 0 {
            tracing::debug!(
                %room_id,
                remaining_members,
                "stats: user_leave_room no-op (room still has members)"
            );
            return Ok(());
        }
        let mut conn = self.conn.clone();
        let _: i64 = conn
            .srem(ACTIVE_ROOMS_KEY, room_id.to_string())
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        tracing::debug!(%room_id, "stats: room deactivated (last user left)");
        Ok(())
    }

    async fn get_online_count(&self) -> Result<u64, AppError> {
        let mut conn = self.conn.clone();
        let count: i64 = conn
            .pfcount(ONLINE_USERS_KEY)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(count as u64)
    }

    async fn get_active_room_count(&self) -> Result<u64, AppError> {
        let mut conn = self.conn.clone();
        let count: i64 = conn
            .scard(ACTIVE_ROOMS_KEY)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(count as u64)
    }

    async fn take_snapshot(&self) -> Result<(), AppError> {
        let mut conn = self.conn.clone();

        // 1. 讀取兩項計數
        let online: i64 = conn
            .pfcount(ONLINE_USERS_KEY)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        let rooms: i64 = conn
            .scard(ACTIVE_ROOMS_KEY)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        // 2. 構建快照 key：stats:snapshot:{date}:{HH:MM}
        let now = chrono::Utc::now();
        let key = format!(
            "stats:snapshot:{}:{}",
            now.format("%Y-%m-%d"),
            now.format("%H:%M")
        );

        // 3. 寫入 Hash + 設置 TTL（MULTI/EXEC 原子 pipeline，防止 HSET 成功但 EXPIRE 失敗）
        redis::pipe()
            .atomic()
            .hset(&key, "online_users", online)
            .hset(&key, "active_rooms", rooms)
            .expire(&key, SNAPSHOT_TTL_SECS)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;

        tracing::info!(
            online_users = online,
            active_rooms = rooms,
            snapshot_key = %key,
            "stats: snapshot written"
        );
        Ok(())
    }
}

// ─── FakeStatsService（內存實現，用於單元測試）───────────────────────────────

/// 基於 HashSet 的內存統計服務，用於測試注入
///
/// - `online_users`：模擬 HyperLogLog（insert = pfadd，count = len）
/// - `active_rooms`：模擬 Redis Set（insert/remove）
/// - `snapshot_calls`：累計快照調用次數
#[derive(Default)]
pub struct FakeStatsService {
    pub online_users: std::sync::Mutex<std::collections::HashSet<uuid::Uuid>>,
    pub active_rooms: std::sync::Mutex<std::collections::HashSet<uuid::Uuid>>,
    pub snapshot_calls: std::sync::atomic::AtomicU32,
}

#[async_trait]
impl StatsPort for FakeStatsService {
    async fn user_online(&self, user_id: uuid::Uuid) -> Result<(), AppError> {
        self.online_users.lock().unwrap().insert(user_id);
        Ok(())
    }

    async fn user_offline(&self, _user_id: uuid::Uuid) -> Result<(), AppError> {
        // HLL append-only：Fake 同樣 no-op，不從集合移除
        Ok(())
    }

    async fn user_join_room(&self, room_id: uuid::Uuid) -> Result<(), AppError> {
        self.active_rooms.lock().unwrap().insert(room_id);
        Ok(())
    }

    async fn user_leave_room(
        &self,
        room_id: uuid::Uuid,
        remaining_members: usize,
    ) -> Result<(), AppError> {
        if remaining_members == 0 {
            self.active_rooms.lock().unwrap().remove(&room_id);
        }
        Ok(())
    }

    async fn get_online_count(&self) -> Result<u64, AppError> {
        let count = self.online_users.lock().unwrap().len() as u64;
        Ok(count)
    }

    async fn get_active_room_count(&self) -> Result<u64, AppError> {
        let count = self.active_rooms.lock().unwrap().len() as u64;
        Ok(count)
    }

    async fn take_snapshot(&self) -> Result<(), AppError> {
        self.snapshot_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

// ─── 為 Arc<FakeStatsService> 實現 StatsPort（方便測試持有計數同時傳入 trait）
// 不需要——直接 Arc::new(FakeStatsService::default()) as Arc<dyn StatsPort>

// ─── 單元測試 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use uuid::Uuid;

    use super::*;

    fn fake() -> Arc<FakeStatsService> {
        Arc::new(FakeStatsService::default())
    }

    // ST01: user_online 後 get_online_count 返回 1
    #[tokio::test]
    async fn st01_user_online_increments_count() {
        let svc = fake();
        let uid = Uuid::new_v4();

        svc.user_online(uid).await.unwrap();

        let count = svc.get_online_count().await.unwrap();
        assert_eq!(
            count, 1,
            "online count should be 1 after one user comes online"
        );
    }

    // ST02: 同一用戶多次 online，count 仍為 1（HLL 去重）
    #[tokio::test]
    async fn st02_same_user_online_deduplicated() {
        let svc = fake();
        let uid = Uuid::new_v4();

        svc.user_online(uid).await.unwrap();
        svc.user_online(uid).await.unwrap();
        svc.user_online(uid).await.unwrap();

        let count = svc.get_online_count().await.unwrap();
        assert_eq!(
            count, 1,
            "HLL deduplication: same user_id should only be counted once"
        );
    }

    // ST03: user_offline 不減少 online_count（HLL append-only）
    #[tokio::test]
    async fn st03_user_offline_does_not_decrease_count() {
        let svc = fake();
        let uid = Uuid::new_v4();

        svc.user_online(uid).await.unwrap();
        svc.user_offline(uid).await.unwrap(); // HLL no-op

        let count = svc.get_online_count().await.unwrap();
        assert_eq!(
            count, 1,
            "HLL is append-only: offline must NOT decrease online count"
        );
    }

    // ST04: user_join_room 後 get_active_room_count 返回 1
    #[tokio::test]
    async fn st04_user_join_room_increments_room_count() {
        let svc = fake();
        let rid = Uuid::new_v4();

        svc.user_join_room(rid).await.unwrap();

        let count = svc.get_active_room_count().await.unwrap();
        assert_eq!(
            count, 1,
            "active room count should be 1 after joining one room"
        );
    }

    // ST05: user_leave_room（remaining=0）後 get_active_room_count 返回 0
    #[tokio::test]
    async fn st05_user_leave_room_decrements_room_count() {
        let svc = fake();
        let rid = Uuid::new_v4();

        svc.user_join_room(rid).await.unwrap();
        svc.user_leave_room(rid, 0).await.unwrap();

        let count = svc.get_active_room_count().await.unwrap();
        assert_eq!(
            count, 0,
            "active room count should be 0 after last user leaves the room"
        );
    }

    // ST05B (P1-4): user_leave_room 在房間仍有成員時 NOT 移除房間
    #[tokio::test]
    async fn st05b_user_leave_room_keeps_room_when_members_remain() {
        let svc = fake();
        let rid = Uuid::new_v4();

        svc.user_join_room(rid).await.unwrap();
        // 模擬：3 人房間，1 人離開後仍剩 2 人
        svc.user_leave_room(rid, 2).await.unwrap();

        let count = svc.get_active_room_count().await.unwrap();
        assert_eq!(
            count, 1,
            "room must remain active while other members are still inside"
        );
    }

    // ST06: 同一 room 被 join 兩次後 count 為 1（Set 冪等）
    #[tokio::test]
    async fn st06_join_same_room_twice_idempotent() {
        let svc = fake();
        let rid = Uuid::new_v4();

        svc.user_join_room(rid).await.unwrap();
        svc.user_join_room(rid).await.unwrap();

        let count = svc.get_active_room_count().await.unwrap();
        assert_eq!(
            count, 1,
            "Set is idempotent: joining same room twice should count as 1"
        );
    }

    // ST07: take_snapshot 不返回 Err
    #[tokio::test]
    async fn st07_take_snapshot_no_error() {
        let svc = fake();

        let result = svc.take_snapshot().await;

        assert!(
            result.is_ok(),
            "take_snapshot must not return an error on FakeStatsService"
        );
        assert_eq!(
            svc.snapshot_calls.load(Ordering::Relaxed),
            1,
            "snapshot_calls counter should be incremented"
        );
    }
}
