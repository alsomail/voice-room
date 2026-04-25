//! 跨服务事件定义（Admin Server → App Server via Redis PubSub）
//!
//! 此模块作为 Admin 与 App 两端共享的"单一事实源"，编译期锁定字段名，
//! 杜绝 `new_balance` ↔ `balance_after` 之类的契约破坏（缺陷 #1 P0）。
//!
//! 协议参考：`doc/protocol/transaction_and_gift.md` §balance_updated

pub mod balance;

pub use balance::BalanceUpdatedEvent;
