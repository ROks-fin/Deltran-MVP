//! Ledger gRPC server binary

use ledger_core::{Config, Ledger};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting DelTran Ledger Server");

    // Load configuration
    let config = Config::default();

    // Open ledger
    let ledger = Ledger::open(config).await?;
    tracing::info!("Ledger opened successfully");

    // TODO: Start gRPC server here
    // For now, just keep running
    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down ledger server");
    Ok(())
}
