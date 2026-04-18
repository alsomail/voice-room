use std::sync::Arc;

use tokio::{net::TcpListener, signal};
use voice_room_server::{
    bootstrap::{build_app, AppState},
    infrastructure::{
        config::ServerSettings,
        database::create_pool,
        logging::init_tracing,
        redis_store::RedisCodeStore,
        third_party::sms::{MockSmsProvider, TwilioSmsProvider},
    },
    modules::auth::repository::PgUserRepository,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = ServerSettings::load()?;
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
        .expect("DATABASE_URL must be set");
    let pool = create_pool(db_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let redis_url = settings
        .redis_url
        .as_deref()
        .unwrap_or("redis://127.0.0.1:6379");
    let code_store = Arc::new(RedisCodeStore::new(redis_url)?);

    // 按环境选择 SMS provider（prod 用 Twilio，dev 用 Mock）
    let sms: Arc<dyn voice_room_server::infrastructure::third_party::sms::SmsProvider> =
        if settings.app.environment == "prod" {
            let sid = std::env::var("TWILIO_ACCOUNT_SID").expect("TWILIO_ACCOUNT_SID");
            let token = std::env::var("TWILIO_AUTH_TOKEN").expect("TWILIO_AUTH_TOKEN");
            let from = std::env::var("TWILIO_FROM_NUMBER").expect("TWILIO_FROM_NUMBER");
            Arc::new(TwilioSmsProvider::new(sid, token, from))
        } else {
            Arc::new(MockSmsProvider::default())
        };

    let state = AppState::new(
        Arc::new(PgUserRepository::new(pool)),
        code_store,
        sms,
        settings.jwt_secret.clone(),
    );

    let app = build_app(state);
    let bind_addr = settings.server.bind_addr()?;
    let listener = TcpListener::bind(bind_addr).await?;

    tracing::info!(%bind_addr, "server started");

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

    tracing::info!("shutdown signal received");
}
