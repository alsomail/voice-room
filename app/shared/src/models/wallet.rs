use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 钱包流水类型枚举
///
/// 对应 `wallet_transactions.type` 列（VARCHAR(32)），以 snake_case 字符串持久化。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WalletTxnType {
    /// 送礼扣减
    GiftSend,
    /// 收礼入账（MVP：接收者暂不直接加钻石，保留字段备用）
    GiftReceive,
    /// Admin 手动调整
    AdminAdjust,
    /// 充值（E-08 接入）
    Recharge,
    /// 退款
    Refund,
}

/// 钱包流水表模型
///
/// 对应数据库表 `wallet_transactions`，由迁移 `004_create_wallet.sql` 创建。
/// - `amount` 正数表示加款，负数表示扣款
/// - `balance_after >= 0` 由 DB CHECK 约束保证
#[derive(Debug, FromRow)]
pub struct WalletTransactionModel {
    pub id: Uuid,
    pub user_id: Uuid,
    #[sqlx(rename = "type")]
    pub txn_type: WalletTxnType,
    pub amount: i64,
    pub balance_after: i64,
    pub ref_id: Option<Uuid>,
    pub reason: Option<String>,
    pub operator_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    // ─────────────────────────────────────────────
    // WU01: WalletTxnType 所有变体均可构造
    // ─────────────────────────────────────────────
    #[test]
    fn wu01_wallet_txn_type_all_variants_constructible() {
        let variants = vec![
            WalletTxnType::GiftSend,
            WalletTxnType::GiftReceive,
            WalletTxnType::AdminAdjust,
            WalletTxnType::Recharge,
            WalletTxnType::Refund,
        ];
        assert_eq!(variants.len(), 5);
    }

    // ─────────────────────────────────────────────
    // WU02: WalletTxnType 序列化为 snake_case 字符串
    // ─────────────────────────────────────────────
    #[test]
    fn wu02_wallet_txn_type_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&WalletTxnType::GiftSend).unwrap(),
            r#""gift_send""#
        );
        assert_eq!(
            serde_json::to_string(&WalletTxnType::GiftReceive).unwrap(),
            r#""gift_receive""#
        );
        assert_eq!(
            serde_json::to_string(&WalletTxnType::AdminAdjust).unwrap(),
            r#""admin_adjust""#
        );
        assert_eq!(
            serde_json::to_string(&WalletTxnType::Recharge).unwrap(),
            r#""recharge""#
        );
        assert_eq!(
            serde_json::to_string(&WalletTxnType::Refund).unwrap(),
            r#""refund""#
        );
    }

    // ─────────────────────────────────────────────
    // WU03: WalletTxnType 从 snake_case 字符串反序列化
    // ─────────────────────────────────────────────
    #[test]
    fn wu03_wallet_txn_type_deserializes_from_snake_case() {
        let t: WalletTxnType = serde_json::from_str(r#""gift_send""#).unwrap();
        assert_eq!(t, WalletTxnType::GiftSend);

        let t: WalletTxnType = serde_json::from_str(r#""gift_receive""#).unwrap();
        assert_eq!(t, WalletTxnType::GiftReceive);

        let t: WalletTxnType = serde_json::from_str(r#""admin_adjust""#).unwrap();
        assert_eq!(t, WalletTxnType::AdminAdjust);

        let t: WalletTxnType = serde_json::from_str(r#""recharge""#).unwrap();
        assert_eq!(t, WalletTxnType::Recharge);

        let t: WalletTxnType = serde_json::from_str(r#""refund""#).unwrap();
        assert_eq!(t, WalletTxnType::Refund);
    }

    // ─────────────────────────────────────────────
    // WU04: WalletTxnType 未知字符串反序列化应报错
    // ─────────────────────────────────────────────
    #[test]
    fn wu04_wallet_txn_type_unknown_variant_returns_err() {
        let result: Result<WalletTxnType, _> = serde_json::from_str(r#""unknown_type""#);
        assert!(
            result.is_err(),
            "Deserializing unknown WalletTxnType should fail"
        );
    }

    // ─────────────────────────────────────────────
    // WU05: WalletTxnType 实现 Clone
    // ─────────────────────────────────────────────
    #[test]
    fn wu05_wallet_txn_type_clone() {
        let original = WalletTxnType::AdminAdjust;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    // ─────────────────────────────────────────────
    // WU06: WalletTxnType 实现 Debug
    // ─────────────────────────────────────────────
    #[test]
    fn wu06_wallet_txn_type_debug_format() {
        let debug_str = format!("{:?}", WalletTxnType::GiftSend);
        assert!(debug_str.contains("GiftSend"));
    }

    // ─────────────────────────────────────────────
    // WU07: WalletTransactionModel 所有必填字段可构造
    // ─────────────────────────────────────────────
    #[test]
    fn wu07_wallet_transaction_model_constructible() {
        let model = WalletTransactionModel {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            txn_type: WalletTxnType::Recharge,
            amount: 100,
            balance_after: 100,
            ref_id: None,
            reason: None,
            operator_id: None,
            created_at: Utc::now(),
        };
        assert_eq!(model.amount, 100);
        assert_eq!(model.balance_after, 100);
        assert!(model.ref_id.is_none());
        assert!(model.reason.is_none());
        assert!(model.operator_id.is_none());
    }

    // ─────────────────────────────────────────────
    // WU08: WalletTransactionModel 可选字段均为 None 时构造正常
    // ─────────────────────────────────────────────
    #[test]
    fn wu08_wallet_transaction_model_optional_fields_none() {
        let model = WalletTransactionModel {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            txn_type: WalletTxnType::GiftSend,
            amount: -50,
            balance_after: 50,
            ref_id: None,
            reason: None,
            operator_id: None,
            created_at: Utc::now(),
        };
        assert!(model.ref_id.is_none());
        assert!(model.reason.is_none());
        assert!(model.operator_id.is_none());
    }

    // ─────────────────────────────────────────────
    // WU09: WalletTransactionModel 可选字段均有值时构造正常
    // ─────────────────────────────────────────────
    #[test]
    fn wu09_wallet_transaction_model_optional_fields_some() {
        let ref_id = Uuid::new_v4();
        let operator_id = Uuid::new_v4();
        let model = WalletTransactionModel {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            txn_type: WalletTxnType::AdminAdjust,
            amount: 1000,
            balance_after: 1000,
            ref_id: Some(ref_id),
            reason: Some("管理员补发".to_string()),
            operator_id: Some(operator_id),
            created_at: Utc::now(),
        };
        assert_eq!(model.ref_id, Some(ref_id));
        assert_eq!(model.reason.as_deref(), Some("管理员补发"));
        assert_eq!(model.operator_id, Some(operator_id));
    }

    // ─────────────────────────────────────────────
    // WU10: WalletTransactionModel 实现 Debug
    // ─────────────────────────────────────────────
    #[test]
    fn wu10_wallet_transaction_model_debug_format() {
        let model = WalletTransactionModel {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            txn_type: WalletTxnType::Refund,
            amount: -10,
            balance_after: 90,
            ref_id: None,
            reason: Some("退款测试".to_string()),
            operator_id: None,
            created_at: Utc::now(),
        };
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("WalletTransactionModel"));
    }

    // ─────────────────────────────────────────────
    // WU11: amount 支持负数（扣款场景）
    // ─────────────────────────────────────────────
    #[test]
    fn wu11_wallet_transaction_model_amount_can_be_negative() {
        let model = WalletTransactionModel {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            txn_type: WalletTxnType::GiftSend,
            amount: -100,
            balance_after: 0,
            ref_id: None,
            reason: None,
            operator_id: None,
            created_at: Utc::now(),
        };
        assert!(model.amount < 0);
        assert_eq!(model.balance_after, 0);
    }

    // ─────────────────────────────────────────────
    // WU12: reason 支持 Unicode / Emoji
    // ─────────────────────────────────────────────
    #[test]
    fn wu12_wallet_transaction_model_reason_supports_unicode() {
        let model = WalletTransactionModel {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            txn_type: WalletTxnType::Recharge,
            amount: 100,
            balance_after: 100,
            ref_id: None,
            reason: Some("充值🎁 — 礼包活动".to_string()),
            operator_id: None,
            created_at: Utc::now(),
        };
        let reason = model.reason.as_deref().unwrap();
        assert!(reason.contains('🎁'));
        assert!(reason.contains('礼'));
    }
}
