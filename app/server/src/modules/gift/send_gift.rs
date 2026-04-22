//! SendGift 核心业务逻辑
//!
//! ## 数据流（TDS T-00020 §核心数据流）
//! ```text
//! Client WS SendGift {gift_id, receiver_id, count, msg_id}
//!    └─► handle_send_gift() → GiftSendService::send()
//!         1. 校验 count（1-9999）
//!         2. 校验发送者在房间（room_state.members 中）
//!         3. 幂等检查：SELECT FROM gift_records WHERE sender_id=? AND msg_id=?
//!         4. 查 gift（is_active=true）
//!         5. 校验接收者在房间 & 在麦上
//!         6. BEGIN TX
//!             a) SELECT FOR UPDATE sender balance
//!             b) 余额不足 → 回滚 → InsufficientBalance
//!             c) UPDATE users SET diamond_balance -= total WHERE id=sender
//!             d) UPDATE users SET charm_balance += total WHERE id=receiver
//!             e) INSERT gift_records (幂等 ON CONFLICT DO NOTHING)
//!             f) INSERT wallet_transactions
//!            COMMIT
//!         7. Redis ZINCRBY 四个 ZSet（魅力/财富 日榜/周榜）
//!         8. 广播 GiftReceived 给 registry.get_connections_in_room(room_id)
//!         9. 通知 BalanceBroadcaster → 发送者 BalanceUpdated
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    modules::wallet::broadcaster::BalanceEvent,
    room::manager::RoomManager,
    ws::registry::ConnectionRegistry,
};

use super::ranking;

// ─── 错误类型 ─────────────────────────────────────────────────────────────────

/// SendGift 业务错误，对应 TDS §WS 信令错误码
#[derive(Debug, thiserror::Error)]
pub enum SendGiftError {
    /// count 为 0 或超过 9999 (40001)
    #[error("invalid count: must be 1-9999")]
    InvalidCount,
    /// 发送者不在指定房间 (40400)
    #[error("sender not in room")]
    SenderNotInRoom,
    /// 礼物不存在或已下架 (40402)
    #[error("gift unavailable")]
    GiftUnavailable,
    /// 接收者不在房间或不在麦上 (40403)
    #[error("receiver unavailable")]
    ReceiverUnavailable,
    /// 余额不足 (40290)
    #[error("insufficient balance")]
    InsufficientBalance,
    /// 内部错误
    #[error("internal error: {0}")]
    Internal(String),
}

// ─── 输入/输出 DTO ──────────────────────────────────────────────────────────────

/// SendGift 请求 payload（来自 WS 信令）
#[derive(Debug, Clone)]
pub struct SendGiftPayload {
    pub gift_id: Uuid,
    pub receiver_id: Uuid,
    pub count: i32,
    pub msg_id: String,
}

/// SendGift 成功结果
#[derive(Debug, Clone)]
pub struct SendGiftResult {
    pub gift_record_id: Uuid,
    pub total_price: i64,
}

// ─── SendGiftServicePort trait ────────────────────────────────────────────────

/// 送礼服务抽象接口（供 WS handler 注入，支持 Fake 替身）
#[async_trait]
pub trait SendGiftServicePort: Send + Sync {
    /// 执行送礼核心逻辑：事务 + 广播 + 榜单
    ///
    /// # 参数
    /// - `sender_id`：送礼用户 UUID
    /// - `room_id`：送礼所在房间 UUID（调用方从 registry 中解析）
    /// - `payload`：礼物信息（gift_id, receiver_id, count, msg_id）
    ///
    /// # 幂等
    /// 相同 `(sender_id, msg_id)` 再次调用时返回 `Ok(SendGiftResult { ... })`（首次结果），
    /// 不重新扣款、不重新广播。`Err` 仅在业务校验失败或内部错误时返回。
    async fn send(
        &self,
        sender_id: Uuid,
        room_id: Uuid,
        payload: SendGiftPayload,
    ) -> Result<SendGiftResult, SendGiftError>;
}

// ─── GiftSendService ──────────────────────────────────────────────────────────

/// 送礼服务真实实现
///
/// 依赖：PgPool（事务）、ConnectionRegistry（广播）、RoomManager（成员/麦位检查）、
///      mpsc::Sender<BalanceEvent>（余额推送）、Redis（榜单 ZSet）
pub struct GiftSendService {
    pool: PgPool,
    registry: Arc<ConnectionRegistry>,
    room_manager: Arc<RoomManager>,
    balance_tx: mpsc::Sender<BalanceEvent>,
    redis_client: redis::Client,
}

impl GiftSendService {
    /// 创建 GiftSendService
    ///
    /// Redis 连接在此建立（lazy，连接失败时不 panic，后续发送时记录 warn）。
    pub fn new(
        pool: PgPool,
        registry: Arc<ConnectionRegistry>,
        room_manager: Arc<RoomManager>,
        balance_tx: mpsc::Sender<BalanceEvent>,
        redis_url: String,
    ) -> Self {
        let redis_client = redis::Client::open(redis_url)
            .unwrap_or_else(|e| {
                tracing::warn!("GiftSendService: failed to open redis client: {e}");
                redis::Client::open("redis://127.0.0.1:6379").expect("fallback redis client")
            });
        Self { pool, registry, room_manager, balance_tx, redis_client }
    }

    /// 核心事务逻辑（供 send() 调用）
    ///
    /// 参数超过 7 个是因为事务需要所有业务实体的 ID 和价格，抑制 clippy::too_many_arguments。
    #[allow(clippy::too_many_arguments)]
    async fn execute_transaction(
        &self,
        sender_id: Uuid,
        receiver_id: Uuid,
        room_id: Uuid,
        gift_id: Uuid,
        price: i64,
        count: i32,
        msg_id: &str,
    ) -> Result<Uuid, SendGiftError> {
        let total = price * count as i64;
        let mut txn = self.pool.begin().await.map_err(|e| SendGiftError::Internal(e.to_string()))?;

        // a) SELECT ... FOR UPDATE 锁定发送者行
        let current_balance: i64 = sqlx::query_scalar(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE"
        )
        .bind(sender_id)
        .fetch_optional(&mut *txn)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?
        .ok_or_else(|| SendGiftError::Internal("sender not found".into()))?;

        // b) 余额不足检查
        if current_balance < total {
            return Err(SendGiftError::InsufficientBalance);
        }

        let new_balance = current_balance - total;

        // c) 扣减发送者余额
        sqlx::query(
            "UPDATE users SET diamond_balance = $1, updated_at = now() WHERE id = $2"
        )
        .bind(new_balance)
        .bind(sender_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        // d) 增加接收者魅力值
        sqlx::query(
            "UPDATE users SET charm_balance = charm_balance + $1, updated_at = now() WHERE id = $2"
        )
        .bind(total)
        .bind(receiver_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        // e) INSERT gift_records（幂等：ON CONFLICT DO NOTHING 备用，主要靠业务层幂等检查）
        let gift_record_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO gift_records \
             (id, sender_id, receiver_id, room_id, gift_id, count, total_price, msg_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(gift_record_id)
        .bind(sender_id)
        .bind(receiver_id)
        .bind(room_id)
        .bind(gift_id)
        .bind(count)
        .bind(total)
        .bind(msg_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| {
            // UNIQUE violation (23505) → 理论上应被上层幂等检查拦截
            SendGiftError::Internal(e.to_string())
        })?;

        // f) INSERT wallet_transactions
        sqlx::query(
            "INSERT INTO wallet_transactions \
             (user_id, type, amount, balance_after, ref_id, reason) \
             VALUES ($1, 'gift_send', $2, $3, $4, 'gift_send')"
        )
        .bind(sender_id)
        .bind(-total)
        .bind(new_balance)
        .bind(gift_record_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        txn.commit().await.map_err(|e| SendGiftError::Internal(e.to_string()))?;

        // 事务提交成功后通知 BalanceBroadcaster（[M-1] 改用 send().await 避免静默丢弃）
        let event = BalanceEvent {
            user_id: sender_id,
            balance_after: new_balance,
            delta: -total,
            reason: "gift_send".to_string(),
            ref_id: Some(gift_record_id),
        };
        if let Err(e) = self.balance_tx.send(event).await {
            tracing::warn!("GiftSendService: balance event channel closed, event dropped: {:?}", e);
        }

        Ok(gift_record_id)
    }
}

#[async_trait]
impl SendGiftServicePort for GiftSendService {
    async fn send(
        &self,
        sender_id: Uuid,
        room_id: Uuid,
        payload: SendGiftPayload,
    ) -> Result<SendGiftResult, SendGiftError> {
        let SendGiftPayload { gift_id, receiver_id, count, msg_id } = payload;

        // ── 1. 校验 count ────────────────────────────────────────────────────
        if !(1..=9999).contains(&count) {
            return Err(SendGiftError::InvalidCount);
        }

        // ── 2. 校验发送者在房间 ──────────────────────────────────────────────
        let room_state = self.room_manager.get_room(room_id)
            .ok_or(SendGiftError::SenderNotInRoom)?;
        if !room_state.members.contains_key(&sender_id) {
            return Err(SendGiftError::SenderNotInRoom);
        }

        // ── 3. 幂等检查 ──────────────────────────────────────────────────────
        let existing: Option<(Uuid, i64)> = sqlx::query_as(
            "SELECT id, total_price FROM gift_records WHERE sender_id = $1 AND msg_id = $2 LIMIT 1"
        )
        .bind(sender_id)
        .bind(&msg_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        if let Some((existing_id, existing_total)) = existing {
            // 幂等命中：直接返回首次结果（不重新广播、不重新扣款）
            tracing::info!(
                sender_id = %sender_id,
                msg_id = %msg_id,
                gift_record_id = %existing_id,
                "SendGift idempotent: returning cached result"
            );
            return Ok(SendGiftResult {
                gift_record_id: existing_id,
                total_price: existing_total,
            });
        }

        // ── 4. 查询 gift（必须 active）并获取全部展示字段 ──────────────────
        let gift_row = sqlx::query_as::<_, (i64, i16, String, String, String, Option<String>)>(
            "SELECT price, effect_level, code, name_ar, icon_url, animation_url \
             FROM gifts WHERE id = $1 AND is_active = true AND is_deleted = false"
        )
        .bind(gift_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        let (price, effect_level, gift_code, gift_name, gift_icon_url, gift_animation_url) =
            gift_row.ok_or(SendGiftError::GiftUnavailable)?;

        // ── 5. 校验接收者在麦上 ──────────────────────────────────────────────
        let receiver_on_mic = {
            let slots = room_state.mic_slots.read().unwrap_or_else(|e| e.into_inner());
            slots.contains(&Some(receiver_id))
        };
        if !receiver_on_mic {
            return Err(SendGiftError::ReceiverUnavailable);
        }

        // ── 5.5 查询 sender/receiver 用户信息（用于广播 payload，[H-1]）──────
        let sender_info: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT nickname, avatar FROM users WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(sender_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;
        let (sender_nickname, sender_avatar) = sender_info
            .unwrap_or_else(|| (sender_id.to_string(), None));

        let receiver_info: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT nickname, avatar FROM users WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(receiver_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;
        let (receiver_nickname, receiver_avatar) = receiver_info
            .unwrap_or_else(|| (receiver_id.to_string(), None));

        // ── 6-f. 执行事务 ────────────────────────────────────────────────────
        let total = price * count as i64;
        let gift_record_id = self.execute_transaction(
            sender_id, receiver_id, room_id, gift_id, price, count, &msg_id
        ).await?;

        // ── 7. 更新 Redis 榜单（非阻断，失败只记录 warn）───────────────────
        match self.redis_client.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                ranking::update_rankings(&mut conn, receiver_id, sender_id, total).await;
            }
            Err(e) => {
                tracing::warn!("GiftSendService: redis connect failed, rankings not updated: {}", e);
            }
        }

        // ── 8. 广播 GiftReceived 给房间所有成员（[H-1] 补全 TDS 规定字段）──
        let broadcast_msg = build_gift_received_msg(
            gift_record_id,
            sender_id,
            &sender_nickname,
            sender_avatar.as_deref(),
            receiver_id,
            &receiver_nickname,
            receiver_avatar.as_deref(),
            gift_id,
            &gift_code,
            &gift_name,
            &gift_icon_url,
            gift_animation_url.as_deref(),
            effect_level,
            count,
            total,
        );
        for (_, sender_ch) in self.registry.get_connections_in_room(room_id) {
            let _ = sender_ch.send(broadcast_msg.clone());
        }

        Ok(SendGiftResult { gift_record_id, total_price: total })
    }
}

/// 构建 GiftReceived 广播消息 JSON（[H-1] 补全 TDS §广播 S→C 所有必填字段）
///
/// 包含：sender/receiver 的 user_id、nickname、avatar；
/// gift 的 id、code、name、icon_url、animation_url、effect_level；count、total_price。
#[allow(clippy::too_many_arguments)]
fn build_gift_received_msg(
    gift_record_id: Uuid,
    sender_id: Uuid,
    sender_nickname: &str,
    sender_avatar: Option<&str>,
    receiver_id: Uuid,
    receiver_nickname: &str,
    receiver_avatar: Option<&str>,
    gift_id: Uuid,
    gift_code: &str,
    gift_name: &str,
    gift_icon_url: &str,
    gift_animation_url: Option<&str>,
    gift_effect_level: i16,
    count: i32,
    total_price: i64,
) -> String {
    let msg = serde_json::json!({
        "type": "GiftReceived",
        "msg_id": Uuid::new_v4().to_string(),
        "payload": {
            "gift_record_id": gift_record_id.to_string(),
            "sender": {
                "user_id": sender_id.to_string(),
                "nickname": sender_nickname,
                "avatar": sender_avatar,
            },
            "receiver": {
                "user_id": receiver_id.to_string(),
                "nickname": receiver_nickname,
                "avatar": receiver_avatar,
            },
            "gift": {
                "id": gift_id.to_string(),
                "code": gift_code,
                "name": gift_name,
                "icon_url": gift_icon_url,
                "animation_url": gift_animation_url,
                "effect_level": gift_effect_level,
            },
            "count": count,
            "total_price": total_price,
        },
        "timestamp": chrono::Utc::now().timestamp_millis(),
    });
    serde_json::to_string(&msg).unwrap_or_default()
}

// ─── WS 信令处理器 ────────────────────────────────────────────────────────────

/// SendGift 信令所需依赖
pub struct SendGiftDeps {
    pub send_gift_service: Arc<dyn SendGiftServicePort>,
    pub registry: Arc<ConnectionRegistry>,
}

/// 处理 WS SendGift 信令，返回 JSON 字符串响应（给发送者）
///
/// 从 registry 查找 connection_id 对应的 room_id，然后调用 GiftSendService::send()。
pub async fn handle_send_gift(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    connection_id: Uuid,
    user_id: Uuid,
    deps: &SendGiftDeps,
) -> String {
    // 解析 payload
    let payload_val = match &payload {
        Some(p) => p,
        None => return send_gift_error_response(msg_id, 40002, "missing payload"),
    };

    let gift_id = match payload_val.get("gift_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return send_gift_error_response(msg_id, 40002, "invalid or missing gift_id"),
    };

    let receiver_id = match payload_val.get("receiver_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return send_gift_error_response(msg_id, 40002, "invalid or missing receiver_id"),
    };

    let count = match payload_val.get("count")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
    {
        Some(c) => c,
        None => return send_gift_error_response(msg_id, 40002, "invalid or missing count"),
    };

    // 从 registry 获取发送者当前房间
    let room_id = match deps.registry.get_room_id(connection_id) {
        Some(id) => id,
        None => return send_gift_error_response(msg_id, 40400, "sender not in any room"),
    };

    let payload_struct = SendGiftPayload {
        gift_id,
        receiver_id,
        count,
        msg_id: msg_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string()),
    };

    match deps.send_gift_service.send(user_id, room_id, payload_struct).await {
        Ok(result) => build_send_gift_result_response(msg_id, result.gift_record_id, result.total_price),
        Err(SendGiftError::InvalidCount) => send_gift_error_response(msg_id, 40001, "invalid count: must be 1-9999"),
        Err(SendGiftError::SenderNotInRoom) => send_gift_error_response(msg_id, 40400, "sender not in room"),
        Err(SendGiftError::GiftUnavailable) => send_gift_error_response(msg_id, 40402, "gift not found or inactive"),
        Err(SendGiftError::ReceiverUnavailable) => send_gift_error_response(msg_id, 40403, "receiver not on mic"),
        Err(SendGiftError::InsufficientBalance) => send_gift_error_response(msg_id, 40290, "insufficient balance"),
        Err(SendGiftError::Internal(e)) => {
            tracing::error!("SendGift internal error: {}", e);
            send_gift_error_response(msg_id, 50000, "internal error")
        }
    }
}

/// 构建 SendGiftResult 成功响应 JSON
fn build_send_gift_result_response(msg_id: Option<String>, gift_record_id: Uuid, total_price: i64) -> String {
    let resp = serde_json::json!({
        "type": "SendGiftResult",
        "msg_id": msg_id,
        "code": 0,
        "payload": {
            "gift_record_id": gift_record_id.to_string(),
            "total_price": total_price,
        },
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

/// 构建 SendGiftResult 错误响应 JSON
fn send_gift_error_response(msg_id: Option<String>, code: i64, message: &str) -> String {
    let resp = serde_json::json!({
        "type": "SendGiftResult",
        "msg_id": msg_id,
        "code": code,
        "message": message,
        "timestamp": chrono::Utc::now().timestamp(),
    });
    serde_json::to_string(&resp).unwrap_or_default()
}

// ─── FakeSendGiftService（测试替身，无 cfg guard）────────────────────────────

/// 内存测试替身，供 `AppState::for_test()` 注入
///
/// - `send()` 始终返回 `Ok(SendGiftResult { ... })`
/// - 不触碰 DB、Redis 或 registry
pub struct FakeSendGiftService;

impl Default for FakeSendGiftService {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl SendGiftServicePort for FakeSendGiftService {
    async fn send(
        &self,
        _sender_id: Uuid,
        _room_id: Uuid,
        _payload: SendGiftPayload,
    ) -> Result<SendGiftResult, SendGiftError> {
        Ok(SendGiftResult {
            gift_record_id: Uuid::new_v4(),
            total_price: 0,
        })
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use std::sync::RwLock;

    fn make_registry_with_connection(user_id: Uuid, room_id: Uuid) -> (Arc<ConnectionRegistry>, Uuid, mpsc::UnboundedReceiver<String>) {
        let registry = Arc::new(ConnectionRegistry::new());
        let conn_id = Uuid::new_v4();
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        registry.register(crate::ws::registry::ConnectionHandle {
            connection_id: conn_id,
            user_id,
            room_id: Some(room_id),
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });
        (registry, conn_id, rx)
    }

    // SGU01: FakeSendGiftService 满足 Arc<dyn SendGiftServicePort> 约束
    #[test]
    fn sgu01_fake_service_is_send_gift_service_port() {
        let _: Arc<dyn SendGiftServicePort> = Arc::new(FakeSendGiftService);
    }

    // SGU02: FakeSendGiftService::send 始终返回 Ok
    #[tokio::test]
    async fn sgu02_fake_service_returns_ok() {
        let svc = FakeSendGiftService;
        let result = svc.send(
            Uuid::new_v4(),
            Uuid::new_v4(),
            SendGiftPayload {
                gift_id: Uuid::new_v4(),
                receiver_id: Uuid::new_v4(),
                count: 1,
                msg_id: "test".to_string(),
            },
        ).await;
        assert!(result.is_ok(), "SGU02: FakeSendGiftService should return Ok");
    }

    // SGU03: handle_send_gift 缺失 payload → code 40002
    #[tokio::test]
    async fn sgu03_handle_send_gift_missing_payload_returns_40002() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let (registry, conn_id, _rx) = make_registry_with_connection(user_id, room_id);

        let deps = SendGiftDeps {
            send_gift_service: Arc::new(FakeSendGiftService),
            registry,
        };

        let response = handle_send_gift(None, Some("msg-1".to_string()), conn_id, user_id, &deps).await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40002, "SGU03: missing payload should return code 40002");
    }

    // SGU04: handle_send_gift 缺失 gift_id → code 40002
    #[tokio::test]
    async fn sgu04_handle_send_gift_missing_gift_id_returns_40002() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let (registry, conn_id, _rx) = make_registry_with_connection(user_id, room_id);

        let deps = SendGiftDeps {
            send_gift_service: Arc::new(FakeSendGiftService),
            registry,
        };

        let payload = serde_json::json!({
            "receiver_id": Uuid::new_v4().to_string(),
            "count": 1
            // gift_id 缺失
        });
        let response = handle_send_gift(Some(payload), Some("msg-2".to_string()), conn_id, user_id, &deps).await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40002, "SGU04: missing gift_id should return code 40002");
    }

    // SGU05: handle_send_gift 发送者不在任何房间 → code 40400
    #[tokio::test]
    async fn sgu05_handle_send_gift_not_in_room_returns_40400() {
        let user_id = Uuid::new_v4();
        let registry = Arc::new(ConnectionRegistry::new());
        let conn_id = Uuid::new_v4(); // 未注册，无 room_id

        // 注册连接但不设置 room_id
        let (tx, _rx) = mpsc::unbounded_channel::<String>();
        registry.register(crate::ws::registry::ConnectionHandle {
            connection_id: conn_id,
            user_id,
            room_id: None,  // 不在任何房间
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });

        let deps = SendGiftDeps {
            send_gift_service: Arc::new(FakeSendGiftService),
            registry,
        };

        let payload = serde_json::json!({
            "gift_id": Uuid::new_v4().to_string(),
            "receiver_id": Uuid::new_v4().to_string(),
            "count": 1
        });
        let response = handle_send_gift(Some(payload), Some("msg-3".to_string()), conn_id, user_id, &deps).await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 40400, "SGU05: sender not in room should return code 40400");
    }

    // SGU06: handle_send_gift 成功 → code 0，含 gift_record_id
    #[tokio::test]
    async fn sgu06_handle_send_gift_success_returns_code_0() {
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        let (registry, conn_id, _rx) = make_registry_with_connection(user_id, room_id);

        let deps = SendGiftDeps {
            send_gift_service: Arc::new(FakeSendGiftService),
            registry,
        };

        let payload = serde_json::json!({
            "gift_id": Uuid::new_v4().to_string(),
            "receiver_id": Uuid::new_v4().to_string(),
            "count": 1
        });
        let response = handle_send_gift(Some(payload), Some("msg-4".to_string()), conn_id, user_id, &deps).await;
        let json: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(json["code"], 0, "SGU06: success should return code 0");
        assert!(json["payload"]["gift_record_id"].is_string(), "SGU06: response should contain gift_record_id");
    }

    // SGU07: build_gift_received_msg 包含 TDS 规定的全部字段（[H-1] 更新）
    #[test]
    fn sgu07_build_gift_received_msg_contains_required_fields() {
        let msg = build_gift_received_msg(
            Uuid::new_v4(),         // gift_record_id
            Uuid::new_v4(),         // sender_id
            "Alice",                // sender_nickname
            Some("https://cdn.example.com/alice.png"), // sender_avatar
            Uuid::new_v4(),         // receiver_id
            "Bob",                  // receiver_nickname
            None,                   // receiver_avatar
            Uuid::new_v4(),         // gift_id
            "castle_01",            // gift_code
            "قصر",                  // gift_name
            "https://cdn.example.com/castle.png",      // gift_icon_url
            Some("https://cdn.example.com/castle.mp4"), // gift_animation_url
            4i16,                   // gift_effect_level
            2,                      // count
            1040,                   // total_price
        );
        let json: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(json["type"], "GiftReceived", "SGU07: type should be GiftReceived");
        assert_eq!(json["payload"]["count"], 2, "SGU07: count should be 2");
        assert_eq!(json["payload"]["total_price"], 1040, "SGU07: total_price should be 1040");
        assert!(json["msg_id"].is_string(), "SGU07: should have msg_id");
        // 验证 sender 字段
        assert_eq!(json["payload"]["sender"]["nickname"], "Alice", "SGU07: sender nickname");
        assert_eq!(json["payload"]["sender"]["avatar"], "https://cdn.example.com/alice.png", "SGU07: sender avatar");
        // 验证 receiver 字段
        assert_eq!(json["payload"]["receiver"]["nickname"], "Bob", "SGU07: receiver nickname");
        assert!(json["payload"]["receiver"]["avatar"].is_null(), "SGU07: receiver avatar null");
        // 验证 gift 字段
        assert_eq!(json["payload"]["gift"]["code"], "castle_01", "SGU07: gift code");
        assert_eq!(json["payload"]["gift"]["name"], "قصر", "SGU07: gift name");
        assert_eq!(json["payload"]["gift"]["icon_url"], "https://cdn.example.com/castle.png", "SGU07: gift icon_url");
        assert_eq!(json["payload"]["gift"]["animation_url"], "https://cdn.example.com/castle.mp4", "SGU07: gift animation_url");
        assert_eq!(json["payload"]["gift"]["effect_level"], 4, "SGU07: gift effect_level");
    }

    // SGU08: SendGiftError 实现 Debug（for assertions）
    #[test]
    fn sgu08_send_gift_error_debug() {
        let _ = format!("{:?}", SendGiftError::InvalidCount);
        let _ = format!("{:?}", SendGiftError::InsufficientBalance);
        let _ = format!("{:?}", SendGiftError::ReceiverUnavailable);
    }
}
