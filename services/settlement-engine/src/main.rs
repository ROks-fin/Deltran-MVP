mod accounts;
mod config;
mod error;
mod grpc;
mod integration;
mod recovery;
mod server;
mod settlement;
mod nats_consumer;

use config::Config;
use server::SettlementServer;
use tracing::{info, error};
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

    // Start NATS consumer for settlement execution
    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://localhost:4222".to_string());

    info!("💸 Starting NATS consumer for settlement execution...");
    if let Err(e) = nats_consumer::start_settlement_consumer(&nats_url).await {
        error!("Failed to start NATS consumer: {}", e);
        return Err(e);
    }
    info!("✅ NATS consumer started successfully");

    // Create and start server
    let server = SettlementServer::new(config).await?;

    info!("Settlement Engine initialized successfully");

    server.start().await?;

    Ok(())
}
