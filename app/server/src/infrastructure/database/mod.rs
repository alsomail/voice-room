use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::common::error::AppError;

pub async fn create_pool(database_url: &str) -> Result<PgPool, AppError> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .map_err(|e| AppError::Internal(format!("db pool: {e}")))
}
