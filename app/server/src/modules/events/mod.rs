//! Events 模块 — T-00022 HTTP 批量接收埋点事件
//!
//! ## 功能
//! - `POST /api/v1/events/batch` — 批量接收，JWT 可选
//!
//! ## 路由
//! ```rust,ignore
//! .merge(events_routes())
//! ```

pub mod handler;

pub use routes::events_routes;

mod routes {
    use axum::{routing::post, Router};

    use crate::bootstrap::AppState;

    use super::handler::batch_events;

    pub fn events_routes() -> Router<AppState> {
        Router::new().route("/api/v1/events/batch", post(batch_events))
    }
}
