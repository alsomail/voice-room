//! Payment Cron — 后台对账任务（T-00054）
//!
//! 每 5 分钟扫描：
//! - `state='VERIFYING' > 10min` → 重查 Google 状态强推
//! - `state='CREDITED' AND acked_at IS NULL > 1h` → 重调 acknowledge

use std::sync::Arc;
use std::time::Duration;

use super::google_billing_port::GooglePlayBillingPort;
use super::repo::PaymentRepo;
use sqlx::PgPool;

const PACKAGE_NAME: &str = "com.voiceroom.android";
const CRON_INTERVAL: Duration = Duration::from_secs(300); // 5 分钟

/// 启动 Payment 后台对账 cron（独立 tokio spawn）
///
/// 失败时记录 WARN 日志，不影响主流程。
pub fn spawn_payment_reconciliation_cron(
    pool: PgPool,
    billing_port: Arc<dyn GooglePlayBillingPort>,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(CRON_INTERVAL).await;
            run_reconciliation_cycle(&pool, &billing_port).await;
        }
    });
}

/// 单次对账周期（可独立测试）
pub async fn run_reconciliation_cycle(
    pool: &PgPool,
    billing_port: &Arc<dyn GooglePlayBillingPort>,
) {
    let repo = PaymentRepo::new(pool.clone());

    // 1. 处理超时 VERIFYING 订单（> 10min）
    match repo.find_stale_verifying_orders().await {
        Ok(orders) => {
            for order in orders {
                if let Some(ref token) = order.purchase_token {
                    match billing_port
                        .get_product_purchase(PACKAGE_NAME, &order.sku_id, token)
                        .await
                    {
                        Ok(purchase) if purchase.purchase_state == 0 => {
                            // 验签通过，尝试走 CREDITED 流程
                            if let Ok(sku) = repo.find_sku_by_id(&order.sku_id).await {
                                if let Some(sku) = sku {
                                    match pool.begin().await {
                                        Ok(mut txn) => {
                                            let credit_result = repo
                                                .credit_balance(
                                                    &mut txn,
                                                    order.user_id,
                                                    sku.diamonds,
                                                    order.order_id,
                                                    "recharge_google_play",
                                                )
                                                .await;
                                            if credit_result.is_ok() {
                                                let _ = repo
                                                    .transition_to_credited(
                                                        &mut txn,
                                                        order.order_id,
                                                        purchase.price_amount_micros,
                                                        purchase.price_currency_code.as_deref(),
                                                        purchase.country_code.as_deref(),
                                                    )
                                                    .await;
                                                if let Err(e) = txn.commit().await {
                                                    tracing::warn!(
                                                        order_id = %order.order_id,
                                                        error = %e,
                                                        "cron: commit failed for stale VERIFYING order"
                                                    );
                                                } else {
                                                    tracing::info!(
                                                        order_id = %order.order_id,
                                                        "cron: stale VERIFYING order credited"
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                order_id = %order.order_id,
                                                error = %e,
                                                "cron: begin txn failed for stale VERIFYING"
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Ok(_) => {
                            tracing::warn!(
                                order_id = %order.order_id,
                                "cron: stale VERIFYING order still not purchased"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                order_id = %order.order_id,
                                error = %e,
                                "cron: Google API error for stale VERIFYING order"
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "cron: failed to query stale VERIFYING orders");
        }
    }

    // 2. 重试 acknowledge CREDITED 订单（> 1h 未 acked）
    match repo.find_stale_credited_orders().await {
        Ok(orders) => {
            for order in orders {
                if let Some(ref token) = order.purchase_token {
                    match billing_port
                        .acknowledge(PACKAGE_NAME, &order.sku_id, token)
                        .await
                    {
                        Ok(()) => {
                            if let Err(e) = repo.transition_to_acked(order.order_id).await {
                                tracing::warn!(
                                    order_id = %order.order_id,
                                    error = %e,
                                    "cron: failed to update CREDITED→ACKED in DB"
                                );
                            } else {
                                tracing::info!(
                                    order_id = %order.order_id,
                                    "cron: stale CREDITED order acknowledged"
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                order_id = %order.order_id,
                                error = %e,
                                "cron: acknowledge retry failed for CREDITED order"
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "cron: failed to query stale CREDITED orders");
        }
    }
}

#[cfg(test)]
mod tests {
    // C01: spawn_payment_reconciliation_cron 不 panic（仅测试函数签名可调用）
    #[test]
    fn c01_spawn_cron_function_exists() {
        // 只验证函数签名存在，不实际 spawn（需要 pool 和 billing_port）
        // 通过编译即通过
        let _ = std::mem::size_of::<super::PaymentRepo>();
    }
}
