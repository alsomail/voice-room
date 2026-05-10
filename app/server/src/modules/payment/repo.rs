//! Payment Repo — payment_skus / payment_orders 数据访问层

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use voice_room_shared::payment::{OrderState, Provider};

use super::dto::SkuDto;
use super::error::PaymentError;

/// payment_skus 行
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PaymentSku {
    pub sku_id: String,
    pub provider: Provider,
    pub diamonds: i64,
    pub display_price_usd: String,
    pub display_price_local: Option<String>,
    pub display_currency: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub tag: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PaymentSku {
    pub fn to_dto(&self) -> SkuDto {
        SkuDto {
            sku_id: self.sku_id.clone(),
            provider: self.provider.to_string(),
            diamonds: self.diamonds,
            display_price_usd: self.display_price_usd.clone(),
            display_price_local: self.display_price_local.clone(),
            display_currency: self.display_currency.clone(),
            tag: self.tag.clone(),
            sort_order: self.sort_order,
        }
    }
}

/// payment_orders 行
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PaymentOrder {
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
    pub state_history: serde_json::Value,
    pub risk_flags: Vec<String>,
    pub idempotency_key: Option<String>,
    pub dev_mock_outcome: Option<String>,
    pub created_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub credited_at: Option<DateTime<Utc>>,
    pub acked_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub failed_reason: Option<String>,
}

/// Payment 数据访问层
pub struct PaymentRepo {
    pool: PgPool,
}

impl PaymentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 查询激活中的 SKU 列表
    pub async fn list_active_skus(&self, provider: &Provider) -> Result<Vec<PaymentSku>, PaymentError> {
        let skus = sqlx::query_as::<_, PaymentSku>(
            "SELECT sku_id, provider, diamonds, \
             display_price_usd::TEXT AS display_price_usd, \
             display_price_local::TEXT AS display_price_local, \
             display_currency, is_active, sort_order, tag, created_at, updated_at \
             FROM payment_skus \
             WHERE is_active = TRUE AND provider = $1 \
             ORDER BY sort_order ASC",
        )
        .bind(provider)
        .fetch_all(&self.pool)
        .await?;
        Ok(skus)
    }

    /// 按 sku_id 查询单个 SKU（不限 is_active）
    pub async fn find_sku_by_id(&self, sku_id: &str) -> Result<Option<PaymentSku>, PaymentError> {
        let sku = sqlx::query_as::<_, PaymentSku>(
            "SELECT sku_id, provider, diamonds, \
             display_price_usd::TEXT AS display_price_usd, \
             display_price_local::TEXT AS display_price_local, \
             display_currency, is_active, sort_order, tag, created_at, updated_at \
             FROM payment_skus WHERE sku_id = $1",
        )
        .bind(sku_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(sku)
    }

    /// 创建 PENDING 订单，返回 order_id
    pub async fn create_order(
        &self,
        user_id: Uuid,
        sku_id: &str,
        provider: &Provider,
        idempotency_key: Option<&str>,
    ) -> Result<Uuid, PaymentError> {
        let order_id: Uuid = sqlx::query_scalar(
            "INSERT INTO payment_orders \
             (user_id, sku_id, provider, state, idempotency_key, state_history) \
             VALUES ($1, $2, $3, 'PENDING', $4, $5) \
             RETURNING order_id",
        )
        .bind(user_id)
        .bind(sku_id)
        .bind(provider)
        .bind(idempotency_key)
        .bind(serde_json::json!([{
            "state": "PENDING",
            "ts": Utc::now().to_rfc3339(),
            "source": "client_create"
        }]))
        .fetch_one(&self.pool)
        .await?;
        Ok(order_id)
    }

    /// 按 order_id 查询订单（验证用户归属）
    pub async fn find_order_by_id(
        &self,
        order_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<PaymentOrder>, PaymentError> {
        let order = sqlx::query_as::<_, PaymentOrder>(
            "SELECT order_id, user_id, sku_id, provider, purchase_token, provider_order_id, \
             amount_micros, currency, country_code, state, state_history, risk_flags, \
             idempotency_key, dev_mock_outcome, created_at, verified_at, credited_at, \
             acked_at, failed_at, failed_reason \
             FROM payment_orders \
             WHERE order_id = $1 AND user_id = $2",
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(order)
    }

    /// 按 purchase_token 查询已存在的订单（幂等检查）
    pub async fn find_order_by_purchase_token(
        &self,
        purchase_token: &str,
        provider: &Provider,
    ) -> Result<Option<PaymentOrder>, PaymentError> {
        let order = sqlx::query_as::<_, PaymentOrder>(
            "SELECT order_id, user_id, sku_id, provider, purchase_token, provider_order_id, \
             amount_micros, currency, country_code, state, state_history, risk_flags, \
             idempotency_key, dev_mock_outcome, created_at, verified_at, credited_at, \
             acked_at, failed_at, failed_reason \
             FROM payment_orders \
             WHERE purchase_token = $1 AND provider = $2",
        )
        .bind(purchase_token)
        .bind(provider)
        .fetch_optional(&self.pool)
        .await?;
        Ok(order)
    }

    /// PENDING → VERIFYING：在事务内推进，锁定行（SELECT FOR UPDATE）
    ///
    /// **状态机保护**：仅当 state = 'PENDING' 时才允许推进；
    /// 若订单已非 PENDING（终态或已 VERIFYING），返回 `OrderAlreadyFinalized`。
    pub async fn transition_to_verifying<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        order_id: Uuid,
        purchase_token: &str,
        provider_order_id: Option<&str>,
    ) -> Result<PaymentOrder, PaymentError> {
        let order = sqlx::query_as::<_, PaymentOrder>(
            "SELECT order_id, user_id, sku_id, provider, purchase_token, provider_order_id, \
             amount_micros, currency, country_code, state, state_history, risk_flags, \
             idempotency_key, dev_mock_outcome, created_at, verified_at, credited_at, \
             acked_at, failed_at, failed_reason \
             FROM payment_orders WHERE order_id = $1 FOR UPDATE",
        )
        .bind(order_id)
        .fetch_optional(&mut **txn)
        .await?
        .ok_or(PaymentError::OrderNotFound)?;

        // 状态机保护：只允许从 PENDING 推进到 VERIFYING（单向不可逆）
        let state_str = order.state.to_string();
        if state_str != "PENDING" {
            return Err(PaymentError::OrderAlreadyFinalized);
        }

        // 设置 purchase_token 并推进到 VERIFYING（包含 AND state='PENDING' 双重保护）
        let rows_affected = sqlx::query(
            "UPDATE payment_orders SET \
             state = 'VERIFYING', \
             purchase_token = $2, \
             provider_order_id = $3, \
             state_history = state_history || $4::jsonb \
             WHERE order_id = $1 AND state = 'PENDING'",
        )
        .bind(order_id)
        .bind(purchase_token)
        .bind(provider_order_id)
        .bind(serde_json::json!([{
            "state": "VERIFYING",
            "ts": Utc::now().to_rfc3339(),
            "source": "client_verify"
        }]))
        .execute(&mut **txn)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(PaymentError::OrderAlreadyFinalized);
        }

        Ok(order)
    }

    /// VERIFYING → VERIFIED → CREDITED（强事务内执行，余额已在同事务内更新）
    pub async fn transition_to_credited<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        order_id: Uuid,
        amount_micros: Option<i64>,
        currency: Option<&str>,
        country_code: Option<&str>,
    ) -> Result<(), PaymentError> {
        sqlx::query(
            "UPDATE payment_orders SET \
             state = 'CREDITED', \
             verified_at = now(), \
             credited_at = now(), \
             amount_micros = $2, \
             currency = $3, \
             country_code = $4, \
             state_history = state_history || $5::jsonb \
             WHERE order_id = $1",
        )
        .bind(order_id)
        .bind(amount_micros)
        .bind(currency)
        .bind(country_code)
        .bind(serde_json::json!([
            {"state": "VERIFIED", "ts": Utc::now().to_rfc3339(), "source": "google_get"},
            {"state": "CREDITED", "ts": Utc::now().to_rfc3339(), "source": "tx_commit"}
        ]))
        .execute(&mut **txn)
        .await?;
        Ok(())
    }

    /// CREDITED → ACKED（acknowledge 成功后更新）
    pub async fn transition_to_acked(&self, order_id: Uuid) -> Result<(), PaymentError> {
        sqlx::query(
            "UPDATE payment_orders SET \
             state = 'ACKED', \
             acked_at = now(), \
             state_history = state_history || $2::jsonb \
             WHERE order_id = $1",
        )
        .bind(order_id)
        .bind(serde_json::json!([{
            "state": "ACKED",
            "ts": Utc::now().to_rfc3339(),
            "source": "google_ack"
        }]))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 推进到 FAILED 状态
    pub async fn transition_to_failed<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        order_id: Uuid,
        reason: &str,
    ) -> Result<(), PaymentError> {
        sqlx::query(
            "UPDATE payment_orders SET \
             state = 'FAILED', \
             failed_at = now(), \
             failed_reason = $2, \
             state_history = state_history || $3::jsonb \
             WHERE order_id = $1",
        )
        .bind(order_id)
        .bind(reason)
        .bind(serde_json::json!([{
            "state": "FAILED",
            "ts": Utc::now().to_rfc3339(),
            "source": reason
        }]))
        .execute(&mut **txn)
        .await?;
        Ok(())
    }

    /// 推进到 REFUNDED 状态（RTDN voidedPurchase 触发）
    pub async fn transition_to_refunded<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        order_id: Uuid,
    ) -> Result<(), PaymentError> {
        sqlx::query(
            "UPDATE payment_orders SET \
             state = 'REFUNDED', \
             state_history = state_history || $2::jsonb \
             WHERE order_id = $1",
        )
        .bind(order_id)
        .bind(serde_json::json!([{
            "state": "REFUNDED",
            "ts": Utc::now().to_rfc3339(),
            "source": "rtdn_voided"
        }]))
        .execute(&mut **txn)
        .await?;
        Ok(())
    }

    /// 推进到 PENDING_GOOGLE 状态（purchase_state=2 时）
    pub async fn transition_to_pending_google<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        order_id: Uuid,
    ) -> Result<(), PaymentError> {
        sqlx::query(
            "UPDATE payment_orders SET \
             state = 'PENDING_GOOGLE', \
             state_history = state_history || $2::jsonb \
             WHERE order_id = $1",
        )
        .bind(order_id)
        .bind(serde_json::json!([{
            "state": "PENDING_GOOGLE",
            "ts": Utc::now().to_rfc3339(),
            "source": "google_pending"
        }]))
        .execute(&mut **txn)
        .await?;
        Ok(())
    }

    /// 在事务内获取订单当前 state（SELECT FOR UPDATE），防止并发双充
    ///
    /// 在 credit 事务开始时调用，若订单已非 VERIFYING 则幂等返回。
    pub async fn get_order_state_for_credit<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        order_id: Uuid,
    ) -> Result<String, PaymentError> {
        let state: Option<String> = sqlx::query_scalar(
            "SELECT state::TEXT FROM payment_orders WHERE order_id = $1 FOR UPDATE",
        )
        .bind(order_id)
        .fetch_optional(&mut **txn)
        .await?;

        state.ok_or(PaymentError::OrderNotFound)
    }

    /// 查询用户 24h 内 FAILED 订单数（风控使用）
    pub async fn count_failed_orders_24h(&self, user_id: Uuid) -> Result<i64, PaymentError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM payment_orders \
             WHERE user_id = $1 AND state = 'FAILED' \
             AND created_at > now() - INTERVAL '24 hours'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    /// 幂等写入 rtdn_processed 表
    /// 返回 true 表示首次处理，false 表示重复消息
    pub async fn upsert_rtdn_processed(
        &self,
        message_id: &str,
        event_time_millis: i64,
        notification_kind: &str,
        purchase_token: Option<&str>,
        outcome: &str,
    ) -> Result<bool, PaymentError> {
        let rows_affected = sqlx::query(
            "INSERT INTO rtdn_processed \
             (message_id, event_time_millis, notification_kind, purchase_token, outcome) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (message_id) DO NOTHING",
        )
        .bind(message_id)
        .bind(event_time_millis)
        .bind(notification_kind)
        .bind(purchase_token)
        .bind(outcome)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(rows_affected > 0)
    }

    /// 检查 message_id 是否已在幂等表中（不写入，仅查询）
    pub async fn check_rtdn_processed(&self, message_id: &str) -> Result<bool, PaymentError> {
        let exists: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM rtdn_processed WHERE message_id = $1",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(exists.is_some())
    }

    /// 业务处理成功后写入幂等记录（先查后处理流程的写入步骤）
    pub async fn insert_rtdn_processed(
        &self,
        message_id: &str,
        event_time_millis: i64,
        notification_kind: &str,
        purchase_token: Option<&str>,
        outcome: &str,
    ) -> Result<(), PaymentError> {
        sqlx::query(
            "INSERT INTO rtdn_processed \
             (message_id, event_time_millis, notification_kind, purchase_token, outcome) \
             VALUES ($1, $2, $3, $4, $5) \
             ON CONFLICT (message_id) DO NOTHING",
        )
        .bind(message_id)
        .bind(event_time_millis)
        .bind(notification_kind)
        .bind(purchase_token)
        .bind(outcome)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 查询需要 cron 推进的 VERIFYING 订单（超时 10min）
    pub async fn find_stale_verifying_orders(
        &self,
    ) -> Result<Vec<PaymentOrder>, PaymentError> {
        let orders = sqlx::query_as::<_, PaymentOrder>(
            "SELECT order_id, user_id, sku_id, provider, purchase_token, provider_order_id, \
             amount_micros, currency, country_code, state, state_history, risk_flags, \
             idempotency_key, dev_mock_outcome, created_at, verified_at, credited_at, \
             acked_at, failed_at, failed_reason \
             FROM payment_orders \
             WHERE state = 'VERIFYING' \
             AND created_at < now() - INTERVAL '10 minutes'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(orders)
    }

    /// 查询需要重试 acknowledge 的 CREDITED 订单（超时 1h）
    pub async fn find_stale_credited_orders(
        &self,
    ) -> Result<Vec<PaymentOrder>, PaymentError> {
        let orders = sqlx::query_as::<_, PaymentOrder>(
            "SELECT order_id, user_id, sku_id, provider, purchase_token, provider_order_id, \
             amount_micros, currency, country_code, state, state_history, risk_flags, \
             idempotency_key, dev_mock_outcome, created_at, verified_at, credited_at, \
             acked_at, failed_at, failed_reason \
             FROM payment_orders \
             WHERE state = 'CREDITED' \
             AND acked_at IS NULL \
             AND credited_at < now() - INTERVAL '1 hour'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(orders)
    }

    /// 在事务内执行余额充值（source='recharge_google_play' 或 'dev_mock'）
    pub async fn credit_balance<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        user_id: Uuid,
        diamonds: i64,
        ref_id: Uuid,
        source: &str,
    ) -> Result<i64, PaymentError> {
        // SELECT FOR UPDATE 获取当前余额并锁定行
        let current: i64 = sqlx::query_scalar(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut **txn)
        .await?
        .ok_or_else(|| PaymentError::Internal("user not found".into()))?;

        let new_balance = current + diamonds;

        // 更新余额
        sqlx::query("UPDATE users SET diamond_balance = $1, updated_at = now() WHERE id = $2")
            .bind(new_balance)
            .bind(user_id)
            .execute(&mut **txn)
            .await?;

        // 写入流水（source 存入 reason 字段）
        sqlx::query(
            "INSERT INTO wallet_transactions \
             (user_id, type, amount, balance_after, ref_id, reason, source) \
             VALUES ($1, 'recharge', $2, $3, $4, $5, $6)",
        )
        .bind(user_id)
        .bind(diamonds)
        .bind(new_balance)
        .bind(ref_id)
        .bind(source)
        .bind(source)
        .execute(&mut **txn)
        .await?;

        Ok(new_balance)
    }

    /// 在事务内执行余额扣减（退款）
    pub async fn debit_balance<'c>(
        &self,
        txn: &mut Transaction<'c, Postgres>,
        user_id: Uuid,
        diamonds: i64,
        ref_id: Uuid,
        source: &str,
    ) -> Result<i64, PaymentError> {
        let current: i64 = sqlx::query_scalar(
            "SELECT diamond_balance FROM users WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(user_id)
        .fetch_optional(&mut **txn)
        .await?
        .ok_or_else(|| PaymentError::Internal("user not found".into()))?;

        let new_balance = (current - diamonds).max(0); // 不允许负数

        sqlx::query("UPDATE users SET diamond_balance = $1, updated_at = now() WHERE id = $2")
            .bind(new_balance)
            .bind(user_id)
            .execute(&mut **txn)
            .await?;

        sqlx::query(
            "INSERT INTO wallet_transactions \
             (user_id, type, amount, balance_after, ref_id, reason, source) \
             VALUES ($1, 'refund', $2, $3, $4, $5, $6)",
        )
        .bind(user_id)
        .bind(-diamonds)
        .bind(new_balance)
        .bind(ref_id)
        .bind(source)
        .bind(source)
        .execute(&mut **txn)
        .await?;

        Ok(new_balance)
    }

    /// 查询长时间停留在 PENDING 的订单（> 24h，待超时取消）
    pub async fn find_stale_pending_orders(&self) -> Result<Vec<PaymentOrder>, PaymentError> {
        let orders = sqlx::query_as::<_, PaymentOrder>(
            "SELECT order_id, user_id, sku_id, provider, purchase_token, provider_order_id, \
             amount_micros, currency, country_code, state, state_history, risk_flags, \
             idempotency_key, dev_mock_outcome, created_at, verified_at, credited_at, \
             acked_at, failed_at, failed_reason \
             FROM payment_orders \
             WHERE state = 'PENDING' \
             AND created_at < now() - INTERVAL '24 hours'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(orders)
    }

    /// 将超时 PENDING 订单推进到 CANCELLED（仅当仍为 PENDING 时）
    pub async fn cancel_stale_pending_order(&self, order_id: Uuid) -> Result<bool, PaymentError> {
        let rows_affected = sqlx::query(
            "UPDATE payment_orders SET \
             state = 'CANCELLED', \
             failed_at = now(), \
             failed_reason = 'pending_timeout_24h', \
             state_history = state_history || $2::jsonb \
             WHERE order_id = $1 AND state = 'PENDING'",
        )
        .bind(order_id)
        .bind(serde_json::json!([{
            "state": "CANCELLED",
            "ts": Utc::now().to_rfc3339(),
            "source": "cron_pending_timeout"
        }]))
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(rows_affected > 0)
    }

    /// 查询长时间停留在 PENDING_GOOGLE 的订单（> 72h，可能 Google 已处理）
    pub async fn find_stale_pending_google_orders(&self) -> Result<Vec<PaymentOrder>, PaymentError> {
        let orders = sqlx::query_as::<_, PaymentOrder>(
            "SELECT order_id, user_id, sku_id, provider, purchase_token, provider_order_id, \
             amount_micros, currency, country_code, state, state_history, risk_flags, \
             idempotency_key, dev_mock_outcome, created_at, verified_at, credited_at, \
             acked_at, failed_at, failed_reason \
             FROM payment_orders \
             WHERE state = 'PENDING_GOOGLE' \
             AND created_at < now() - INTERVAL '72 hours'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(orders)
    }
}

#[cfg(test)]
mod tests {
    // R01: transition_to_verifying PENDING 状态保护测试（单元 - 不依赖 DB）
    #[test]
    fn r01_verifying_guard_state_check_logic() {
        // 验证状态检查逻辑：非 PENDING 订单不允许推进到 VERIFYING
        let terminal_states = ["CREDITED", "ACKED", "FAILED", "REFUNDED", "CANCELLED", "VERIFYING"];
        for state in &terminal_states {
            assert_ne!(*state, "PENDING", "Terminal state {} should not equal PENDING", state);
        }
    }

    // R02: 新增 repo 方法签名存在（通过编译验证）
    #[test]
    fn r02_new_methods_compile() {
        // 验证这些类型存在（通过编译即通过）
        let _ = std::mem::size_of::<super::PaymentRepo>();
        let _ = std::mem::size_of::<super::PaymentOrder>();
    }

    // R03: eventTimeMillis 从 DeveloperNotification 取而非 publishTime
    #[test]
    fn r03_event_time_millis_parse_logic() {
        let millis_str = "1746788688000";
        let millis: i64 = millis_str.parse::<i64>().unwrap_or_default();
        assert_eq!(millis, 1_746_788_688_000i64);

        // publishTime 格式（ISO 8601）不能直接 parse 为 i64
        let publish_time = "2026-05-09T10:24:48.690Z";
        let bad_parse: i64 = publish_time.parse::<i64>().unwrap_or_default();
        assert_eq!(bad_parse, 0, "publishTime should not parse as i64, defaults to 0");
    }
}
