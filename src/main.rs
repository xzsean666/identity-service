use std::net::SocketAddr;

use identity_service::{
    application::bootstrap::build_application_services, config::AppConfig, interfaces::http::router,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("configuration error: {error}");
            std::process::exit(1);
        }
    };

    let address: SocketAddr = format!("{}:{}", config.http.host, config.http.port)
        .parse()
        .expect("validated host and port must form a socket address");
    let services = build_application_services(config)
        .await
        .expect("application services must initialize");
    let app = router(services);

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("http listener must bind");
    tracing::info!("identity service listening on {address}");
    axum::serve(listener, app)
        .await
        .expect("http server must run");
}
