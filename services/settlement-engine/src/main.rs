mod accounts;
mod config;
mod error;
mod grpc;
mod integration;
mod recovery;
mod server;
mod settlement;

use config::Config;
use server::SettlementServer;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Settlement Engine starting...");

    // Load configuration
    let config = Config::from_env()?;

    info!(
        "Configuration loaded - gRPC port: {}, HTTP port: {}",
        config.server.grpc_port, config.server.http_port
    );

    // Create and start server
    let server = SettlementServer::new(config).await?;

    info!("Settlement Engine initialized successfully");

    server.start().await?;

    Ok(())
}
