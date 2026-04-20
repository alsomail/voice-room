//! room 模块 — WS 房间运行时状态管理
//!
//! 区别于 `modules::room`（HTTP API 业务层），本模块负责
//! WebSocket 信令处理和内存状态维护。
//!
//! - `state`：RoomState / MemberInfo
//! - `manager`：RoomManager（DashMap<Uuid, Arc<RoomState>>）
//! - `handler`：handle_join_room
//! - `filter`：敏感词过滤

pub mod filter;
pub mod handler;
pub mod manager;
pub mod state;

pub use manager::RoomManager;
pub use state::{MemberInfo, RoomState};
