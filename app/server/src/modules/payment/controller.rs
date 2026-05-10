//! Payment Controller — HTTP handlers
//!
//! - `create_order_handler` — POST /api/v1/payments/orders（T-00051）
//! - `list_skus_handler` — GET /api/v1/payments/skus（T-00051）
//! - `verify_handler` — POST /api/v1/payments/google/verify（T-00052）
//! - `rtdn_webhook_handler` — POST /webhook/google/rtdn（T-00053）
//! - `mock_recharge_handler` — POST /api/v1/_dev/mock_recharge（T-00055, dev only）

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

use crate::{
    bootstrap::AppState,
    common::{auth::AuthContext, response::ApiResponse, RequestContext},
};

use super::dto::{CreateOrderRequest, RtdnEnvelope, VerifyRequest};

// ─── SKU 列表（GET /api/v1/payments/skus）────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SkuListQuery {
    pub provider: Option<String>,
}

/// GET /api/v1/payments/skus
/// 公开接口，无需鉴权
pub async fn list_skus_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Query(query): Query<SkuListQuery>,
) -> axum::response::Response {
    let provider = query.provider.as_deref().unwrap_or("google_play");
    match state.payment_order_service.list_skus(provider).await {
        Ok(data) => Json(ApiResponse::ok(data, rc.request_id())).into_response(),
        Err(e) => e.with_request_id(rc.request_id()).into_response(),
    }
}

// ─── 创建订单（POST /api/v1/payments/orders）─────────────────────────────────

/// POST /api/v1/payments/orders（T-00051）
///
/// 严格匹配 payment_api.md §9.3.2
pub async fn create_order_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    auth: AuthContext,
    Json(req): Json<CreateOrderRequest>,
) -> axum::response::Response {
    let idempotency_key = None::<&str>; // TODO: 从 Idempotency-Key header 提取

    match state
        .payment_order_service
        .create_order(auth.user_id, &req.sku_id, &req.provider, idempotency_key)
        .await
    {
        Ok(data) => {
            tracing::info!(
                user_id = %auth.user_id,
                order_id = %data.order_id,
                sku_id = %req.sku_id,
                "payment.create_order success"
            );
            Json(ApiResponse::ok(data, rc.request_id())).into_response()
        }
        Err(e) => {
            tracing::warn!(
                user_id = %auth.user_id,
                sku_id = %req.sku_id,
                error = %e,
                "payment.create_order failed"
            );
            e.with_request_id(rc.request_id()).into_response()
        }
    }
}

// ─── Google 验签（POST /api/v1/payments/google/verify）───────────────────────

/// POST /api/v1/payments/google/verify（T-00052）
///
/// 严格匹配 payment_api.md §9.3.3
pub async fn verify_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    auth: AuthContext,
    Json(req): Json<VerifyRequest>,
) -> axum::response::Response {
    match state
        .payment_verify_service
        .verify_and_credit(
            auth.user_id,
            req.order_id,
            &req.purchase_token,
            req.provider_order_id.as_deref(),
        )
        .await
    {
        Ok(data) => Json(ApiResponse::ok(data, rc.request_id())).into_response(),
        Err(e) => e.with_request_id(rc.request_id()).into_response(),
    }
}

// ─── RTDN Webhook（POST /webhook/google/rtdn）────────────────────────────────

/// POST /webhook/google/rtdn（T-00053）
///
/// Cloud Pub/Sub Push 模式，不做 JWT 鉴权（依赖 VPC + Cloud Armor），
/// 仅验证 message.messageId 幂等
pub async fn rtdn_webhook_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    Json(envelope): Json<RtdnEnvelope>,
) -> axum::response::Response {
    match state.payment_rtdn_service.handle_rtdn(envelope).await {
        Ok(result) => {
            tracing::info!(
                outcome = %result.outcome,
                "RTDN processed: {}",
                result.message
            );
            Json(ApiResponse::ok(
                serde_json::json!({"outcome": result.outcome}),
                rc.request_id(),
            ))
            .into_response()
        }
        Err(e) => {
            // RTDN 处理错误返回 5xx，让 Pub/Sub 重试
            tracing::error!(error = %e, "RTDN processing error, Pub/Sub will retry");
            e.with_request_id(rc.request_id()).into_response()
        }
    }
}

// ─── Dev Mock 充值（POST /api/v1/_dev/mock_recharge）─────────────────────────

/// POST /api/v1/_dev/mock_recharge（T-00055）
///
/// 仅 dev_payment_mock feature 启用时编译
#[cfg(feature = "dev_payment_mock")]
pub async fn mock_recharge_handler(
    State(state): State<AppState>,
    Extension(rc): Extension<RequestContext>,
    auth: AuthContext,
    Json(req): Json<MockRechargeRequest>,
) -> axum::response::Response {
    match state
        .payment_mock_service
        .mock_recharge(auth.user_id, &req.sku_id, &req.force_outcome, req.client_note.as_deref())
        .await
    {
        Ok(data) => Json(ApiResponse::ok(data, rc.request_id())).into_response(),
        Err(e) => e.with_request_id(rc.request_id()).into_response(),
    }
}


