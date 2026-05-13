use std::{collections::HashMap, sync::Arc};

use tokio::{net::TcpListener, signal};
use voice_room_admin_server::{
    bootstrap::{build_app, AppState},
    infrastructure::config::AdminSettings,
    modules::{
        audit::repository::PgAuditRepository,
        auth::{PgAdminLogRepository, PgAdminRepository},
        event::{
            publisher::{EventPublisher, NoopEventPublisher, RedisEventPublisher},
            PgEventQueryRepository,
        },
        gift::repo::PgGiftRepository,
        governance::repo::PgGovernanceRepo,
        nobility::repository::PgNobilityRepo,
        payment::{
            admin_service::PgPaymentAdminRepository,
            repo::PgPaymentOrderRepo,
            report_query::PgReportQuery,
            report_service::ExchangeRates,
            sku_repo::PgSkuRepository,
        },
        room::PgAdminRoomRepository,
        stats::PgAdminStatsRepository,
        user::PgAdminUserRepository,
        wallet::repository::PgWalletRepository,
    },
};

/// 启动期 config 失败统一处理：stderr 首行 `CONFIG ERROR:`，退出码 78（EX_CONFIG）。
/// 与 T-00040 / preflight.sh / RUNBOOK 形成 grep 锚点（T-10020 §2.5）。
fn fatal_config(err: anyhow::Error) -> ! {
    eprintln!("CONFIG ERROR: {err:#}");
    std::process::exit(78);
}

/// Admin Server 入口。
///
/// 加载链：`AdminSettings::load()`（dotenv → default.toml → {profile}.toml → ENV）→
/// 注入 PgPool → Repository → 构建 Axum Router → 启动 HTTP 监听。
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化 tracing（默认输出到 stdout，RUST_LOG 控制级别）
    // 注：load() 内会用 tracing::warn!/info!，所以 subscriber 必须先于 load 初始化。
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "voice_room_admin_server=info,tower_http=debug".into()),
        )
        .init();

    // 9 步加载链 + fail-fast（任一字段缺失即 exit 78）
    let settings = AdminSettings::load().unwrap_or_else(|e| fatal_config(e));

    let database_url = settings
        .database
        .url
        .clone()
        .expect("DATABASE_URL injected by AdminSettings::load() (load fail-fast 兜底)");
    let jwt_secret = settings.jwt_secret.clone();
    let port = settings.server.port;
    let host = settings.server.host.clone();

    // 初始化 PostgreSQL 连接池
    let pool = sqlx::PgPool::connect(&database_url).await?;
    tracing::info!("database connected");

    // T-0000M：双服务共库 Migration 表隔离 — 使用自定义登记表 `_sqlx_admin_migrations`，
    // 避免与 AppServer 默认 `_sqlx_migrations` 互相覆盖／校验互掐。
    // 详见 doc/tds/infra/T-0000M.md §2.2。
    voice_room_shared::migrate::run_migrations_with_table(
        &pool,
        &sqlx::migrate!("./migrations"),
        "_sqlx_admin_migrations",
    )
    .await?;
    tracing::info!("migrations applied");

    // 初始化 Redis 事件发布器（dev 允许缺失 → Noop；其他 profile load 阶段已 fail-fast）
    let event_publisher: Arc<dyn EventPublisher> = match settings.redis_url.as_deref() {
        Some(url) => {
            let client = redis::Client::open(url)?;
            tracing::info!("redis event publisher connected");
            Arc::new(RedisEventPublisher::new(client))
        }
        None => {
            tracing::warn!(
                "REDIS_URL not set, using NoopEventPublisher (events will not be published)"
            );
            Arc::new(NoopEventPublisher::default())
        }
    };

    // 构建 AppState（注入真实 PgRepository）
    let pool_nobility = pool.clone();
    let pool_payment_order = pool.clone();
    let pool_payment_admin = pool.clone();
    let pool_sku = pool.clone();
    let pool_report = pool.clone();
    let pool_governance = pool.clone();
    let state = AppState::new(
        Arc::new(PgAdminRepository::new(pool.clone())),
        Arc::new(PgAdminLogRepository::new(pool.clone())),
        Arc::new(PgAdminRoomRepository::new(pool.clone())),
        Arc::new(PgAdminUserRepository::new(pool.clone())),
        Arc::new(PgAdminStatsRepository::new(pool.clone())),
        jwt_secret,
        event_publisher,
        Arc::new(PgAuditRepository::new(pool.clone())),
        Arc::new(PgWalletRepository::new(pool.clone())),
        Arc::new(PgGiftRepository::new(pool.clone())),
        Arc::new(PgEventQueryRepository::new(pool.clone())),
        Arc::new(PgGovernanceRepo::new(pool_governance)),
        Arc::new(PgPaymentOrderRepo::new(pool_payment_order)),
        Arc::new(PgPaymentAdminRepository::new(pool_payment_admin)),
        Arc::new(PgSkuRepository::new(pool_sku)),
        Arc::new(PgReportQuery::new(pool_report)),
        ExchangeRates(HashMap::new()),
        Arc::new(PgNobilityRepo::new(pool_nobility)),
    );

    let app = build_app(state.clone());

    // P0-3: 若提供 REDIS_URL，则注入 AdminStatsService 以读取实时 online_users / active_rooms
    if let Some(url) = settings.redis_url.as_deref() {
        state.stats_service.try_init_redis(url).await;
    }

    let bind_addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&bind_addr).await?;

    tracing::info!(addr = %bind_addr, "admin server started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, starting graceful shutdown");
}
