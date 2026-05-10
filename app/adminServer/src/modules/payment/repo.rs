//! T-10025/26: PaymentOrderRepo — 订单只读 + 写操作仓库
//!
//! PaymentOrderRepo: 订单列表/详情（只读）
//! PaymentAdminRepo: 补单/退款原子事务（写）

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;
use voice_room_shared::payment::{OrderState, Provider};

use crate::common::error::AppError;

use super::dto::{mask_purchase_token, AdminOrderDetail, AdminOrderListItem};

// ─── 数据行 ───────────────────────────────────────────────────────────────────

/// 从数据库读取的完整 payment_orders 行（含 JOIN payment_skus 字段）。
#[derive(Debug, Clone)]
pub struct PaymentOrderRow {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub sku_id: String,
    pub provider: Provider,
    pub purchase_token: Option<String>,
    pub provider_order_id: Option<String>,
    pub amount_micros: Option<i64>,
    pub currency: Option<String>,
    pub country_code: Option<String>,
    pub state: OrderState,
    pub state_history: Value,
    pub provider_response_raw: Option<Value>,
    pub risk_flags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub credited_at: Option<DateTime<Utc>>,
    pub acked_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub failed_reason: Option<String>,
    // JOIN from payment_skus
    pub diamonds: i64,
}

impl PaymentOrderRow {
    /// 转换为列表项 DTO（脱敏 token）
    pub fn to_list_item(&self) -> AdminOrderListItem {
        AdminOrderListItem {
            order_id: self.order_id,
            user_id: self.user_id,
            sku_id: self.sku_id.clone(),
            provider: self.provider.to_string(),
            amount_micros: self.amount_micros,
            currency: self.currency.clone(),
            country_code: self.country_code.clone(),
            state: self.state.to_string(),
            purchase_token_masked: self
                .purchase_token
                .as_deref()
                .and_then(mask_purchase_token),
            provider_order_id: self.provider_order_id.clone(),
            created_at: self.created_at,
            credited_at: self.credited_at,
            acked_at: self.acked_at,
            failed_at: self.failed_at,
        }
    }

    /// 转换为详情 DTO
    pub fn to_detail(&self) -> AdminOrderDetail {
        AdminOrderDetail {
            order_id: self.order_id,
            user_id: self.user_id,
            sku_id: self.sku_id.clone(),
            provider: self.provider.to_string(),
            amount_micros: self.amount_micros,
            currency: self.currency.clone(),
            country_code: self.country_code.clone(),
            state: self.state.to_string(),
            state_history: self.state_history.clone(),
            provider_response_raw: self.provider_response_raw.clone(),
            purchase_token_masked: self
                .purchase_token
                .as_deref()
                .and_then(mask_purchase_token),
            provider_order_id: self.provider_order_id.clone(),
            risk_flags: self.risk_flags.clone(),
            created_at: self.created_at,
            verified_at: self.verified_at,
            credited_at: self.credited_at,
            acked_at: self.acked_at,
            failed_at: self.failed_at,
            failed_reason: self.failed_reason.clone(),
        }
    }
}

// ─── 过滤器 ───────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct OrderFilter {
    pub user_id: Option<Uuid>,
    pub state: Option<String>,
    pub provider: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub amount_min: Option<i64>,
    pub amount_max: Option<i64>,
    /// 0-based offset
    pub offset: i64,
    pub limit: i64,
}

// ─── Trait ────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait PaymentOrderRepo: Send + Sync {
    /// 分页查询订单列表，返回 (total, rows)
    async fn list_orders(
        &self,
        filter: OrderFilter,
    ) -> Result<(i64, Vec<PaymentOrderRow>), AppError>;

    /// 按 order_id 查询单条订单（含 state_history + provider_response_raw）
    async fn find_by_id(&self, order_id: Uuid) -> Result<Option<PaymentOrderRow>, AppError>;
}

// ─── PostgreSQL 生产实现 ──────────────────────────────────────────────────────

pub struct PgPaymentOrderRepo {
    pool: PgPool,
}

impl PgPaymentOrderRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// SQLx 的 payment_orders 行结构（FromRow）
#[derive(Debug, sqlx::FromRow)]
struct OrderDbRow {
    pub order_id: Uuid,
    pub user_id: Uuid,
    pub sku_id: String,
    pub provider: Provider,
    pub purchase_token: Option<String>,
    pub provider_order_id: Option<String>,
    pub amount_micros: Option<i64>,
    pub currency: Option<String>,
    pub country_code: Option<String>,
    pub state: OrderState,
    pub state_history: Value,
    pub provider_response_raw: Option<Value>,
    pub risk_flags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub credited_at: Option<DateTime<Utc>>,
    pub acked_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub failed_reason: Option<String>,
    pub diamonds: i64,
}

impl From<OrderDbRow> for PaymentOrderRow {
    fn from(r: OrderDbRow) -> Self {
        Self {
            order_id: r.order_id,
            user_id: r.user_id,
            sku_id: r.sku_id,
            provider: r.provider,
            purchase_token: r.purchase_token,
            provider_order_id: r.provider_order_id,
            amount_micros: r.amount_micros,
            currency: r.currency,
            country_code: r.country_code,
            state: r.state,
            state_history: r.state_history,
            provider_response_raw: r.provider_response_raw,
            risk_flags: r.risk_flags,
            created_at: r.created_at,
            verified_at: r.verified_at,
            credited_at: r.credited_at,
            acked_at: r.acked_at,
            failed_at: r.failed_at,
            failed_reason: r.failed_reason,
            diamonds: r.diamonds,
        }
    }
}

#[async_trait]
impl PaymentOrderRepo for PgPaymentOrderRepo {
    async fn list_orders(
        &self,
        f: OrderFilter,
    ) -> Result<(i64, Vec<PaymentOrderRow>), AppError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM payment_orders o \
             JOIN payment_skus s ON o.sku_id = s.sku_id \
             WHERE ($1::uuid IS NULL OR o.user_id = $1) \
               AND ($2::text IS NULL OR o.state::text = $2) \
               AND ($3::text IS NULL OR o.provider::text = $3) \
               AND ($4::timestamptz IS NULL OR o.created_at >= $4) \
               AND ($5::timestamptz IS NULL OR o.created_at <= $5) \
               AND ($6::bigint IS NULL OR o.amount_micros >= $6) \
               AND ($7::bigint IS NULL OR o.amount_micros <= $7)",
        )
        .bind(f.user_id)
        .bind(f.state.as_deref())
        .bind(f.provider.as_deref())
        .bind(f.created_from)
        .bind(f.created_to)
        .bind(f.amount_min)
        .bind(f.amount_max)
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query_as::<_, OrderDbRow>(
            "SELECT o.order_id, o.user_id, o.sku_id, o.provider, \
                    o.purchase_token, o.provider_order_id, \
                    o.amount_micros, o.currency, o.country_code, \
                    o.state, o.state_history, o.provider_response_raw, \
                    o.risk_flags, o.created_at, o.verified_at, \
                    o.credited_at, o.acked_at, o.failed_at, o.failed_reason, \
                    s.diamonds \
             FROM payment_orders o \
             JOIN payment_skus s ON o.sku_id = s.sku_id \
             WHERE ($1::uuid IS NULL OR o.user_id = $1) \
               AND ($2::text IS NULL OR o.state::text = $2) \
               AND ($3::text IS NULL OR o.provider::text = $3) \
               AND ($4::timestamptz IS NULL OR o.created_at >= $4) \
               AND ($5::timestamptz IS NULL OR o.created_at <= $5) \
               AND ($6::bigint IS NULL OR o.amount_micros >= $6) \
               AND ($7::bigint IS NULL OR o.amount_micros <= $7) \
             ORDER BY o.created_at DESC \
             LIMIT $8 OFFSET $9",
        )
        .bind(f.user_id)
        .bind(f.state.as_deref())
        .bind(f.provider.as_deref())
        .bind(f.created_from)
        .bind(f.created_to)
        .bind(f.amount_min)
        .bind(f.amount_max)
        .bind(f.limit)
        .bind(f.offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((count.0, rows.into_iter().map(Into::into).collect()))
    }

    async fn find_by_id(&self, order_id: Uuid) -> Result<Option<PaymentOrderRow>, AppError> {
        let row = sqlx::query_as::<_, OrderDbRow>(
            "SELECT o.order_id, o.user_id, o.sku_id, o.provider, \
                    o.purchase_token, o.provider_order_id, \
                    o.amount_micros, o.currency, o.country_code, \
                    o.state, o.state_history, o.provider_response_raw, \
                    o.risk_flags, o.created_at, o.verified_at, \
                    o.credited_at, o.acked_at, o.failed_at, o.failed_reason, \
                    s.diamonds \
             FROM payment_orders o \
             JOIN payment_skus s ON o.sku_id = s.sku_id \
             WHERE o.order_id = $1",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }
}

// ─── Fake 实现（内存，用于单元/集成测试）──────────────────────────────────────

#[cfg(any(test, feature = "test-utils"))]
#[derive(Default)]
pub struct FakePaymentOrderRepo {
    orders: Arc<Mutex<Vec<PaymentOrderRow>>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl FakePaymentOrderRepo {
    pub fn seed(&self, row: PaymentOrderRow) {
        self.orders.lock().unwrap().push(row);
    }

    pub fn get_orders(&self) -> Vec<PaymentOrderRow> {
        self.orders.lock().unwrap().clone()
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl PaymentOrderRepo for FakePaymentOrderRepo {
    async fn list_orders(
        &self,
        f: OrderFilter,
    ) -> Result<(i64, Vec<PaymentOrderRow>), AppError> {
        let guard = self.orders.lock().unwrap();
        let filtered: Vec<_> = guard
            .iter()
            .filter(|o| {
                if let Some(uid) = f.user_id {
                    if o.user_id != uid {
                        return false;
                    }
                }
                if let Some(ref st) = f.state {
                    if &o.state.to_string() != st {
                        return false;
                    }
                }
                if let Some(ref prov) = f.provider {
                    if &o.provider.to_string() != prov {
                        return false;
                    }
                }
                if let Some(from) = f.created_from {
                    if o.created_at < from {
                        return false;
                    }
                }
                if let Some(to) = f.created_to {
                    if o.created_at > to {
                        return false;
                    }
                }
                if let Some(min) = f.amount_min {
                    if o.amount_micros.unwrap_or(0) < min {
                        return false;
                    }
                }
                if let Some(max) = f.amount_max {
                    if o.amount_micros.unwrap_or(0) > max {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total = filtered.len() as i64;
        let offset = f.offset as usize;
        let limit = f.limit as usize;
        let page_items = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok((total, page_items))
    }

    async fn find_by_id(&self, order_id: Uuid) -> Result<Option<PaymentOrderRow>, AppError> {
        let guard = self.orders.lock().unwrap();
        Ok(guard.iter().find(|o| o.order_id == order_id).cloned())
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use voice_room_shared::payment::OrderState;

    fn make_order(state: OrderState, user_id: Uuid) -> PaymentOrderRow {
        PaymentOrderRow {
            order_id: Uuid::new_v4(),
            user_id,
            sku_id: "diamond_600".to_string(),
            provider: Provider::GooglePlay,
            purchase_token: Some("oojkl1234567ABCD".to_string()),
            provider_order_id: Some("GPA.0001-xxxx".to_string()),
            amount_micros: Some(9_990_000),
            currency: Some("USD".to_string()),
            country_code: Some("US".to_string()),
            state,
            state_history: serde_json::json!([]),
            provider_response_raw: None,
            risk_flags: vec![],
            created_at: Utc::now(),
            verified_at: None,
            credited_at: None,
            acked_at: None,
            failed_at: None,
            failed_reason: None,
            diamonds: 600,
        }
    }

    // ── R-01: list 返回全部 ────────────────────────────────────────────────

    #[tokio::test]
    async fn r01_list_returns_all_orders() {
        let repo = FakePaymentOrderRepo::default();
        repo.seed(make_order(OrderState::Credited, Uuid::new_v4()));
        repo.seed(make_order(OrderState::Acked, Uuid::new_v4()));

        let (total, rows) = repo
            .list_orders(OrderFilter {
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(total, 2);
        assert_eq!(rows.len(), 2);
    }

    // ── R-02: 按 state 过滤 ───────────────────────────────────────────────

    #[tokio::test]
    async fn r02_filter_by_state() {
        let repo = FakePaymentOrderRepo::default();
        repo.seed(make_order(OrderState::Credited, Uuid::new_v4()));
        repo.seed(make_order(OrderState::Failed, Uuid::new_v4()));
        repo.seed(make_order(OrderState::Credited, Uuid::new_v4()));

        let (total, rows) = repo
            .list_orders(OrderFilter {
                state: Some("CREDITED".to_string()),
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(total, 2, "R-02: only CREDITED orders should match");
        assert!(rows.iter().all(|r| r.state == OrderState::Credited));
    }

    // ── R-03: 按 user_id 过滤 ─────────────────────────────────────────────

    #[tokio::test]
    async fn r03_filter_by_user_id() {
        let repo = FakePaymentOrderRepo::default();
        let target_user = Uuid::new_v4();
        repo.seed(make_order(OrderState::Acked, target_user));
        repo.seed(make_order(OrderState::Acked, Uuid::new_v4()));

        let (total, _) = repo
            .list_orders(OrderFilter {
                user_id: Some(target_user),
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(total, 1, "R-03: only 1 order for target user");
    }

    // ── R-04: find_by_id 找到 ─────────────────────────────────────────────

    #[tokio::test]
    async fn r04_find_by_id_found() {
        let repo = FakePaymentOrderRepo::default();
        let order = make_order(OrderState::Credited, Uuid::new_v4());
        let oid = order.order_id;
        repo.seed(order);

        let found = repo.find_by_id(oid).await.unwrap();
        assert!(found.is_some(), "R-04: order should be found");
        assert_eq!(found.unwrap().order_id, oid);
    }

    // ── R-05: find_by_id 不存在 → None ──────────────────────────────────

    #[tokio::test]
    async fn r05_find_by_id_not_found() {
        let repo = FakePaymentOrderRepo::default();
        let found = repo.find_by_id(Uuid::new_v4()).await.unwrap();
        assert!(found.is_none(), "R-05: missing order should return None");
    }

    // ── R-06: to_detail 包含 state_history ───────────────────────────────

    #[tokio::test]
    async fn r06_to_detail_contains_state_history() {
        let mut order = make_order(OrderState::Credited, Uuid::new_v4());
        order.state_history =
            serde_json::json!([{"state":"CREDITED","ts":"2024-01-01T00:00:00Z","source":"admin_recredit"}]);
        let detail = order.to_detail();
        let history = detail.state_history.as_array().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0]["state"], "CREDITED");
    }

    // ── R-07: to_list_item 脱敏 token ────────────────────────────────────

    #[test]
    fn r07_to_list_item_masked_token() {
        let order = make_order(OrderState::Credited, Uuid::new_v4());
        let item = order.to_list_item();
        assert_eq!(
            item.purchase_token_masked,
            Some("oojkl...ABCD".to_string())
        );
    }

    // ── R-08: offset/limit 分页 ───────────────────────────────────────────

    #[tokio::test]
    async fn r08_pagination_offset_limit() {
        let repo = FakePaymentOrderRepo::default();
        for _ in 0..5 {
            repo.seed(make_order(OrderState::Acked, Uuid::new_v4()));
        }

        let (total, rows) = repo
            .list_orders(OrderFilter {
                limit: 2,
                offset: 3,
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(total, 5, "total must be 5");
        assert_eq!(rows.len(), 2, "page should return 2 items");
    }
}
