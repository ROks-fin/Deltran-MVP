//! Consensus Node Binary
//!
//! Runs a DelTran consensus node with CometBFT integration.

use consensus::{Config, LedgerApp, Result};
use ledger_core::{Config as LedgerConfig, Ledger};
use std::sync::Arc;
use tendermint_abci::Server;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("Starting DelTran Consensus Node");

    // Load configuration
    let config = if let Ok(config_path) = std::env::var("CONSENSUS_CONFIG") {
        info!("Loading config from: {}", config_path);
        Config::from_file(&config_path)?
    } else {
        info!("Loading config from environment variables");
        Config::from_env()?
    };

    info!(
        "Node ID: {}, Chain ID: {}",
        config.node_id, config.cometbft.chain_id
    );

    // Initialize ledger
    let ledger_config = LedgerConfig {
        data_dir: config.ledger.data_dir.clone(),
        batching: ledger_core::BatchingConfig {
            enabled: config.ledger.enable_batching,
            max_batch_size: config.ledger.batch_size,
            batch_timeout: std::time::Duration::from_millis(config.ledger.batch_timeout_ms),
        },
        rocksdb: Default::default(),
    };

    info!("Opening ledger at: {:?}", ledger_config.data_dir);
    let ledger = Arc::new(Ledger::open(ledger_config).await?);

    // Create ABCI application
    info!("Creating ABCI application");
    let app = LedgerApp::new(ledger).await?;

    // Parse ABCI listen address
    let abci_addr = config
        .cometbft
        .rpc_addr
        .trim_start_matches("tcp://")
        .parse()
        .map_err(|e| consensus::Error::Config(format!("Invalid RPC address: {}", e)))?;

    info!("Starting ABCI server on {}", abci_addr);

    // Start ABCI server
    let server = Server::new(abci_addr);

    // Spawn server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run(app).await {
            error!("ABCI server error: {}", e);
        }
    });

    info!("Consensus node running");
    info!("- ABCI: {}", config.cometbft.rpc_addr);
    info!("- P2P: {}", config.cometbft.p2p_addr);
    info!("- Chain: {}", config.cometbft.chain_id);

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
        }
        Err(err) => {
            error!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // Graceful shutdown
    info!("Shutting down consensus node...");
    server_handle.abort();

    info!("Consensus node stopped");
    Ok(())
}