//! Payment 共享类型
//!
//! - `OrderState` — 订单状态枚举（payment_order_state 数据库枚举映射）
//! - `Provider` — 支付渠道枚举（payment_provider 数据库枚举映射）
//!
//! 参见 doc/protocol/payment_api.md §9.2.3

use serde::{Deserialize, Serialize};

/// 支付渠道枚举（对应 DB payment_provider）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "payment_provider", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Google Play 正式渠道
    GooglePlay,
    /// Apple IAP（预留）
    AppleIap,
    /// Dev/Staging Mock 通道
    Mock,
}

/// 订单状态枚举（对应 DB payment_order_state）
///
/// 状态机参见 payment_api.md §9.2.3。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "payment_order_state", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderState {
    /// 服务端预创建，等待客户端发起支付
    Pending,
    /// 收到 purchaseToken，待 Google 验签
    Verifying,
    /// Google 验签通过
    Verified,
    /// 钻石入账事务完成
    Credited,
    /// Google 已 acknowledge
    Acked,
    /// 用户取消
    Cancelled,
    /// 验签失败 / 风控拦截 / token 重放
    Failed,
    /// RTDN 退款处理完成
    Refunded,
    /// Google 返回 purchaseState=2（慢速支付场景）
    PendingGoogle,
}

impl std::fmt::Display for OrderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OrderState::Pending => "PENDING",
            OrderState::Verifying => "VERIFYING",
            OrderState::Verified => "VERIFIED",
            OrderState::Credited => "CREDITED",
            OrderState::Acked => "ACKED",
            OrderState::Cancelled => "CANCELLED",
            OrderState::Failed => "FAILED",
            OrderState::Refunded => "REFUNDED",
            OrderState::PendingGoogle => "PENDING_GOOGLE",
        };
        write!(f, "{s}")
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Provider::GooglePlay => "google_play",
            Provider::AppleIap => "apple_iap",
            Provider::Mock => "mock",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // P01: OrderState 所有变体可构造
    #[test]
    fn p01_order_state_all_variants_constructible() {
        let states = vec![
            OrderState::Pending,
            OrderState::Verifying,
            OrderState::Verified,
            OrderState::Credited,
            OrderState::Acked,
            OrderState::Cancelled,
            OrderState::Failed,
            OrderState::Refunded,
            OrderState::PendingGoogle,
        ];
        assert_eq!(states.len(), 9);
    }

    // P02: OrderState 序列化为 SCREAMING_SNAKE_CASE
    #[test]
    fn p02_order_state_serializes_to_screaming_snake_case() {
        assert_eq!(
            serde_json::to_string(&OrderState::Pending).unwrap(),
            r#""PENDING""#
        );
        assert_eq!(
            serde_json::to_string(&OrderState::PendingGoogle).unwrap(),
            r#""PENDING_GOOGLE""#
        );
        assert_eq!(
            serde_json::to_string(&OrderState::Acked).unwrap(),
            r#""ACKED""#
        );
    }

    // P03: Provider 序列化为 snake_case
    #[test]
    fn p03_provider_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&Provider::GooglePlay).unwrap(),
            r#""google_play""#
        );
        assert_eq!(
            serde_json::to_string(&Provider::Mock).unwrap(),
            r#""mock""#
        );
    }

    // P04: OrderState Display
    #[test]
    fn p04_order_state_display() {
        assert_eq!(OrderState::Pending.to_string(), "PENDING");
        assert_eq!(OrderState::PendingGoogle.to_string(), "PENDING_GOOGLE");
        assert_eq!(OrderState::Acked.to_string(), "ACKED");
    }

    // P05: Provider Display
    #[test]
    fn p05_provider_display() {
        assert_eq!(Provider::GooglePlay.to_string(), "google_play");
        assert_eq!(Provider::Mock.to_string(), "mock");
    }

    // P06: OrderState 未知变体反序列化应报错
    #[test]
    fn p06_order_state_unknown_variant_returns_err() {
        let result: Result<OrderState, _> = serde_json::from_str(r#""UNKNOWN""#);
        assert!(result.is_err());
    }

    // P07: Provider Clone + PartialEq
    #[test]
    fn p07_provider_clone_eq() {
        let a = Provider::GooglePlay;
        let b = a.clone();
        assert_eq!(a, b);
    }
}
