use std::sync::Arc;

use tokio::{net::TcpListener, signal};
use voice_room_server::{
    bootstrap::{build_app, AppState},
    core::analytics::{scheduler::start_partition_scheduler, writer::EventWriter},
    infrastructure::{
        config::ServerSettings,
        database::create_pool,
        logging::init_tracing,
        redis_store::RedisCodeStore,
        third_party::sms::{MockSmsProvider, TwilioSmsProvider},
    },
    modules::{
        auth::repository::PgUserRepository,
        gift::{send_gift::GiftSendService, service::GiftService},
        governance::{
            kick::{RealKickAuditDb, RealKickRedis},
            mute::{RealMuteDb, RealMuteRedis},
            transfer::RealTransferAdminRepo,
        },
        ranking::service::RankingService,
        room::{password::RealRoomPasswordRedis, repository::PgRoomRepository},
        wallet::{broadcaster::BalanceBroadcaster, service::WalletService},
    },
    room::mic_lock::RealMicLock,
    stats::{snapshot_task::start_snapshot_task, StatsService},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = ServerSettings::load().unwrap_or_else(|e| fatal_config(e));
    init_tracing(&settings.log)?;

    let startup_span = tracing::info_span!(
        "server_bootstrap",
        request_id = tracing::field::Empty,
        service_name = %settings.log.service_name,
        environment = %settings.app.environment,
        host = %settings.server.host,
        port = settings.server.port
    );
    let _startup_guard = startup_span.enter();

    let db_url = settings
        .database
        .url
        .as_deref()
        .expect("DATABASE_URL injected by ServerSettings::load()");
    let pool = create_pool(
        db_url,
        settings.database.max_connections,
        settings.database.connect_timeout_secs,
    )
    .await?;
    // T-0000M：双服务共库 Migration 表隔离 — 使用自定义登记表 `_sqlx_app_migrations`，
    // 避免与 AdminServer 默认 `_sqlx_migrations` 互相覆盖／校验互掐。
    // 详见 doc/tds/infra/T-0000M.md §2.2（保底方案：手管登记表 SQL，复用宏的 Migration 列表）。
    voice_room_shared::migrate::run_migrations_with_table(
        &pool,
        &sqlx::migrate!("./migrations"),
        "_sqlx_app_migrations",
    )
    .await?;

    let redis_url = settings
        .redis_url
        .as_deref()
        .expect("REDIS_URL injected by ServerSettings::load()");
    let code_store = Arc::new(RedisCodeStore::new(redis_url).await?);
    let stats_service = Arc::new(StatsService::new(redis_url).await?);

    // 按环境选择 SMS provider（prod 用 Twilio，dev 用 Mock）
    let sms: Arc<dyn voice_room_server::infrastructure::third_party::sms::SmsProvider> =
        if settings.app.environment == "prod" {
            let sid = std::env::var("TWILIO_ACCOUNT_SID").expect("TWILIO_ACCOUNT_SID");
            let token = std::env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN");
            let from = std::env::var("TWILIO_FROM_NUMBER").expect("TWILIO_FROM_NUMBER");
            Arc::new(TwilioSmsProvider::new(sid, token, from))
        } else {
            Arc::new(MockSmsProvider)
        };

    // 创建 BalanceBroadcaster channel
    let (balance_tx, balance_rx) = tokio::sync::mpsc::channel::<
        voice_room_server::modules::wallet::broadcaster::BalanceEvent,
    >(256);
    let wallet_service = Arc::new(WalletService::new(pool.clone(), balance_tx.clone()));
    let gift_service = Arc::new(GiftService::new_with_pool(pool.clone()));

    // 创建 GiftSendService（T-00020）
    let room_manager = Arc::new(voice_room_server::room::RoomManager::new());
    let ws_registry = Arc::new(voice_room_server::ws::ConnectionRegistry::new());
    let send_gift_service = Arc::new(GiftSendService::new(
        pool.clone(),
        ws_registry.clone(),
        room_manager.clone(),
        balance_tx,
        redis_url.to_string(),
    )?);

    // 创建 RankingService（T-00021）
    let ranking_service = Arc::new(RankingService::new(pool.clone(), redis_url.to_string()));

    // 创建 EventWriter（T-00022）
    let event_writer = Arc::new(EventWriter::new(pool.clone()));

    // 创建 RealRoomPasswordRedis（T-00026 密码房校验）
    let room_password_redis = Arc::new(RealRoomPasswordRedis::new(redis_url)?);

    // ── R1 P0-1 治理模块真实仓储装配（启动期 fail-fast：Redis client open 失败直接退出）
    let kick_redis = Arc::new(RealKickRedis::new(redis_url)?);
    let kick_audit_db = Arc::new(RealKickAuditDb::new(pool.clone()));
    let mute_redis = Arc::new(RealMuteRedis::new(redis_url)?);
    let mute_db = Arc::new(RealMuteDb::new(pool.clone()));
    let transfer_admin_repo = Arc::new(RealTransferAdminRepo::new(pool.clone()));
    let mic_lock = Arc::new(RealMicLock::new(redis_url)?);

    let state = AppState::new_with_managers(
        Arc::new(PgUserRepository::new(pool.clone())),
        code_store,
        sms,
        settings.jwt_secret.clone(),
        Arc::new(PgRoomRepository::new(pool.clone())),
        stats_service,
        wallet_service,
        gift_service,
        send_gift_service,
        ranking_service,
        ws_registry,
        room_manager,
        event_writer,
        // 治理 / 密码房 / 抢麦锁真实仓储
        room_password_redis,
        kick_redis,
        kick_audit_db,
        mute_redis,
        mute_db,
        mic_lock,
        transfer_admin_repo,
        Arc::new(voice_room_server::modules::chat::RealChatRepository::new(
            pool.clone(),
        )),
    );

    // 启动 BalanceBroadcaster（HIGH-2：同时监听本进程 mpsc channel 和 Redis PubSub）
    let broadcaster = BalanceBroadcaster::new(state.ws_registry.clone());
    let (snapshot_shutdown_tx, snapshot_shutdown_rx) = tokio::sync::watch::channel(false);
    let broadcaster_shutdown = snapshot_shutdown_tx.subscribe();
    tokio::spawn(broadcaster.run_with_redis(
        balance_rx,
        redis_url.to_string(),
        broadcaster_shutdown,
    ));

    // spawn 週期快照 task（每 60s 寫一次 Redis snapshot）
    let stats_for_snapshot = state.stats_service.clone();
    tokio::spawn(start_snapshot_task(
        stats_for_snapshot,
        snapshot_shutdown_rx,
    ));

    // 启动 Ranking 定时归档任务（T-00021）
    let ranking_shutdown = snapshot_shutdown_tx.subscribe();
    voice_room_server::modules::ranking::scheduler::start_ranking_scheduler(
        redis_url.to_string(),
        ranking_shutdown,
    );

    // 启动 Partition 定时创建任务（T-00022）
    let partition_shutdown = snapshot_shutdown_tx.subscribe();
    start_partition_scheduler(pool, partition_shutdown);

    // T-00041：启动 WebSocket 心跳后台 task（每 5s 扫描，30s 静默主动 Close(1000)）
    let heartbeat_shutdown = snapshot_shutdown_tx.subscribe();
    tokio::spawn(voice_room_server::ws::heartbeat::heartbeat_task(
        state.ws_registry.clone(),
        heartbeat_shutdown,
    ));

    let app = build_app(state);
    let bind_addr = settings.server.bind_addr()?;
    let listener = TcpListener::bind(bind_addr).await?;

    tracing::info!(%bind_addr, "server started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // 通知 snapshot_task / ranking_scheduler 停止（優雅停機）
    let _ = snapshot_shutdown_tx.send(true);

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

    tracing::info!("shutdown signal received");
}

/// 配置加载失败时的 fail-fast：统一前缀 `CONFIG ERROR:` + 退出码 78（EX_CONFIG）。
fn fatal_config(err: anyhow::Error) -> ! {
    eprintln!("CONFIG ERROR: {err:#}");
    std::process::exit(78);
}
