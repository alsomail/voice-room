use axum::{
    routing::{get, post},
    Router,
};

use crate::bootstrap::AppState;

use super::controller::{get_me, login, send_code};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/send-code", post(send_code))
        .route("/auth/login", post(login))
        .route("/users/me", get(get_me))
}
