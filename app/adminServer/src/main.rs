use std::sync::Arc;

use tokio::{net::TcpListener, signal};
use voice_room_admin_server::{
    bootstrap::{build_app, AppState},
    modules::{
        audit::repository::PgAuditRepository,
        auth::{PgAdminLogRepository, PgAdminRepository},
        event::{
            publisher::{EventPublisher, NoopEventPublisher, RedisEventPublisher},
            PgEventQueryRepository,
        },
        gift::repo::PgGiftRepository,
        governance::repo::PgGovernanceRepo,
        room::PgAdminRoomRepository,
        stats::PgAdminStatsRepository,
        user::PgAdminUserRepository,
        wallet::repository::PgWalletRepository,
    },
};

/// Admin Server 入口。
///
/// 读取环境变量（支持 .env 文件）→ 初始化 PgPool → 注入真实 Repository →
/// 构建 Axum Router → 启动 HTTP 监听。
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载 .env 文件（文件不存在时忽略，生产环境依赖真实环境变量）
    let _ = dotenvy::dotenv();

    // 初始化 tracing（默认输出到 stdout，RUST_LOG 控制级别）
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "voice_room_admin_server=info,tower_http=debug".into()),
        )
        .init();

    // 读取必要环境变量
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse()
        .expect("PORT must be a valid u16");

    // 初始化 PostgreSQL 连接池
    let pool = sqlx::PgPool::connect(&database_url).await?;
    tracing::info!("database connected");

    // 运行数据库迁移
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("migrations applied");

    // 初始化 Redis 事件发布器（若未配置则降级为 Noop）
    let event_publisher: Arc<dyn EventPublisher> = match std::env::var("REDIS_URL") {
        Ok(url) => {
            let client = redis::Client::open(url)?;
            tracing::info!("redis event publisher connected");
            Arc::new(RedisEventPublisher::new(client))
        }
        Err(_) => {
            tracing::warn!("REDIS_URL not set, using NoopEventPublisher (events will not be published)");
            Arc::new(NoopEventPublisher::default())
        }
    };

    // 构建 AppState（注入真实 PgRepository）
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
        Arc::new(PgGovernanceRepo::new(pool)),
    );

    let app = build_app(state);
    let bind_addr = format!("0.0.0.0:{port}");
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
