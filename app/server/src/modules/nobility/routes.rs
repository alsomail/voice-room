//! 贵族模块路由注册（T-00066 / T-00067）
//!
//! - GET  /api/v1/nobles/tiers       — 无需鉴权
//! - GET  /api/v1/nobles/me          — JWT 鉴权
//! - POST /api/v1/nobles/purchase    — JWT 鉴权
//! - PATCH /api/v1/nobles/me/auto_renew — JWT 鉴权

use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::bootstrap::AppState;

use super::controller::{get_me_handler, list_tiers_handler, purchase_handler, set_auto_renew_handler};

/// 注册贵族 API 路由
pub fn nobility_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/nobles/tiers", get(list_tiers_handler))
        .route("/api/v1/nobles/me", get(get_me_handler))
        .route("/api/v1/nobles/purchase", post(purchase_handler))
        .route("/api/v1/nobles/me/auto_renew", patch(set_auto_renew_handler))
}
