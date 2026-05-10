//! Payment 路由注册（T-00051 / T-00052 / T-00053 / T-00055）
//!
//! - `GET /api/v1/payments/skus` — SKU 列表（公开）
//! - `POST /api/v1/payments/orders` — 创建订单（JWT 必选）
//! - `POST /api/v1/payments/google/verify` — 验签入账（JWT 必选）
//! - `POST /webhook/google/rtdn` — RTDN Webhook（无 JWT，Cloud Pub/Sub）
//! - `POST /api/v1/_dev/mock_recharge` — Dev Mock（仅 dev_payment_mock feature）

use axum::{
    routing::{get, post},
    Router,
};

use crate::bootstrap::AppState;

use super::controller::{
    create_order_handler, list_skus_handler, rtdn_webhook_handler, verify_handler,
};

/// 注册 Payment API 路由
pub fn payment_routes() -> Router<AppState> {
    #[cfg(not(feature = "dev_payment_mock"))]
    let router = Router::new()
        // T-00051: SKU 列表（公开，无需鉴权）
        .route("/api/v1/payments/skus", get(list_skus_handler))
        // T-00051: 创建订单（JWT 必选）
        .route("/api/v1/payments/orders", post(create_order_handler))
        // T-00052: Google 验签 + 入账（JWT 必选）
        .route("/api/v1/payments/google/verify", post(verify_handler))
        // T-00053: RTDN Webhook（Cloud Pub/Sub Push）
        .route("/webhook/google/rtdn", post(rtdn_webhook_handler));

    // T-00055: Dev Mock 充值（仅 dev_payment_mock feature 编译）
    #[cfg(feature = "dev_payment_mock")]
    let router = {
        let mut r = Router::new()
            .route("/api/v1/payments/skus", get(list_skus_handler))
            .route("/api/v1/payments/orders", post(create_order_handler))
            .route("/api/v1/payments/google/verify", post(verify_handler))
            .route("/webhook/google/rtdn", post(rtdn_webhook_handler));
        r = r.route(
            "/api/v1/_dev/mock_recharge",
            post(super::controller::mock_recharge_handler),
        );
        r
    };

    router
}
