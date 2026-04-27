//! `GiftSendService` — 真实送礼服务实现
//!
//! 拆分自原 `send_gift.rs`（缺陷 #5）。本文件集中以下三处修复：
//!
//! - **缺陷 #2 (P1)** — 幂等竞争窗口：原实现先 `SELECT` 再 `INSERT`，并发同 msg_id
//!   会出现 UNIQUE 冲突 `Internal` 误报。本实现改用
//!   `INSERT ... ON CONFLICT (sender_id,msg_id) DO NOTHING RETURNING id`：
//!     * `RETURNING` 命中 → 首次插入，事务正常推进
//!     * `RETURNING` 为空 → 已存在重复，事务回滚（drop txn 自动 rollback），
//!       `send()` 外层捕获后再次 `SELECT` 返回首次记录的 `(id, total_price)`，
//!       保证返回值与首次完全一致。
//! - **缺陷 #4 (P1)** — Redis URL fail-fast：`new()` 改返回 `anyhow::Result<Self>`，
//!   不再 fallback 到 `redis://127.0.0.1:6379`，避免生产环境配置错误时静默退化。
//! - **缺陷 #6 (P2)** — sender / receiver 用户信息批量查询：
//!   原两次串行 `SELECT` 改为单次 `WHERE id = ANY($1)`。

use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    modules::wallet::broadcaster::BalanceEvent, room::manager::RoomManager,
    ws::registry::ConnectionRegistry,
};

use super::super::ranking;
use super::messages::build_gift_received_msg;
use super::types::{SendGiftError, SendGiftPayload, SendGiftResult, SendGiftServicePort};

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
    /// 创建 `GiftSendService`
    ///
    /// **缺陷 #4 修复**：Redis URL 解析失败立即返回 `Err`，不再 fallback 到
    /// `redis://127.0.0.1:6379` —— 避免生产环境配置错误时静默退化。
    pub fn new(
        pool: PgPool,
        registry: Arc<ConnectionRegistry>,
        room_manager: Arc<RoomManager>,
        balance_tx: mpsc::Sender<BalanceEvent>,
        redis_url: String,
    ) -> anyhow::Result<Self> {
        let redis_client = redis::Client::open(redis_url.as_str())
            .map_err(|e| anyhow::anyhow!("REDIS_URL invalid for GiftSendService: {e}"))?;
        Ok(Self {
            pool,
            registry,
            room_manager,
            balance_tx,
            redis_client,
        })
    }

    /// 核心事务逻辑（供 `send()` 调用）。
    ///
    /// **缺陷 #2 修复**：返回 `Ok(Some((id, sender_balance, receiver_charm)))` 表示首次插入成功；
    /// 返回 `Ok(None)` 表示 `(sender_id, msg_id)` 已存在（事务自动回滚）。
    /// `send()` 据此决定是否回查首次结果以保证幂等返回值一致。
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
    ) -> Result<Option<(Uuid, i64, i64)>, SendGiftError> {
        let total = price * count as i64;
        let mut txn = self
            .pool
            .begin()
            .await
            .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        // a) SELECT ... FOR UPDATE 锁定发送者行
        let current_balance: i64 = sqlx::query_scalar(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
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
        sqlx::query("UPDATE users SET diamond_balance = $1, updated_at = now() WHERE id = $2")
            .bind(new_balance)
            .bind(sender_id)
            .execute(&mut *txn)
            .await
            .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        // d) 增加接收者魅力值，并返回新值
        let receiver_new_charm: i64 = sqlx::query_scalar(
            "UPDATE users SET charm_balance = charm_balance + $1, updated_at = now() WHERE id = $2 RETURNING charm_balance",
        )
        .bind(total)
        .bind(receiver_id)
        .fetch_one(&mut *txn)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        // e) INSERT gift_records — ON CONFLICT DO NOTHING RETURNING id（缺陷 #2）
        let gift_record_id_new = Uuid::new_v4();
        let inserted_id: Option<Uuid> = sqlx::query_scalar(
            "INSERT INTO gift_records \
             (id, sender_id, receiver_id, room_id, gift_id, count, total_price, msg_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
             ON CONFLICT (sender_id, msg_id) DO NOTHING \
             RETURNING id",
        )
        .bind(gift_record_id_new)
        .bind(sender_id)
        .bind(receiver_id)
        .bind(room_id)
        .bind(gift_id)
        .bind(count)
        .bind(total)
        .bind(msg_id)
        .fetch_optional(&mut *txn)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        let gift_record_id = match inserted_id {
            Some(id) => id,
            None => {
                // 幂等命中：drop txn → 余额/魅力扣减自动回滚
                tracing::info!(
                    sender_id = %sender_id,
                    msg_id = %msg_id,
                    "SendGift idempotent: ON CONFLICT in execute_transaction, rolling back"
                );
                return Ok(None);
            }
        };

        // f) INSERT wallet_transactions
        sqlx::query(
            "INSERT INTO wallet_transactions \
             (user_id, type, amount, balance_after, ref_id, reason) \
             VALUES ($1, 'gift_send', $2, $3, $4, 'gift_send')",
        )
        .bind(sender_id)
        .bind(-total)
        .bind(new_balance)
        .bind(gift_record_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        // 事务提交成功后通知 BalanceBroadcaster
        let event = BalanceEvent {
            user_id: sender_id,
            balance_after: new_balance,
            delta: -total,
            reason: "gift_send".to_string(),
            ref_id: Some(gift_record_id),
        };
        if let Err(e) = self.balance_tx.send(event).await {
            tracing::warn!(
                "GiftSendService: balance event channel closed, event dropped: {:?}",
                e
            );
        }

        Ok(Some((gift_record_id, new_balance, receiver_new_charm)))
    }

    /// 查询单条幂等命中记录的 (id, total_price, sender_balance, receiver_charm)
    async fn lookup_existing(
        &self,
        sender_id: Uuid,
        receiver_id: Uuid,
        msg_id: &str,
    ) -> Result<Option<(Uuid, i64, i64, i64)>, SendGiftError> {
        // 查询 gift_record 和 users 表
        let record: Option<(Uuid, i64)> = sqlx::query_as(
            "SELECT id, total_price FROM gift_records WHERE sender_id = $1 AND msg_id = $2 LIMIT 1",
        )
        .bind(sender_id)
        .bind(msg_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        if let Some((id, total)) = record {
            // 查询当前余额和魅力值
            let sender_balance: i64 = sqlx::query_scalar(
                "SELECT diamond_balance FROM users WHERE id = $1",
            )
            .bind(sender_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| SendGiftError::Internal(e.to_string()))?;

            let receiver_charm: i64 = sqlx::query_scalar(
                "SELECT charm_balance FROM users WHERE id = $1",
            )
            .bind(receiver_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| SendGiftError::Internal(e.to_string()))?;

            Ok(Some((id, total, sender_balance, receiver_charm)))
        } else {
            Ok(None)
        }
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
        let SendGiftPayload {
            gift_id,
            receiver_id,
            count,
            msg_id,
        } = payload;

        // ── 1. 校验 count ────────────────────────────────────────────────────
        if !(1..=9999).contains(&count) {
            return Err(SendGiftError::InvalidCount);
        }

        // ── 2. 校验发送者在房间 ──────────────────────────────────────────────
        let room_state = self
            .room_manager
            .get_room(room_id)
            .ok_or(SendGiftError::SenderNotInRoom)?;
        if !room_state.members.contains_key(&sender_id) {
            return Err(SendGiftError::SenderNotInRoom);
        }

        // ── 3. 幂等快路径：先查一次（节省常见情况下的事务开销）─────────────
        if let Some((existing_id, existing_total, sender_balance, receiver_charm)) =
            self.lookup_existing(sender_id, receiver_id, &msg_id).await?
        {
            tracing::info!(
                sender_id = %sender_id,
                msg_id = %msg_id,
                gift_record_id = %existing_id,
                "SendGift idempotent (fast path): returning cached result"
            );
            return Ok(SendGiftResult {
                gift_record_id: existing_id,
                total_price: existing_total,
                sender_new_balance: sender_balance,
                receiver_new_charm: receiver_charm,
            });
        }

        // ── 4. 查询 gift（必须 active）并获取全部展示字段 ──────────────────
        let gift_row = sqlx::query_as::<_, (i64, i16, String, String, String, Option<String>)>(
            "SELECT price, effect_level, code, name_ar, icon_url, animation_url \
             FROM gifts WHERE id = $1 AND is_active = true AND is_deleted = false",
        )
        .bind(gift_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        let (price, effect_level, gift_code, gift_name, gift_icon_url, gift_animation_url) =
            gift_row.ok_or(SendGiftError::GiftUnavailable)?;

        // ── 5. 校验接收者在麦上 ──────────────────────────────────────────────
        let receiver_on_mic = {
            let slots = room_state
                .mic_slots
                .read()
                .unwrap_or_else(|e| e.into_inner());
            slots.contains(&Some(receiver_id))
        };
        if !receiver_on_mic {
            return Err(SendGiftError::ReceiverUnavailable);
        }

        // ── 5.5 批量查询 sender / receiver 用户信息（缺陷 #6）──────────────
        let user_rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT id, nickname, avatar FROM users \
             WHERE id = ANY($1) AND deleted_at IS NULL",
        )
        .bind(&[sender_id, receiver_id][..])
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SendGiftError::Internal(e.to_string()))?;

        let mut sender_nickname = sender_id.to_string();
        let mut sender_avatar: Option<String> = None;
        let mut receiver_nickname = receiver_id.to_string();
        let mut receiver_avatar: Option<String> = None;
        for (id, nickname, avatar) in user_rows {
            if id == sender_id {
                sender_nickname = nickname;
                sender_avatar = avatar;
            } else if id == receiver_id {
                receiver_nickname = nickname;
                receiver_avatar = avatar;
            }
        }

        // ── 6-f. 执行事务（含幂等 ON CONFLICT，缺陷 #2）─────────────────────
        let total = price * count as i64;
        let (gift_record_id, sender_new_balance, receiver_new_charm) = match self
            .execute_transaction(
                sender_id,
                receiver_id,
                room_id,
                gift_id,
                price,
                count,
                &msg_id,
            )
            .await?
        {
            Some(result) => result,
            None => {
                // 并发提交：在 fast-path 之后另一并发请求已成功插入；回查首次结果
                let existing = self.lookup_existing(sender_id, receiver_id, &msg_id).await?;
                let (existing_id, existing_total, sender_balance, receiver_charm) = existing.ok_or_else(|| {
                    SendGiftError::Internal(
                        "ON CONFLICT triggered but no row found on re-query".into(),
                    )
                })?;
                tracing::info!(
                    sender_id = %sender_id,
                    msg_id = %msg_id,
                    gift_record_id = %existing_id,
                    "SendGift idempotent (race resolved): returning cached result"
                );
                return Ok(SendGiftResult {
                    gift_record_id: existing_id,
                    total_price: existing_total,
                    sender_new_balance: sender_balance,
                    receiver_new_charm: receiver_charm,
                });
            }
        };

        // ── 7. 更新 Redis 榜单（非阻断）──────────────────────────────────────
        match self.redis_client.get_multiplexed_async_connection().await {
            Ok(mut conn) => {
                ranking::update_rankings(&mut conn, receiver_id, sender_id, total).await;
            }
            Err(e) => {
                tracing::warn!(
                    "GiftSendService: redis connect failed, rankings not updated: {}",
                    e
                );
            }
        }

        // ── 8. 广播 GiftReceived（异步，不阻塞 HTTP 响应）───────────────────
        let gift_envelope = build_gift_received_msg(
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
        
        let registry_clone = Arc::clone(&self.registry);
        let room_manager_clone = Arc::clone(&self.room_manager);
        tokio::spawn(async move {
            if let Some(rs) = room_manager_clone.get_room(room_id) {
                crate::ws::broadcaster::broadcast_to_room(&registry_clone, &rs, gift_envelope);
            } else {
                crate::ws::broadcaster::broadcast_to_room_no_state(
                    &registry_clone,
                    room_id,
                    gift_envelope,
                );
            }
        });

        Ok(SendGiftResult {
            gift_record_id,
            total_price: total,
            sender_new_balance,
            receiver_new_charm,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // SGS-01: GiftSendService::new 在无效 redis_url 上 fail-fast（缺陷 #4）
    #[tokio::test]
    async fn sgs01_new_returns_err_for_invalid_redis_url() {
        // 注：构造 PgPool 不会真正连接（lazy）；这里使用 connect_lazy 占位
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://invalid:invalid@127.0.0.1:5432/x")
            .expect("lazy pool");
        let registry = Arc::new(ConnectionRegistry::new());
        let room_manager = Arc::new(RoomManager::new());
        let (tx, _rx) = mpsc::channel::<BalanceEvent>(8);

        let result = GiftSendService::new(
            pool,
            registry,
            room_manager,
            tx,
            "not-a-valid-url".to_string(),
        );
        assert!(
            result.is_err(),
            "SGS-01: invalid redis URL should return Err, no fallback (缺陷 #4)"
        );
        let msg = format!("{:?}", result.err().unwrap());
        assert!(
            msg.contains("REDIS_URL invalid"),
            "SGS-01: error should mention REDIS_URL, got: {msg}"
        );
    }

    // SGS-02: GiftSendService::new 在合法 redis_url 上返回 Ok
    #[tokio::test]
    async fn sgs02_new_ok_for_valid_redis_url() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://invalid:invalid@127.0.0.1:5432/x")
            .expect("lazy pool");
        let registry = Arc::new(ConnectionRegistry::new());
        let room_manager = Arc::new(RoomManager::new());
        let (tx, _rx) = mpsc::channel::<BalanceEvent>(8);

        let result = GiftSendService::new(
            pool,
            registry,
            room_manager,
            tx,
            "redis://127.0.0.1:6379".to_string(),
        );
        assert!(result.is_ok(), "SGS-02: valid redis URL should return Ok");
    }
}
