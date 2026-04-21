//! Gift 响应 DTO
//!
//! - `GiftItem`     — 列表项（包含语言选择后的 name）
//! - `GiftListData` — 整体响应 data 结构（含 items + version）

use serde::Serialize;
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
