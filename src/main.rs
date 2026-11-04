use identity_service::{
    api::create_router,
    config::Config,
    db::run_migrations,
    providers::ProviderRegistry,
    services::token::JwtEngine,
    AppState,
};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "identity_service=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let addr = SocketAddr::new(
        config.host.parse()?,
        config.port,
    );

    let pool = identity_service::db::init_pool_with_options(
        &config.database_url,
        config.database_max_connections,
        config.database_min_connections,
    )
    .await?;
    run_migrations(&pool).await?;

    // Background maintenance worker: periodically cleans up expired/revoked refresh tokens
    let cleanup_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
        loop {
            interval.tick().await;
            match identity_service::db::cleanup_expired_refresh_tokens(&cleanup_pool).await {
                Ok(count) if count > 0 => {
                    tracing::info!(cleaned_tokens = count, "Cleaned up expired refresh tokens");
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!(error = %err, "Failed to clean up expired refresh tokens");
                }
            }
        }
    });

    let jwt_engine = JwtEngine::new(&config)?;
    let providers = ProviderRegistry::from_config(&config);
    let state = AppState::new(pool, jwt_engine, config, providers);

    let app = create_router(state);

    tracing::info!("Starting identity-service on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully");

    Ok(())
}
