use tokio::signal;
use voice_room_server::{
    bootstrap::build_app,
    infrastructure::{config::ServerSettings, logging::init_tracing},
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

    let app = build_app();
    let bind_addr = settings.server.bind_addr()?;

    tracing::info!(%bind_addr, "server skeleton initialized");

    axum::Server::bind(&bind_addr)
        .serve(app.into_make_service())
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
