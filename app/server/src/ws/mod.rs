//! WebSocket 模块 — 连接管理、心跳检测、信令处理
//!
//! 子模块：
//! - `registry`     — ConnectionRegistry (DashMap)
//! - `heartbeat`    — 心跳检测后台 task
//! - `connection`   — 单连接生命周期与信令处理
//! - `handler`      — Axum WS 升级处理器 (JWT 鉴权)
//! - `schema_guard` — T-00103 出栈 envelope schema 校验（dev/test panic，prod no-op）

pub mod broadcaster;
pub mod connection;
pub mod handler;
pub mod heartbeat;
pub mod registry;
pub mod schema_guard;

pub use handler::ws_handler;
pub use registry::{ConnectionHandle, ConnectionRegistry};
