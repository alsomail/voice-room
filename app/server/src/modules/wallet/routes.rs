//! 钱包模块路由注册

use axum::{routing::get, Router};

use crate::bootstrap::AppState;

use super::controller::{get_balance, list_transactions};

/// 注册钱包 API 路由
///
/// - `GET /api/v1/wallet/balance`        — 查询余额（JWT 必选）
/// - `GET /api/v1/wallet/transactions`   — 分页流水（JWT 必选）
pub fn wallet_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/wallet/balance", get(get_balance))
        .route("/api/v1/wallet/transactions", get(list_transactions))
}
