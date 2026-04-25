//! SendGift 共享类型 — DTO、错误码、port trait

use async_trait::async_trait;
use uuid::Uuid;

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

/// 送礼服务抽象接口（供 WS handler 注入，支持 Fake 替身）
#[async_trait]
pub trait SendGiftServicePort: Send + Sync {
    /// 执行送礼核心逻辑：事务 + 广播 + 榜单
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
