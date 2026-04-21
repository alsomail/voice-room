//! Gift 礼物配置模块
//!
//! 提供礼物配置列表 API（GET /api/v1/gifts/list）：
//! - `repo`     — GiftRepoPort trait + PgGiftRepo + FakeGiftRepo（测试替身）
//! - `service`  — GiftServicePort trait + GiftService（in-memory 缓存）+ FakeGiftService
//! - `handler`  — HTTP handler，解析 Accept-Language header
//! - `routes`   — 路由注册
//! - `dto`      — 响应 DTO（GiftListData、GiftItem）

pub mod dto;
pub mod handler;
pub mod repo;
pub mod routes;
pub mod service;

pub use routes::gift_routes;
