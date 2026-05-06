//! KickUser 信令处理 — T-00028
//!
//! ## 处理流程
//! 1. 解析 payload（room_id / target_user_id / reason）
//! 2. 加载房间 Model → 权限校验（owner / admin）
//! 3. 目标不能是 owner；目标必须在房间内
//! 4. Redis SETEX kicked:{room_id}:{user_id} 600s（冷却 key）
//! 5. DB INSERT room_kick_records（审计，并发时多条可接受）
//! 6. RoomManager.remove_member 原子移除 → 只有第一个 Some 走完整流程
//! 7. 自动下麦 + 广播 MicLeft forced=true
//! 8. 广播 UserKicked 给目标；广播 UserLeft 给房间其他人
//! 9. 关闭目标 WS 连接（unregister）
//!
//! ## JoinRoom 冷却拦截
//! `handle_join_room` 前置调用 `KickRedis::get_kick_remaining_sec`，
//! 存在则返回 42911 + remaining_sec。

use std::sync::Arc;
#[cfg(any(test, feature = "test-utils"))]
use std::{collections::HashMap, sync::Mutex, time::Instant};

use async_trait::async_trait;
use uuid::Uuid;

use crate::common::error::AppError;
use crate::modules::room::service::RoomService;
use crate::room::handler::broadcast_mic_left;
use crate::room::manager::RoomManager;
use crate::ws::registry::ConnectionRegistry;

// ─── 冷却时长常量 ─────────────────────────────────────────────────────────────

/// 被踢用户的重进冷却时间（秒）
pub const KICK_COOLDOWN_SECS: i64 = 600;

// ─── KickRedis Trait ──────────────────────────────────────────────────────────

/// 踢人冷却 Redis 操作抽象。
///
/// 生产实现使用真实 Redis；测试使用 `FakeKickRedis`（内存 HashMap）。
#[async_trait]
pub trait KickRedis: Send + Sync {
    /// 设置踢出冷却 key：`kicked:{room_id}:{user_id}`，TTL = KICK_COOLDOWN_SECS。
    async fn set_kicked(&self, room_id: Uuid, user_id: Uuid, reason: &str) -> Result<(), AppError>;

    /// 查询冷却剩余秒数；key 不存在或已过期则返回 None。
    async fn get_kick_remaining_sec(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i64>, AppError>;
}

/// Blanket impl：允许 `Arc<T: KickRedis>` 直接用作 `&dyn KickRedis`
#[async_trait]
impl<T: KickRedis + ?Sized> KickRedis for Arc<T> {
    async fn set_kicked(&self, room_id: Uuid, user_id: Uuid, reason: &str) -> Result<(), AppError> {
        (**self).set_kicked(room_id, user_id, reason).await
    }

    async fn get_kick_remaining_sec(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i64>, AppError> {
        (**self).get_kick_remaining_sec(room_id, user_id).await
    }
}

// ─── KickAuditDb Trait ────────────────────────────────────────────────────────

/// 踢人审计记录 DB 操作抽象。
#[async_trait]
pub trait KickAuditDb: Send + Sync {
    /// 插入一条踢人审计记录（并发时可能插入多条，接受）。
    async fn insert_kick_record(
        &self,
        room_id: Uuid,
        operator_id: Uuid,
        target_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError>;
}

/// Blanket impl：允许 `Arc<T: KickAuditDb>` 直接用作 `&dyn KickAuditDb`
#[async_trait]
impl<T: KickAuditDb + ?Sized> KickAuditDb for Arc<T> {
    async fn insert_kick_record(
        &self,
        room_id: Uuid,
        operator_id: Uuid,
        target_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError> {
        (**self)
            .insert_kick_record(room_id, operator_id, target_id, reason)
            .await
    }
}

// ─── FakeKickRedis（测试用内存实现）─────────────────────────────────────────

#[cfg(any(test, feature = "test-utils"))]
struct KickRedisEntry {
    #[allow(dead_code)]
    reason: String,
    expires_at: Instant,
}

/// 内存踢人冷却 Redis（测试专用）。
#[cfg(any(test, feature = "test-utils"))]
pub struct FakeKickRedis {
    data: Mutex<HashMap<String, KickRedisEntry>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for FakeKickRedis {
    fn default() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeKickRedis {
    fn kick_key(room_id: Uuid, user_id: Uuid) -> String {
        format!("kicked:{room_id}:{user_id}")
    }

    /// 测试辅助：立即使所有 key 过期（模拟 10 分钟后）
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
    pub fn key_exists(&self, room_id: Uuid, user_id: Uuid) -> bool {
        let guard = self.data.lock().unwrap();
        let key = Self::kick_key(room_id, user_id);
        guard
            .get(&key)
            .map(|e| Instant::now() < e.expires_at)
            .unwrap_or(false)
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl KickRedis for FakeKickRedis {
    async fn set_kicked(&self, room_id: Uuid, user_id: Uuid, reason: &str) -> Result<(), AppError> {
        let key = Self::kick_key(room_id, user_id);
        let mut guard = self.data.lock().unwrap();
        guard.insert(
            key,
            KickRedisEntry {
                reason: reason.to_string(),
                expires_at: Instant::now()
                    + std::time::Duration::from_secs(KICK_COOLDOWN_SECS as u64),
            },
        );
        Ok(())
    }

    async fn get_kick_remaining_sec(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i64>, AppError> {
        let key = Self::kick_key(room_id, user_id);
        let guard = self.data.lock().unwrap();
        match guard.get(&key) {
            None => Ok(None),
            Some(entry) => {
                let now = Instant::now();
                if entry.expires_at <= now {
                    Ok(None)
                } else {
                    Ok(Some((entry.expires_at - now).as_secs() as i64))
                }
            }
        }
    }
}

// ─── FakeKickAuditDb（测试用内存实现）───────────────────────────────────────

/// 踢人审计记录（测试用快照）
#[cfg(any(test, feature = "test-utils"))]
#[derive(Debug, Clone)]
pub struct KickRecord {
    pub room_id: Uuid,
    pub operator_id: Uuid,
    pub target_id: Uuid,
    pub reason: String,
}

/// 内存踢人审计 DB（测试专用）。
#[cfg(any(test, feature = "test-utils"))]
pub struct FakeKickAuditDb {
    records: Mutex<Vec<KickRecord>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for FakeKickAuditDb {
    fn default() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeKickAuditDb {
    /// 测试辅助：返回已插入的审计记录数
    pub fn record_count(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    /// 测试辅助：获取所有审计记录（克隆）
    pub fn all_records(&self) -> Vec<KickRecord> {
        self.records.lock().unwrap().clone()
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl KickAuditDb for FakeKickAuditDb {
    async fn insert_kick_record(
        &self,
        room_id: Uuid,
        operator_id: Uuid,
        target_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError> {
        self.records.lock().unwrap().push(KickRecord {
            room_id,
            operator_id,
            target_id,
            reason: reason.to_string(),
        });
        Ok(())
    }
}

// ─── RealKickRedis（生产 Redis 实现）─────────────────────────────────────────

/// 生产 Redis 踢人冷却实现（基于真实 Redis Client）
pub struct RealKickRedis {
    client: redis::Client,
}

impl RealKickRedis {
    pub fn new(redis_url: &str) -> Result<Self, AppError> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| AppError::RedisError(format!("redis client open: {e}")))?;
        Ok(Self { client })
    }
}

#[async_trait]
impl KickRedis for RealKickRedis {
    async fn set_kicked(&self, room_id: Uuid, user_id: Uuid, reason: &str) -> Result<(), AppError> {
        use redis::AsyncCommands;
        let key = format!("kicked:{room_id}:{user_id}");
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        let _: () = conn
            .set_ex(&key, reason, KICK_COOLDOWN_SECS as u64)
            .await
            .map_err(|e| AppError::RedisError(e.to_string()))?;
        Ok(())
    }

    async fn get_kick_remaining_sec(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i64>, AppError> {
        use redis::AsyncCommands;
        let key = format!("kicked:{room_id}:{user_id}");
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

// ─── RealKickAuditDb（生产 DB 实现）─────────────────────────────────────────

/// 生产踢人审计 DB 实现（基于 sqlx PgPool）
pub struct RealKickAuditDb {
    pool: sqlx::PgPool,
}

impl RealKickAuditDb {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl KickAuditDb for RealKickAuditDb {
    async fn insert_kick_record(
        &self,
        room_id: Uuid,
        operator_id: Uuid,
        target_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO room_kick_records \
             (room_id, operator_user_id, target_user_id, reason) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(room_id)
        .bind(operator_id)
        .bind(target_id)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

// ─── KickDeps ─────────────────────────────────────────────────────────────────

/// `handle_kick` 所需的全部服务依赖。
pub struct KickDeps {
    /// 房间运行时状态管理器（成员表、麦位）
    pub room_manager: Arc<RoomManager>,
    /// 房间服务（权限校验：owner_id + admin_user_id）
    pub room_service: Arc<RoomService>,
    /// 踢人冷却 Redis
    pub redis: Arc<dyn KickRedis>,
    /// 踢人审计 DB
    pub audit_db: Arc<dyn KickAuditDb>,
    /// WS 连接注册表（广播 + 关闭连接）
    pub registry: Arc<ConnectionRegistry>,
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

fn kick_error(msg_id: Option<String>, code: i64, message: &str) -> String {
    crate::ws::broadcaster::build_outbound_result(
        "KickUserResult",
        msg_id,
        code,
        Some(serde_json::json!({ "message": message })),
    )
}

fn kick_success(msg_id: Option<String>) -> String {
    crate::ws::broadcaster::build_outbound_result("KickUserResult", msg_id, 0, None)
}

// ─── handle_kick ──────────────────────────────────────────────────────────────

/// 处理 KickUser 信令，返回 JSON 字符串响应。
///
/// ## 并发保护
/// `RoomManager::remove_member` 使用 `DashMap::remove()` 原子性：
/// 只有第一个拿到 `Some(member)` 的请求走完整广播/关闭流程；
/// 其余请求静默返回 code:0（已被踢出）。
///
/// Redis SETEX 覆盖写无副作用；
/// DB INSERT 允许多条（记录多位管理员的踢出操作）。
///
// PROTO-BINDING: doc/protocol/schemas/ws/KickUser.schema.json (C→S)
// PROTO-BINDING: doc/protocol/schemas/ws/UserKicked.schema.json (S→Target send)
// PROTO-BINDING: doc/protocol/schemas/ws/UserLeft.schema.json (S→Room broadcast)
// PROTO-BINDING: doc/protocol/schemas/ws/KickUserResult.schema.json (S→C result)
pub async fn handle_kick(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    operator_user_id: Uuid,
    deps: &KickDeps,
) -> String {
    let KickDeps {
        room_manager,
        room_service,
        redis,
        audit_db,
        registry,
    } = deps;

    // ── 1. 解析 payload ────────────────────────────────────────────────────────
    let room_id = match payload
        .as_ref()
        .and_then(|p| p.get("room_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return kick_error(msg_id, 40002, "missing room_id"),
    };

    let target_user_id = match payload
        .as_ref()
        .and_then(|p| p.get("target_user_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return kick_error(msg_id, 40002, "missing target_user_id"),
    };

    // K28-11: reason 空 → 40003
    let reason = match payload
        .as_ref()
        .and_then(|p| p.get("reason"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        Some(r) => r.to_string(),
        None => return kick_error(msg_id, 40003, "reason is required"),
    };

    // ── 2. 获取房间 Model（权限校验用）────────────────────────────────────────
    let room = match room_service.get_active_room_model(room_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return kick_error(msg_id, 40400, "room not found or closed"),
        Err(e) => {
            tracing::error!("get_active_room_model error: {e}");
            return kick_error(msg_id, 50000, "internal error");
        }
    };

    // ── 3. 权限校验：操作者必须是 owner 或 admin ──────────────────────────────
    let is_owner = room.owner_id == operator_user_id;
    let is_admin = room.admin_user_id == Some(operator_user_id);
    // K28-04: 普通用户 → 40301
    if !is_owner && !is_admin {
        return kick_error(msg_id, 40301, "permission denied");
    }

    // ── 4. 不能踢房主 ─────────────────────────────────────────────────────────
    // K28-05: target == owner → 40302
    if target_user_id == room.owner_id {
        return kick_error(msg_id, 40302, "cannot kick owner");
    }

    // ── 5. 检查 target 是否在房间 ─────────────────────────────────────────────
    // K28-06: target 不在房间 → 40400
    if !room_manager.is_member(room_id, target_user_id) {
        return kick_error(msg_id, 40400, "target not in room");
    }

    // ── 6. Redis：设置 10min 冷却 key ─────────────────────────────────────────
    if let Err(e) = redis.set_kicked(room_id, target_user_id, &reason).await {
        tracing::error!("set_kicked failed: {e}");
        return kick_error(msg_id, 50000, "redis error");
    }

    // ── 7. DB：插入审计记录（非阻断性，失败继续踢）─────────────────────────────
    // K28-10: 并发 3 请求 → 3 条审计记录
    if let Err(e) = audit_db
        .insert_kick_record(room_id, operator_user_id, target_user_id, &reason)
        .await
    {
        tracing::error!("insert_kick_record failed: {e}");
        // 非致命，继续执行
    }

    // ── 8. 原子移除成员（并发保护）────────────────────────────────────────────
    // K28-10: 只有第一个返回 Some 的走完整流程
    if room_manager
        .remove_member(room_id, target_user_id)
        .is_none()
    {
        // 已被其他管理员踢出，静默成功
        return kick_success(msg_id);
    }

    // ── 9. 自动下麦（若目标在麦上）+ 获取 room_state 用于广播 ─────────────────
    // K28-09: 踢麦上用户 → 广播 MicLeft forced=true
    let room_state_opt = room_manager.get_room(room_id);
    let mic_slot_left = room_state_opt
        .as_ref()
        .and_then(|s| s.leave_mic_slot(target_user_id));

    // ── 10. 获取操作者昵称（用于 UserKicked 广播）──────────────────────────────
    let operator_nickname = room_state_opt
        .as_ref()
        .and_then(|s| s.members.get(&operator_user_id).map(|m| m.nickname.clone()))
        .unwrap_or_else(|| "admin".to_string());

    // ── 11. 获取目标用户的全部 WS 连接 ────────────────────────────────────────
    let target_conns = registry.get_by_user_id(target_user_id);

    // ── 12. 清除目标连接的房间关联（确保后续房间广播不含目标）──────────────────
    for (conn_id, _) in &target_conns {
        registry.clear_room_id(*conn_id);
    }

    // ── 13. 向目标发送 UserKicked 信令（点对点，不入回放缓冲）──────────────────
    // K28-01/02: 目标收到 UserKicked
    // R1 P1-7: 走统一出口 build_outbound_envelope 注入 msg_id (UUID v4) + timestamp，
    // 前端可基于 msg_id 做 processed_msg_ids 去重，避免重连抖动重复弹"已被踢出"对话框。
    let (user_kicked_msg, _kick_msg_id) = crate::ws::broadcaster::build_outbound_envelope(
        "UserKicked",
        serde_json::json!({
            "room_id": room_id.to_string(),
            "reason": reason,
            "cooldown_sec": KICK_COOLDOWN_SECS,
            "operator_nickname": operator_nickname,
        }),
    );

    for (_, sender) in &target_conns {
        let _ = sender.send(user_kicked_msg.clone());
    }

    // ── 14. 广播 UserLeft 给房间其他成员（走统一出口 broadcast_to_room）──────
    // K28-03: 其他人收到 UserLeft reason=kicked_by_admin
    // PROTO-BINDING: doc/protocol/schemas/ws/UserLeft.schema.json
    if let Some(rs) = room_state_opt.as_ref() {
        let user_left_envelope = serde_json::json!({
            "type": "UserLeft",
            "payload": {
                "user_id": target_user_id.to_string(),
                "reason": "kicked_by_admin",
                "operator_id": operator_user_id.to_string(),
            },
            "timestamp": chrono::Utc::now().timestamp_millis(),
        });
        crate::ws::broadcaster::broadcast_to_room(registry, rs, user_left_envelope);

        // ── 15. 广播 MicLeft forced=true（若目标在麦上）──────────────────────
        if let Some(mic_index) = mic_slot_left {
            broadcast_mic_left(registry, rs, mic_index, target_user_id, true);
        }
    }

    // ── 16. 关闭目标 WS 连接（K28-12）────────────────────────────────────────
    for (conn_id, _) in &target_conns {
        registry.unregister(*conn_id);
    }

    kick_success(msg_id)
}
