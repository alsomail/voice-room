//! 钱包模块
//!
//! 提供余额查询、流水分页和余额广播功能。
//! - `service`     — WalletService（余额查询、流水列表、apply_delta）
//! - `broadcaster` — BalanceBroadcaster（WS 实时推送）
//! - `controller`  — HTTP handlers
//! - `routes`      — 路由注册
//! - `dto`         — 请求/响应 DTO

pub mod broadcaster;
pub mod controller;
pub mod dto;
pub mod routes;
pub mod service;

pub use routes::wallet_routes;
