//! MuteUser/UnmuteUser 信令处理 — T-00029
//!
//! ## 处理流程（handle_mute）
//! 1. 解析 payload（room_id / target_user_id / type / duration_sec / reason）
//! 2. duration_sec = 0 → 转发 handle_unmute 逻辑（删 Redis key + 广播 duration_sec=0）
//! 3. duration_sec ∉ [60, 86400] → 40002
//! 4. 加载房间 Model → 权限校验（owner / admin），target ≠ owner
//! 5. 验证 target 在房间内 → 40400
//! 6. Redis SETEX {type}_muted:{room_id}:{user_id} duration_sec reason
//! 7. DB INSERT room_mute_records（审计）
//! 8. 若 type=mic 且 target 在麦 → 自动下麦 + 广播 MicLeft forced=true
//! 9. 广播 UserMuted 给房间所有人
//!
//! ## SendMessage / TakeMic 前置拦截
//! - SendMessage（T-00016）：`GET chat_muted:{room_id}:{user_id}` 存在 → 40305
//! - TakeMic（T-00014）：`GET mic_muted:{room_id}:{user_id}` 存在 → 40306

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use uuid::Uuid;

use crate::common::error::AppError;
use crate::modules::room::service::RoomService;
use crate::room::handler::broadcast_mic_left;
use crate::room::manager::RoomManager;
use crate::ws::registry::ConnectionRegistry;

// ─── 常量 ─────────────────────────────────────────────────────────────────────

/// 禁麦/禁言时长下限（秒）
pub const MUTE_DURATION_MIN: i64 = 60;
/// 禁麦/禁言时长上限（秒）
pub const MUTE_DURATION_MAX: i64 = 86400;

// ─── MuteRedis Trait ──────────────────────────────────────────────────────────

/// 禁麦/禁言 Redis 操作抽象。
///
/// 生产实现使用真实 Redis；测试使用 `FakeMuteRedis`（内存 HashMap）。
#[async_trait]
pub trait MuteRedis: Send + Sync {
    /// 设置禁麦/禁言 key：`{mute_type}_muted:{room_id}:{user_id}`，TTL = duration_sec。
    async fn set_mute(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
        duration_sec: i64,
        reason: &str,
    ) -> Result<(), AppError>;

    /// 删除禁麦/禁言 key（解除禁麦/禁言）。
    async fn del_mute(&self, mute_type: &str, room_id: Uuid, user_id: Uuid)
        -> Result<(), AppError>;

    /// 查询禁麦/禁言剩余秒数；key 不存在或已过期则返回 None。
    async fn get_mute_ttl(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i64>, AppError>;
}

/// Blanket impl：允许 `Arc<T: MuteRedis>` 直接用作 `&dyn MuteRedis`
#[async_trait]
impl<T: MuteRedis + ?Sized> MuteRedis for Arc<T> {
    async fn set_mute(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
        duration_sec: i64,
        reason: &str,
    ) -> Result<(), AppError> {
        (**self)
            .set_mute(mute_type, room_id, user_id, duration_sec, reason)
            .await
    }

    async fn del_mute(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        (**self).del_mute(mute_type, room_id, user_id).await
    }

    async fn get_mute_ttl(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i64>, AppError> {
        (**self).get_mute_ttl(mute_type, room_id, user_id).await
    }
}

// ─── MuteDb Trait ─────────────────────────────────────────────────────────────

/// 禁麦/禁言审计记录 DB 操作抽象。
#[async_trait]
pub trait MuteDb: Send + Sync {
    /// 插入一条禁麦/禁言审计记录。
    #[allow(clippy::too_many_arguments)]
    async fn insert_mute_record(
        &self,
        room_id: Uuid,
        operator_id: Uuid,
        target_id: Uuid,
        mute_type: &str,
        duration_sec: i64,
        reason: &str,
    ) -> Result<(), AppError>;
}

/// Blanket impl：允许 `Arc<T: MuteDb>` 直接用作 `&dyn MuteDb`
#[async_trait]
impl<T: MuteDb + ?Sized> MuteDb for Arc<T> {
    async fn insert_mute_record(
        &self,
        room_id: Uuid,
        operator_id: Uuid,
        target_id: Uuid,
        mute_type: &str,
        duration_sec: i64,
        reason: &str,
    ) -> Result<(), AppError> {
        (**self)
            .insert_mute_record(
                room_id,
                operator_id,
                target_id,
                mute_type,
                duration_sec,
                reason,
            )
            .await
    }
}

// ─── FakeMuteRedis（测试用内存实现）─────────────────────────────────────────

struct MuteRedisEntry {
    #[allow(dead_code)]
    reason: String,
    expires_at: Instant,
}

/// 内存禁麦/禁言 Redis（测试专用）。
pub struct FakeMuteRedis {
    data: Mutex<HashMap<String, MuteRedisEntry>>,
}

impl Default for FakeMuteRedis {
    fn default() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

impl FakeMuteRedis {
    fn mute_key(mute_type: &str, room_id: Uuid, user_id: Uuid) -> String {
        format!("{mute_type}_muted:{room_id}:{user_id}")
    }

    /// 测试辅助：立即使所有 key 过期（模拟 TTL 到期）
    pub fn expire_all(&self) {
        let mut guard = self.data.lock().unwrap();
        let past = Instant::now()
            .checked_sub(std::time::Duration::from_secs(1))
            .unwrap_or(Instant::now());
        for entry in guard.values_mut() {
            entry.expires_at = past;
        }
    }

    /// 测试辅助：检查指定 key 是否仍有效（未过期）
    pub fn key_exists(&self, mute_type: &str, room_id: Uuid, user_id: Uuid) -> bool {
        let guard = self.data.lock().unwrap();
        let key = Self::mute_key(mute_type, room_id, user_id);
        guard
            .get(&key)
            .map(|e| Instant::now() < e.expires_at)
            .unwrap_or(false)
    }
}

#[async_trait]
impl MuteRedis for FakeMuteRedis {
    async fn set_mute(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
        duration_sec: i64,
        reason: &str,
    ) -> Result<(), AppError> {
        let key = Self::mute_key(mute_type, room_id, user_id);
        let mut guard = self.data.lock().unwrap();
        guard.insert(
            key,
            MuteRedisEntry {
                reason: reason.to_string(),
                expires_at: Instant::now()
                    + std::time::Duration::from_secs(duration_sec.max(0) as u64),
            },
        );
        Ok(())
    }

    async fn del_mute(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let key = Self::mute_key(mute_type, room_id, user_id);
        let mut guard = self.data.lock().unwrap();
        guard.remove(&key);
        Ok(())
    }

    async fn get_mute_ttl(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i64>, AppError> {
        let key = Self::mute_key(mute_type, room_id, user_id);
        let guard = self.data.lock().unwrap();
        match guard.get(&key) {
            None => Ok(None),
            Some(entry) => {
                let now = Instant::now();
                if entry.expires_at <= now {
                    Ok(None) // 已过期
                } else {
                    Ok(Some((entry.expires_at - now).as_secs() as i64))
                }
            }
        }
    }
}

// ─── FakeMuteDb（测试用内存实现）─────────────────────────────────────────────

/// 禁麦/禁言审计记录（测试用快照）
#[derive(Debug, Clone)]
pub struct MuteRecord {
    pub room_id: Uuid,
    pub operator_id: Uuid,
    pub target_id: Uuid,
    pub mute_type: String,
    pub duration_sec: i64,
    pub reason: String,
}

/// 内存禁麦/禁言审计 DB（测试专用）。
pub struct FakeMuteDb {
    records: Mutex<Vec<MuteRecord>>,
}

impl Default for FakeMuteDb {
    fn default() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }
}

impl FakeMuteDb {
    /// 测试辅助：返回已插入的审计记录数
    pub fn record_count(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    /// 测试辅助：获取所有审计记录（克隆）
    pub fn all_records(&self) -> Vec<MuteRecord> {
        self.records.lock().unwrap().clone()
    }
}

#[async_trait]
impl MuteDb for FakeMuteDb {
    async fn insert_mute_record(
        &self,
        room_id: Uuid,
        operator_id: Uuid,
        target_id: Uuid,
        mute_type: &str,
        duration_sec: i64,
        reason: &str,
    ) -> Result<(), AppError> {
        self.records.lock().unwrap().push(MuteRecord {
            room_id,
            operator_id,
            target_id,
            mute_type: mute_type.to_string(),
            duration_sec,
            reason: reason.to_string(),
        });
        Ok(())
    }
}

// ─── RealMuteRedis（生产 Redis 实现）─────────────────────────────────────────

/// 生产禁麦/禁言 Redis 实现（基于真实 Redis Client）
pub struct RealMuteRedis {
    client: redis::Client,
}

impl RealMuteRedis {
    pub fn new(redis_url: &str) -> Result<Self, AppError> {
        let client =
            redis::Client::open(redis_url).map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl MuteRedis for RealMuteRedis {
    async fn set_mute(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
        duration_sec: i64,
        reason: &str,
    ) -> Result<(), AppError> {
        use redis::AsyncCommands;
        let key = format!("{mute_type}_muted:{room_id}:{user_id}");
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        let _: () = conn
            .set_ex(&key, reason, duration_sec.max(0) as u64)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(())
    }

    async fn del_mute(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        use redis::AsyncCommands;
        let key = format!("{mute_type}_muted:{room_id}:{user_id}");
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        let _: () = conn
            .del(&key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(())
    }

    async fn get_mute_ttl(
        &self,
        mute_type: &str,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i64>, AppError> {
        use redis::AsyncCommands;
        let key = format!("{mute_type}_muted:{room_id}:{user_id}");
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        let ttl: i64 = conn
            .ttl(&key)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        match ttl {
            -2 => Ok(None), // key 不存在
            n if n >= 0 => Ok(Some(n)),
            _ => Ok(None),
        }
    }
}

// ─── RealMuteDb（生产 DB 实现）────────────────────────────────────────────────

/// 生产禁麦/禁言审计 DB 实现（基于 sqlx PgPool）
pub struct RealMuteDb {
    pool: sqlx::PgPool,
}

impl RealMuteDb {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MuteDb for RealMuteDb {
    async fn insert_mute_record(
        &self,
        room_id: Uuid,
        operator_id: Uuid,
        target_id: Uuid,
        mute_type: &str,
        duration_sec: i64,
        reason: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO room_mute_records \
             (room_id, operator_user_id, target_user_id, mute_type, duration_sec, reason) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(room_id)
        .bind(operator_id)
        .bind(target_id)
        .bind(mute_type)
        .bind(duration_sec)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

// ─── MuteDeps ─────────────────────────────────────────────────────────────────

/// `handle_mute` / `handle_unmute` 所需的全部服务依赖。
pub struct MuteDeps {
    /// 房间运行时状态管理器（成员表、麦位）
    pub room_manager: Arc<RoomManager>,
    /// 房间服务（权限校验：owner_id + admin_user_id）
    pub room_service: Arc<RoomService>,
    /// 禁麦/禁言 Redis（SETEX / DEL / TTL）
    pub mute_redis: Arc<dyn MuteRedis>,
    /// 禁麦/禁言审计 DB
    pub mute_db: Arc<dyn MuteDb>,
    /// WS 连接注册表（广播）
    pub registry: Arc<ConnectionRegistry>,
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

fn mute_error(msg_id: Option<String>, code: i64, message: &str) -> String {
    serde_json::json!({
        "type": "MuteUserResult",
        "msg_id": msg_id,
        "code": code,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp(),
    })
    .to_string()
}

fn mute_success(msg_id: Option<String>) -> String {
    serde_json::json!({
        "type": "MuteUserResult",
        "msg_id": msg_id,
        "code": 0,
        "timestamp": chrono::Utc::now().timestamp(),
    })
    .to_string()
}

fn unmute_error(msg_id: Option<String>, code: i64, message: &str) -> String {
    serde_json::json!({
        "type": "UnmuteUserResult",
        "msg_id": msg_id,
        "code": code,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp(),
    })
    .to_string()
}

fn unmute_success(msg_id: Option<String>) -> String {
    serde_json::json!({
        "type": "UnmuteUserResult",
        "msg_id": msg_id,
        "code": 0,
        "timestamp": chrono::Utc::now().timestamp(),
    })
    .to_string()
}

/// 广播 UserMuted 给房间内所有连接（走统一出口 broadcast_to_room）
#[allow(clippy::too_many_arguments)]
fn broadcast_user_muted(
    registry: &ConnectionRegistry,
    room_state: &crate::room::state::RoomState,
    room_id: Uuid,
    target_user_id: Uuid,
    operator_id: Uuid,
    mute_type: &str,
    duration_sec: i64,
    expires_at: Option<String>,
) {
    let payload = if duration_sec == 0 {
        serde_json::json!({
            "room_id": room_id.to_string(),
            "target_user_id": target_user_id.to_string(),
            "type": mute_type,
            "duration_sec": 0,
            "operator_id": operator_id.to_string(),
        })
    } else {
        serde_json::json!({
            "room_id": room_id.to_string(),
            "target_user_id": target_user_id.to_string(),
            "type": mute_type,
            "duration_sec": duration_sec,
            "expires_at": expires_at,
            "operator_id": operator_id.to_string(),
        })
    };

    let envelope = serde_json::json!({
        "type": "UserMuted",
        "payload": payload,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    crate::ws::broadcaster::broadcast_to_room(registry, room_state, envelope);
}

// ─── handle_mute ──────────────────────────────────────────────────────────────

/// 处理 MuteUser 信令，返回 JSON 字符串响应。
///
/// duration_sec=0 时等同于 UnmuteUser，走解除禁麦/禁言路径。
pub async fn handle_mute(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    operator_user_id: Uuid,
    deps: &MuteDeps,
) -> String {
    let MuteDeps {
        room_manager,
        room_service,
        mute_redis,
        mute_db,
        registry,
    } = deps;

    // ── 1. 解析 room_id ────────────────────────────────────────────────────────
    let room_id = match payload
        .as_ref()
        .and_then(|p| p.get("room_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return mute_error(msg_id, 40002, "missing room_id"),
    };

    // ── 2. 解析 target_user_id ────────────────────────────────────────────────
    let target_user_id = match payload
        .as_ref()
        .and_then(|p| p.get("target_user_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return mute_error(msg_id, 40002, "missing target_user_id"),
    };

    // ── 3. 解析 mute_type（"mic" 或 "chat"）──────────────────────────────────
    let mute_type = match payload
        .as_ref()
        .and_then(|p| p.get("type"))
        .and_then(|v| v.as_str())
        .filter(|s| *s == "mic" || *s == "chat")
    {
        Some(t) => t.to_string(),
        None => {
            return mute_error(
                msg_id,
                40002,
                "missing or invalid type (must be 'mic' or 'chat')",
            )
        }
    };

    // ── 4. 解析 duration_sec ──────────────────────────────────────────────────
    let duration_sec = match payload
        .as_ref()
        .and_then(|p| p.get("duration_sec"))
        .and_then(|v| v.as_i64())
    {
        Some(d) => d,
        None => return mute_error(msg_id, 40002, "missing duration_sec"),
    };

    // ── 5. duration=0 → 走解除路径 ───────────────────────────────────────────
    if duration_sec == 0 {
        return do_unmute(
            room_id,
            target_user_id,
            &mute_type,
            msg_id,
            operator_user_id,
            room_service,
            mute_redis,
            registry,
            room_manager,
        )
        .await;
    }

    // ── 6. duration 范围校验：[60, 86400] ─────────────────────────────────────
    if duration_sec < MUTE_DURATION_MIN || duration_sec > MUTE_DURATION_MAX {
        return mute_error(
            msg_id,
            40002,
            &format!("duration_sec must be 0 or in [{MUTE_DURATION_MIN}, {MUTE_DURATION_MAX}]"),
        );
    }

    // ── 7. 解析 reason ────────────────────────────────────────────────────────
    let reason = payload
        .as_ref()
        .and_then(|p| p.get("reason"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // ── 8. 获取房间 Model（权限校验用）────────────────────────────────────────
    let room = match room_service.get_active_room_model(room_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return mute_error(msg_id, 40400, "room not found or closed"),
        Err(e) => {
            tracing::error!("get_active_room_model error: {e}");
            return mute_error(msg_id, 50000, "internal error");
        }
    };

    // ── 9. 权限校验：操作者必须是 owner 或 admin ──────────────────────────────
    let is_owner = room.owner_id == operator_user_id;
    let is_admin = room.admin_user_id == Some(operator_user_id);
    if !is_owner && !is_admin {
        return mute_error(msg_id, 40301, "permission denied");
    }

    // ── 10. 不能禁麦房主 ──────────────────────────────────────────────────────
    if target_user_id == room.owner_id {
        return mute_error(msg_id, 40302, "cannot mute owner");
    }

    // ── 11. 检查 target 是否在房间 ────────────────────────────────────────────
    if !room_manager.is_member(room_id, target_user_id) {
        return mute_error(msg_id, 40400, "target not in room");
    }

    // ── 12. Redis SETEX ───────────────────────────────────────────────────────
    if let Err(e) = mute_redis
        .set_mute(&mute_type, room_id, target_user_id, duration_sec, &reason)
        .await
    {
        tracing::error!("set_mute failed: {e}");
        return mute_error(msg_id, 50000, "redis error");
    }

    // ── 13. DB INSERT 审计记录 ────────────────────────────────────────────────
    if let Err(e) = mute_db
        .insert_mute_record(
            room_id,
            operator_user_id,
            target_user_id,
            &mute_type,
            duration_sec,
            &reason,
        )
        .await
    {
        tracing::error!("insert_mute_record failed: {e}");
        // 非致命，继续执行
    }

    // ── 14. 若 type=mic 且 target 在麦 → 自动下麦 ────────────────────────────
    let room_state_opt = room_manager.get_room(room_id);
    if mute_type == "mic" {
        let mic_slot_left = room_state_opt
            .as_ref()
            .and_then(|s| s.leave_mic_slot(target_user_id));
        if let (Some(mic_index), Some(rs)) = (mic_slot_left, room_state_opt.as_ref()) {
            broadcast_mic_left(registry, rs, mic_index, target_user_id, true);
        }
    }

    // ── 15. 广播 UserMuted 给房间所有人 ──────────────────────────────────────
    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(duration_sec))
        .map(|t| t.to_rfc3339());

    if let Some(rs) = room_state_opt.as_ref() {
        broadcast_user_muted(
            registry,
            rs,
            room_id,
            target_user_id,
            operator_user_id,
            &mute_type,
            duration_sec,
            expires_at,
        );
    }

    mute_success(msg_id)
}

// ─── handle_unmute ────────────────────────────────────────────────────────────

/// 处理 UnmuteUser 信令，返回 JSON 字符串响应。
pub async fn handle_unmute(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    operator_user_id: Uuid,
    deps: &MuteDeps,
) -> String {
    let MuteDeps {
        room_manager,
        room_service,
        mute_redis,
        mute_db: _,
        registry,
    } = deps;

    // ── 1. 解析 room_id ────────────────────────────────────────────────────────
    let room_id = match payload
        .as_ref()
        .and_then(|p| p.get("room_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return unmute_error(msg_id, 40002, "missing room_id"),
    };

    // ── 2. 解析 target_user_id ────────────────────────────────────────────────
    let target_user_id = match payload
        .as_ref()
        .and_then(|p| p.get("target_user_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return unmute_error(msg_id, 40002, "missing target_user_id"),
    };

    // ── 3. 解析 mute_type ─────────────────────────────────────────────────────
    let mute_type = match payload
        .as_ref()
        .and_then(|p| p.get("type"))
        .and_then(|v| v.as_str())
        .filter(|s| *s == "mic" || *s == "chat")
    {
        Some(t) => t.to_string(),
        None => {
            return unmute_error(
                msg_id,
                40002,
                "missing or invalid type (must be 'mic' or 'chat')",
            )
        }
    };

    // ── 4. 权限校验 ───────────────────────────────────────────────────────────
    let room = match room_service.get_active_room_model(room_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return unmute_error(msg_id, 40400, "room not found or closed"),
        Err(e) => {
            tracing::error!("get_active_room_model error: {e}");
            return unmute_error(msg_id, 50000, "internal error");
        }
    };

    let is_owner = room.owner_id == operator_user_id;
    let is_admin = room.admin_user_id == Some(operator_user_id);
    if !is_owner && !is_admin {
        return unmute_error(msg_id, 40301, "permission denied");
    }

    // ── 5. 调用内部解除逻辑 ───────────────────────────────────────────────────
    do_unmute_internal(
        room_id,
        target_user_id,
        &mute_type,
        msg_id,
        operator_user_id,
        mute_redis,
        registry,
        room_manager,
        false,
    )
    .await
}

/// 内部解除禁麦/禁言（供 handle_mute duration=0 和 handle_unmute 共用）。
/// `from_mute_cmd`：true 时返回 MuteUserResult，false 时返回 UnmuteUserResult
#[allow(clippy::too_many_arguments)]
async fn do_unmute(
    room_id: Uuid,
    target_user_id: Uuid,
    mute_type: &str,
    msg_id: Option<String>,
    operator_user_id: Uuid,
    room_service: &Arc<RoomService>,
    mute_redis: &Arc<dyn MuteRedis>,
    registry: &Arc<ConnectionRegistry>,
    room_manager: &Arc<RoomManager>,
) -> String {
    // 权限校验
    let room = match room_service.get_active_room_model(room_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return mute_error(msg_id, 40400, "room not found or closed"),
        Err(e) => {
            tracing::error!("get_active_room_model error: {e}");
            return mute_error(msg_id, 50000, "internal error");
        }
    };

    let is_owner = room.owner_id == operator_user_id;
    let is_admin = room.admin_user_id == Some(operator_user_id);
    if !is_owner && !is_admin {
        return mute_error(msg_id, 40301, "permission denied");
    }

    do_unmute_internal(
        room_id,
        target_user_id,
        mute_type,
        msg_id,
        operator_user_id,
        mute_redis,
        registry,
        room_manager,
        true,
    )
    .await
}

/// 核心解除逻辑（删 Redis key + 广播 UserMuted duration_sec=0）
#[allow(clippy::too_many_arguments)]
async fn do_unmute_internal(
    room_id: Uuid,
    target_user_id: Uuid,
    mute_type: &str,
    msg_id: Option<String>,
    operator_user_id: Uuid,
    mute_redis: &Arc<dyn MuteRedis>,
    registry: &Arc<ConnectionRegistry>,
    room_manager: &Arc<RoomManager>,
    from_mute_cmd: bool,
) -> String {
    // 删 Redis key
    if let Err(e) = mute_redis
        .del_mute(mute_type, room_id, target_user_id)
        .await
    {
        tracing::error!("del_mute failed: {e}");
        if from_mute_cmd {
            return mute_error(msg_id, 50000, "redis error");
        } else {
            return unmute_error(msg_id, 50000, "redis error");
        }
    }

    // 广播 UserMuted with duration_sec=0（解除）— 走统一出口 broadcast_to_room
    if let Some(rs) = room_manager.get_room(room_id) {
        broadcast_user_muted(
            registry,
            &rs,
            room_id,
            target_user_id,
            operator_user_id,
            mute_type,
            0,
            None,
        );
    }

    if from_mute_cmd {
        mute_success(msg_id)
    } else {
        unmute_success(msg_id)
    }
}
