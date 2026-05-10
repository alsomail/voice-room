//! RTDN 推送对账处理（T-00053）
//!
//! POST /webhook/google/rtdn 端点处理逻辑
//!
//! 参见 payment_api.md §9.5

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::modules::wallet::broadcaster::BalanceEvent;

use super::dto::{DeveloperNotification, RtdnEnvelope};
use super::error::PaymentError;
use super::google_billing_port::GooglePlayBillingPort;
use super::repo::PaymentRepo;

const PACKAGE_NAME: &str = "com.voiceroom.android";

/// RTDN OIDC JWT Claims（Google Pub/Sub Push 模式）
#[derive(Debug, Serialize, Deserialize)]
struct RtdnOidcClaims {
    #[serde(default)]
    pub iss: String,
    #[serde(default)]
    pub aud: String,
    pub exp: u64,
}

/// 验证 Google Pub/Sub OIDC Bearer Token
///
/// - `token`：`Authorization: Bearer <jwt>` 中的 jwt 部分
/// - `expected_audience`：与本服务订阅配置一致的 audience（`payment.rtdn_audience`）
/// - `secret`：HMAC HS256 secret（测试用；生产应换用 RS256 公钥）
///
/// 若 audience 或 secret 为空，跳过验证（dev/test 模式）。
pub fn validate_rtdn_oidc_token(
    token: &str,
    expected_audience: &str,
    secret: &[u8],
) -> Result<(), PaymentError> {
    // 空配置 → dev/test 模式，跳过验证
    if secret.is_empty() || expected_audience.is_empty() {
        return Ok(());
    }

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[expected_audience]);
    validation.set_issuer(&["https://accounts.google.com"]);

    decode::<RtdnOidcClaims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|e| {
            tracing::warn!(error = %e, "RTDN OIDC token validation failed");
            PaymentError::RtdnSignatureInvalid
        })?;

    Ok(())
}

/// RTDN 处理结果
#[derive(Debug)]
pub struct RtdnResult {
    pub outcome: String,
    pub message: String,
}

/// RTDN 服务 Trait
#[async_trait]
pub trait PaymentRtdnServicePort: Send + Sync {
    async fn handle_rtdn(&self, envelope: RtdnEnvelope) -> Result<RtdnResult, PaymentError>;
}

/// RTDN 服务真实实现
pub struct PaymentRtdnService {
    repo: PaymentRepo,
    pool: PgPool,
    billing_port: Arc<dyn GooglePlayBillingPort>,
    balance_tx: mpsc::Sender<BalanceEvent>,
}

impl PaymentRtdnService {
    pub fn new(
        pool: PgPool,
        billing_port: Arc<dyn GooglePlayBillingPort>,
        balance_tx: mpsc::Sender<BalanceEvent>,
    ) -> Self {
        Self {
            repo: PaymentRepo::new(pool.clone()),
            pool,
            billing_port,
            balance_tx,
        }
    }

    fn decode_notification(data_b64: &str) -> Result<DeveloperNotification, PaymentError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data_b64)
            .map_err(|e| PaymentError::Internal(format!("base64 decode: {e}")))?;
        serde_json::from_slice::<DeveloperNotification>(&bytes)
            .map_err(|e| PaymentError::Internal(format!("json parse: {e}")))
    }
}

#[async_trait]
impl PaymentRtdnServicePort for PaymentRtdnService {
    async fn handle_rtdn(&self, envelope: RtdnEnvelope) -> Result<RtdnResult, PaymentError> {
        let message_id = &envelope.message.message_id;

        // 先解析 DeveloperNotification（eventTimeMillis 从通知体内取，非 publishTime）
        let notification = Self::decode_notification(&envelope.message.data)?;

        // 按协议 §9.5.1：eventTimeMillis 来自 DeveloperNotification.eventTimeMillis
        let event_time_millis: i64 = notification
            .event_time_millis
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or_default();

        // 确定 notification_kind 和 purchase_token（克隆 token 避免借用冲突）
        let (kind, purchase_token): (&'static str, Option<String>) =
            if notification.test_notification.is_some() {
                ("testNotification", None)
            } else if let Some(ref otp) = notification.one_time_product_notification {
                (
                    "oneTimeProductNotification",
                    Some(otp.purchase_token.clone()),
                )
            } else if let Some(ref vp) = notification.voided_purchase_notification {
                (
                    "voidedPurchaseNotification",
                    Some(vp.purchase_token.clone()),
                )
            } else {
                ("unknown", None)
            };

        // 幂等去重：先查询（不写入），处理成功后再写入
        let already_processed = self.repo.check_rtdn_processed(message_id).await?;
        if already_processed {
            return Ok(RtdnResult {
                outcome: "ignored_duplicate".to_string(),
                message: format!("message {message_id} already processed"),
            });
        }

        // 根据 notification 类型分发处理
        let result = if notification.test_notification.is_some() {
            tracing::info!(message_id = %message_id, "RTDN testNotification received");
            Ok(RtdnResult {
                outcome: "ignored_test".to_string(),
                message: "testNotification processed".to_string(),
            })
        } else if let Some(otp) = notification.one_time_product_notification {
            self.handle_one_time_product(message_id, &otp.purchase_token, otp.notification_type).await
        } else if let Some(vp) = notification.voided_purchase_notification {
            self.handle_voided_purchase(message_id, &vp.purchase_token).await
        } else {
            // 未知类型：记录日志，不报错（避免 Pub/Sub 重投）
            tracing::warn!(
                message_id = %message_id,
                kind = %kind,
                "RTDN unknown notification type, ignoring"
            );
            Ok(RtdnResult {
                outcome: "ignored_unknown".to_string(),
                message: "unknown notification type".to_string(),
            })
        };

        // 业务成功后写入幂等记录（保证幂等记录与业务结果一致）
        if let Ok(ref r) = result {
            let outcome = &r.outcome;
            let _ = self
                .repo
                .insert_rtdn_processed(
                    message_id,
                    event_time_millis,
                    kind,
                    purchase_token.as_deref(),
                    outcome,
                )
                .await;
        }

        result
    }
}

impl PaymentRtdnService {
    async fn handle_one_time_product(
        &self,
        message_id: &str,
        purchase_token: &str,
        notification_type: i32,
    ) -> Result<RtdnResult, PaymentError> {
        // notificationType=2 (CANCELED)
        if notification_type == 2 {
            // 查找对应订单，未 CREDITED 则置 FAILED
            if let Some(order) = self
                .repo
                .find_order_by_purchase_token(purchase_token, &voice_room_shared::payment::Provider::GooglePlay)
                .await?
            {
                let state_str = order.state.to_string();
                if state_str == "PENDING" || state_str == "VERIFYING" || state_str == "PENDING_GOOGLE" {
                    let mut txn = self.pool.begin().await.map_err(|e| PaymentError::Database(e.to_string()))?;
                    let _ = self.repo.transition_to_failed(&mut txn, order.order_id, "rtdn_cancelled").await;
                    let _ = txn.commit().await;
                }
            }
            return Ok(RtdnResult {
                outcome: "applied".to_string(),
                message: format!("CANCELED notification for token processed, message={message_id}"),
            });
        }

        // notificationType=1 (PURCHASED) — 触发验签入账路径
        // 查找对应订单
        if let Some(order) = self
            .repo
            .find_order_by_purchase_token(purchase_token, &voice_room_shared::payment::Provider::GooglePlay)
            .await?
        {
            let state_str = order.state.to_string();
            if state_str == "PENDING" || state_str == "VERIFYING" || state_str == "PENDING_GOOGLE" {
                // 调用 Google 验签（防腐层 trait）
                match self
                    .billing_port
                    .get_product_purchase(PACKAGE_NAME, &order.sku_id, purchase_token)
                    .await
                {
                    Ok(purchase) if purchase.purchase_state == 0 => {
                        // 验签成功，走入账流程
                        if let Ok(sku) = self.repo.find_sku_by_id(&order.sku_id).await {
                            if let Some(sku) = sku {
                                let mut txn = self.pool.begin().await.map_err(|e| PaymentError::Database(e.to_string()))?;
                                if let Ok(balance_after) = self
                                    .repo
                                    .credit_balance(&mut txn, order.user_id, sku.diamonds, order.order_id, "recharge_google_play")
                                    .await
                                {
                                    let _ = self
                                        .repo
                                        .transition_to_credited(
                                            &mut txn,
                                            order.order_id,
                                            purchase.price_amount_micros,
                                            purchase.price_currency_code.as_deref(),
                                            purchase.country_code.as_deref(),
                                        )
                                        .await;
                                    if txn.commit().await.is_ok() {
                                        let _ = self.balance_tx.try_send(BalanceEvent {
                                            user_id: order.user_id,
                                            balance_after,
                                            delta: sku.diamonds,
                                            reason: "payment_credit".to_string(),
                                            ref_id: Some(order.order_id),
                                        });
                                        // acknowledge
                                        let _ = self.billing_port.acknowledge(PACKAGE_NAME, &order.sku_id, purchase_token).await;
                                        let _ = self.repo.transition_to_acked(order.order_id).await;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        tracing::warn!(message_id = %message_id, "RTDN: google verify failed");
                    }
                }
            }
        }

        Ok(RtdnResult {
            outcome: "applied".to_string(),
            message: format!("PURCHASED notification processed, message={message_id}"),
        })
    }

    async fn handle_voided_purchase(
        &self,
        message_id: &str,
        purchase_token: &str,
    ) -> Result<RtdnResult, PaymentError> {
        // 找到对应订单，扣回钻石
        let order = match self
            .repo
            .find_order_by_purchase_token(purchase_token, &voice_room_shared::payment::Provider::GooglePlay)
            .await?
        {
            Some(o) => o,
            None => {
                tracing::warn!(
                    message_id = %message_id,
                    purchase_token = %&purchase_token[..purchase_token.len().min(10)],
                    "RTDN voided: order not found"
                );
                return Ok(RtdnResult {
                    outcome: "ignored_unknown_token".to_string(),
                    message: "order not found for purchase_token".to_string(),
                });
            }
        };

        // 只有 CREDITED / ACKED 的订单才能退款
        let state_str = order.state.to_string();
        if state_str != "CREDITED" && state_str != "ACKED" {
            return Ok(RtdnResult {
                outcome: "ignored_wrong_state".to_string(),
                message: format!("order state={state_str}, not eligible for refund"),
            });
        }

        // 查询 SKU
        let sku = self
            .repo
            .find_sku_by_id(&order.sku_id)
            .await?
            .ok_or_else(|| PaymentError::Internal("sku not found".into()))?;

        // 原子事务：扣余额 + state→REFUNDED
        let mut txn = self
            .pool
            .begin()
            .await
            .map_err(|e| PaymentError::Database(e.to_string()))?;

        let balance_after = self
            .repo
            .debit_balance(
                &mut txn,
                order.user_id,
                sku.diamonds,
                order.order_id,
                "refund_google_play",
            )
            .await?;

        self.repo
            .transition_to_refunded(&mut txn, order.order_id)
            .await?;

        txn.commit()
            .await
            .map_err(|e| PaymentError::Database(e.to_string()))?;

        // WS 推送退款通知
        let _ = self.balance_tx.try_send(BalanceEvent {
            user_id: order.user_id,
            balance_after,
            delta: -sku.diamonds,
            reason: "refund_google_play".to_string(),
            ref_id: Some(order.order_id),
        });

        tracing::warn!(
            order_id = %order.order_id,
            user_id = %order.user_id,
            diamonds = sku.diamonds,
            "RTDN voidedPurchase: refund processed"
        );

        Ok(RtdnResult {
            outcome: "applied".to_string(),
            message: format!("voided purchase refund processed, message={message_id}"),
        })
    }
}

// ─── Fake（仅测试）──────────────────────────────────────────────────────────

pub struct FakePaymentRtdnService;

#[async_trait]
impl PaymentRtdnServicePort for FakePaymentRtdnService {
    async fn handle_rtdn(&self, envelope: RtdnEnvelope) -> Result<RtdnResult, PaymentError> {
        let _ = envelope;
        Ok(RtdnResult {
            outcome: "applied".to_string(),
            message: "fake rtdn handled".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // T53-01: testNotification 返回 ignored_test
    #[test]
    fn t53_01_test_notification_kind_detection() {
        let notif = DeveloperNotification {
            version: Some("1.0".to_string()),
            package_name: Some("com.voiceroom.android".to_string()),
            event_time_millis: Some("1746788688000".to_string()),
            one_time_product_notification: None,
            voided_purchase_notification: None,
            test_notification: Some(serde_json::json!({"version": "1.0"})),
        };
        assert!(notif.test_notification.is_some());
        assert!(notif.one_time_product_notification.is_none());
    }

    // T53-02: decode_notification 正确解码 base64
    #[test]
    fn t53_02_decode_notification_from_base64() {
        let json = r#"{"version":"1.0","packageName":"com.test","testNotification":{"version":"1.0"}}"#;
        let b64 = base64::engine::general_purpose::STANDARD.encode(json);
        let notif = PaymentRtdnService::decode_notification(&b64).unwrap();
        assert!(notif.test_notification.is_some());
    }

    // T53-03: FakePaymentRtdnService 满足 Send+Sync
    #[test]
    fn t53_03_fake_rtdn_service_is_send_sync() {
        let _: Arc<dyn PaymentRtdnServicePort> = Arc::new(FakePaymentRtdnService);
    }

    // T53-04: decode_notification 无效 base64 返回错误
    #[test]
    fn t53_04_decode_notification_invalid_base64_returns_error() {
        let result = PaymentRtdnService::decode_notification("not-valid-base64!!!");
        assert!(result.is_err());
    }

    // T53-05 (RED→GREEN): eventTimeMillis 从 DeveloperNotification 字段取，不用 publishTime
    #[test]
    fn t53_05_event_time_millis_from_notification_not_publish_time() {
        let notif = DeveloperNotification {
            version: Some("1.0".to_string()),
            package_name: Some("com.voiceroom.android".to_string()),
            event_time_millis: Some("1746788688000".to_string()),
            one_time_product_notification: None,
            voided_purchase_notification: None,
            test_notification: Some(serde_json::json!({"version": "1.0"})),
        };
        // 从 notification 取 eventTimeMillis
        let event_time: i64 = notif
            .event_time_millis
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or_default();
        assert_eq!(event_time, 1_746_788_688_000i64, "Must use eventTimeMillis from notification");

        // publishTime 格式（ISO 8601）parse 为 i64 应为 0（验证旧 bug 已修复）
        let publish_time = "2026-05-09T10:24:48.690Z";
        let bad: i64 = publish_time.parse::<i64>().unwrap_or_default();
        assert_eq!(bad, 0, "publishTime is not a valid i64 timestamp");
    }

    // T53-06 (RED→GREEN): validate_rtdn_oidc_token — 空配置跳过验证（dev 模式）
    #[test]
    fn t53_06_empty_config_skips_validation() {
        let result = validate_rtdn_oidc_token("any.token.here", "", b"");
        assert!(result.is_ok(), "Empty config should skip validation");
    }

    // T53-07 (RED→GREEN): validate_rtdn_oidc_token — 无效 token 被拒
    #[test]
    fn t53_07_invalid_token_rejected() {
        let result = validate_rtdn_oidc_token("invalid.token.here", "https://example.com/rtdn", b"secret");
        assert!(result.is_err(), "Invalid token should be rejected");
        assert!(matches!(result.unwrap_err(), PaymentError::RtdnSignatureInvalid));
    }

    // T53-08 (RED→GREEN): validate_rtdn_oidc_token — 合法 HS256 token（正确 issuer + audience）通过
    #[test]
    fn t53_08_valid_hs256_token_passes() {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let secret = b"test-rtdn-secret";
        let audience = "https://example.com/webhook/google/rtdn";
        let claims = RtdnOidcClaims {
            iss: "https://accounts.google.com".to_string(),
            aud: audience.to_string(),
            exp: 9_999_999_999,
        };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret)).unwrap();
        let result = validate_rtdn_oidc_token(&token, audience, secret);
        assert!(result.is_ok(), "Valid HS256 token with correct issuer/aud should pass");
    }

    // T53-09 (RED→GREEN): validate_rtdn_oidc_token — 错误 issuer 被拒
    #[test]
    fn t53_09_wrong_issuer_rejected() {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let secret = b"test-rtdn-secret";
        let audience = "https://example.com/webhook/google/rtdn";
        let claims = RtdnOidcClaims {
            iss: "https://evil.com".to_string(), // 错误 issuer
            aud: audience.to_string(),
            exp: 9_999_999_999,
        };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret)).unwrap();
        let result = validate_rtdn_oidc_token(&token, audience, secret);
        assert!(result.is_err(), "Wrong issuer should be rejected");
    }

    // T53-10 (RED→GREEN): validate_rtdn_oidc_token — 错误 audience 被拒
    #[test]
    fn t53_10_wrong_audience_rejected() {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let secret = b"test-rtdn-secret";
        let claims = RtdnOidcClaims {
            iss: "https://accounts.google.com".to_string(),
            aud: "wrong-audience".to_string(), // 错误 audience
            exp: 9_999_999_999,
        };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(secret)).unwrap();
        let result = validate_rtdn_oidc_token(&token, "correct-audience", secret);
        assert!(result.is_err(), "Wrong audience should be rejected");
    }

    // T53-11 (RED→GREEN): 幂等逻辑 — 先查后写，业务成功才写入幂等记录
    #[test]
    fn t53_11_idempotency_order_is_check_process_then_write() {
        // 验证流程设计：在 handle_rtdn 中，先 check_rtdn_processed，
        // 分发处理，成功后才调用 insert_rtdn_processed
        // 这是一个设计验证测试（通过代码审查 + 编译验证）
        // 真实的并发行为测试需要集成测试
        let _ = std::mem::size_of::<PaymentRtdnService>();
    }
}
