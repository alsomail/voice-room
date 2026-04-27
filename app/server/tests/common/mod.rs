//! 集成测试公共 helper（T-0000M）。
//!
//! 历史上 `app/server/tests/*.rs` 各自直接调用 `sqlx::migrate!("./migrations").run(&pool)`，
//! 在双服务共库（`voiceroom`）场景下会与 AdminServer 默认 `_sqlx_migrations` 表互掐。
//! 本 helper 是 TDS §2.3 要求的统一入口：
//!
//! - 复用 `voice_room_shared::migrate::run_migrations_with_table`
//! - 强制使用 AppServer 自定义登记表 `_sqlx_app_migrations`
//! - 任意测试只需 `mod common; common::run_migrations(&pool).await?;`
//!
//! 注：本文件被多个集成测试 target include，部分 target 仅用其中子集的 fn，
//! 因此 `#[allow(dead_code)]` 是必要的（cargo 把每个 *_test.rs 视作独立 crate）。
#![allow(dead_code)]

use sqlx::PgPool;
use voice_room_shared::migrate::MigrateTableError;

/// AppServer 集成测试统一迁移入口。表名固定 `_sqlx_app_migrations`，与 main.rs 一致。
pub async fn run_migrations(pool: &PgPool) -> Result<(), MigrateTableError> {
    voice_room_shared::migrate::run_migrations_with_table(
        pool,
        &sqlx::migrate!("./migrations"),
        "_sqlx_app_migrations",
    )
    .await
}
