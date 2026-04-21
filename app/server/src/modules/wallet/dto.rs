//! 钱包模块 DTO 定义
//!
//! - `BalanceResponse`    — GET /wallet/balance 响应数据
//! - `TransactionQuery`   — GET /wallet/transactions 查询参数
//! - `TransactionItem`    — 单条流水响应条目
//! - `Paginated<T>`       — 通用分页响应容器

use serde::{Deserialize, Serialize};
use voice_room_shared::models::wallet::{WalletTransactionModel, WalletTxnType};

// ─── 余额响应 ─────────────────────────────────────────────────────────────────

/// GET /api/v1/wallet/balance 的 data 字段
#[derive(Debug, Serialize)]
pub struct BalanceResponse {
    pub diamond_balance: i64,
}

// ─── 通用分页容器 ──────────────────────────────────────────────────────────────

/// 分页响应容器，用于 list_txns 的 HTTP 响应
#[derive(Debug, Serialize)]
pub struct Paginated<T: Serialize> {
    pub total: u64,
    pub page: u32,
    pub size: u32,
    pub items: Vec<T>,
}

// ─── 流水查询参数 ──────────────────────────────────────────────────────────────

/// GET /api/v1/wallet/transactions 查询参数
#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    /// 页码，默认 1，最小 1
    #[serde(default = "default_page")]
    pub page: u32,
    /// 每页大小，默认 20，最大 100
    #[serde(default = "default_size")]
    pub size: u32,
    /// 可选类型过滤（snake_case: gift_send / admin_adjust / ...）
    #[serde(rename = "type")]
    pub txn_type: Option<WalletTxnType>,
}

fn default_page() -> u32 {
    1
}
fn default_size() -> u32 {
    20
}

// ─── 流水响应条目 ──────────────────────────────────────────────────────────────

/// 单条流水记录（HTTP 响应用），字段对齐 TDS T-00018 §HTTP 接口定义
#[derive(Debug, Serialize)]
pub struct TransactionItem {
    pub id: String,
    /// 类型（snake_case）
    #[serde(rename = "type")]
    pub txn_type: WalletTxnType,
    pub amount: i64,
    pub balance_after: i64,
    pub ref_id: Option<String>,
    pub reason: Option<String>,
    /// RFC3339 时间字符串
    pub created_at: String,
}

impl From<WalletTransactionModel> for TransactionItem {
    fn from(m: WalletTransactionModel) -> Self {
        Self {
            id: m.id.to_string(),
            txn_type: m.txn_type,
            amount: m.amount,
            balance_after: m.balance_after,
            ref_id: m.ref_id.map(|u| u.to_string()),
            reason: m.reason,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use voice_room_shared::models::wallet::WalletTxnType;

    // D01: BalanceResponse 序列化包含 diamond_balance 字段
    #[test]
    fn d01_balance_response_serializes() {
        let resp = BalanceResponse {
            diamond_balance: 1234,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["diamond_balance"], 1234);
    }

    // D02: TransactionQuery 默认值 page=1, size=20
    #[test]
    fn d02_transaction_query_defaults() {
        let q = TransactionQuery {
            page: default_page(),
            size: default_size(),
            txn_type: None,
        };
        assert_eq!(q.page, 1);
        assert_eq!(q.size, 20);
        assert!(q.txn_type.is_none());
    }

    // D03: TransactionItem::from 正确转换 WalletTransactionModel
    #[test]
    fn d03_transaction_item_from_model() {
        let model = WalletTransactionModel {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            txn_type: WalletTxnType::GiftSend,
            amount: -100,
            balance_after: 900,
            ref_id: None,
            reason: Some("送礼".to_string()),
            operator_id: None,
            created_at: Utc::now(),
        };
        let item = TransactionItem::from(model);
        assert_eq!(item.amount, -100);
        assert_eq!(item.balance_after, 900);
        assert_eq!(item.reason.as_deref(), Some("送礼"));
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["type"], "gift_send");
    }

    // D04: Paginated 序列化包含 total/page/size/items
    #[test]
    fn d04_paginated_serializes() {
        let p: Paginated<BalanceResponse> = Paginated {
            total: 42,
            page: 2,
            size: 20,
            items: vec![BalanceResponse { diamond_balance: 0 }],
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["total"], 42);
        assert_eq!(json["page"], 2);
        assert_eq!(json["size"], 20);
        assert_eq!(json["items"].as_array().unwrap().len(), 1);
    }
}
