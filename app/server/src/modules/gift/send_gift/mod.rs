//! SendGift 核心业务逻辑（按职责拆分子模块，缺陷 #5）
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
//!             e) INSERT gift_records ON CONFLICT (sender_id,msg_id) DO NOTHING RETURNING id
//!                若 RETURNING 为空 → DuplicateMsgId（事务自动回滚）
//!             f) INSERT wallet_transactions
//!            COMMIT
//!         7. Redis ZINCRBY 四个 ZSet（魅力/财富 日榜/周榜）
//!         8. 广播 GiftReceived 给 registry.get_connections_in_room(room_id)
//!         9. 通知 BalanceBroadcaster → 发送者 BalanceUpdated
//! ```
//!
//! ## 子模块
//! - [`service`]   — `GiftSendService` 真实实现（事务/广播/榜单/批量用户查询）
//! - [`handler`]   — `SendGiftDeps` + `handle_send_gift` WS 信令处理
//! - [`messages`]  — JSON envelope 构造（GiftReceived / SendGiftResult）
//! - [`fake`]      — 测试替身 `FakeSendGiftService`
//! - [`types`]     — 共享 DTO / 错误类型 / `SendGiftServicePort` trait

pub mod fake;
pub mod handler;
pub mod messages;
pub mod service;
pub mod types;

// 公共 re-export，保持既有 import 路径不变（缺陷 #5 拆分约束）
pub use fake::FakeSendGiftService;
pub use handler::{handle_send_gift, SendGiftDeps};
pub use service::GiftSendService;
pub use types::{SendGiftError, SendGiftPayload, SendGiftResult, SendGiftServicePort};
