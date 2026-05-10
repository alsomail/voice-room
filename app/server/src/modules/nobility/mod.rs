//! 贵族体系模块（E-09 / T-00065 ~ T-00070）
//!
//! 模块结构：
//! - `dto`        — 请求/响应 DTO
//! - `service`    — NobilityServicePort trait + FakeNobilityService
//! - `controller` — HTTP handlers
//! - `routes`     — 路由注册
//! - `purchase`   — 购买决策纯逻辑
//! - `cron`       — 续费/过期 cron 逻辑
//! - `privileges` — 特权钩子纯函数

pub mod controller;
pub mod cron;
pub mod dto;
pub mod global_broadcast;
pub mod privileges;
pub mod purchase;
pub mod routes;
pub mod service;

pub use global_broadcast::FakeGlobalBroadcast;
pub use global_broadcast::GlobalBroadcastPort;
pub use routes::nobility_routes;
pub use service::DukeNobilityService;
pub use service::FakeNobilityService;
pub use service::NobilityServicePort;
