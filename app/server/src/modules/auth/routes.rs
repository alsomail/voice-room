use axum::{
    routing::{get, post},
    Router,
};

use crate::bootstrap::AppState;

use super::controller::{get_me, login, send_code};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/verification-codes", post(send_code))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/users/me", get(get_me))
}
