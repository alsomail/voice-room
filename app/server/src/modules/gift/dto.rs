//! Gift 响应 DTO
//!
//! - `GiftItem`     — 列表项（包含语言选择后的 name）
//! - `GiftListData` — 整体响应 data 结构（含 items + version）
//! - `SendGiftRequest` — POST /api/v1/gifts/send 请求体（T-00044）
//! - `SendGiftResponse` — POST /api/v1/gifts/send 响应体（T-00044）

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 单个礼物的响应项
///
/// `name` 字段根据请求的 `Accept-Language` header 选择 `name_en` 或 `name_ar`。
#[derive(Debug, Clone, Serialize)]
pub struct GiftItem {
    pub id: Uuid,
    pub code: String,
    /// 多语言选择后的展示名称
    pub name: String,
    pub icon_url: String,
    pub price: i64,
    pub tier: i16,
    pub effect_level: i16,
    pub animation_url: Option<String>,
    pub sort_order: i32,
}

/// 礼物列表响应 data 结构
#[derive(Debug, Clone, Serialize)]
pub struct GiftListData {
    pub items: Vec<GiftItem>,
    /// 缓存版本标记（Unix 时间戳字符串），供客户端做增量刷新
    pub version: String,
}

/// POST /api/v1/gifts/send 请求体（T-00044）
///
/// `#[serde(deny_unknown_fields)]` — 拒绝含未知字段的请求体，防止字段注入。
/// PROTO-BINDING: doc/protocol/HTTP POST /api/v1/gifts/send
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendGiftRequest {
    pub room_id: Uuid,
    pub gift_id: Uuid,
    pub receiver_id: Uuid,
    pub count: i32,
}

/// POST /api/v1/gifts/send 响应体（T-00044）
#[derive(Debug, Serialize)]
pub struct SendGiftResponse {
    pub gift_record_id: Uuid,
    pub sender_balance: i64,
    pub receiver_charm: i64,
}
