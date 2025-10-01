//! Configuration for consensus node

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Node ID
    pub node_id: String,

    /// Validator public key (hex)
    pub validator_pubkey: String,

    /// Validator power
    pub validator_power: u64,

    /// CometBFT configuration
    pub cometbft: CometBFTConfig,

    /// Ledger configuration
    pub ledger: LedgerConfig,

    /// Network configuration
    pub network: NetworkConfig,
}

/// CometBFT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CometBFTConfig {
    /// RPC listen address
    pub rpc_addr: String,

    /// P2P listen address
    pub p2p_addr: String,

    /// Home directory for CometBFT data
    pub home_dir: PathBuf,

    /// Chain ID
    pub chain_id: String,

    /// Consensus timeout commit (ms)
    pub timeout_commit: u64,

    /// Block time (ms)
    pub block_time: u64,

    /// Max block size (bytes)
    pub max_block_size: u64,
}

/// Ledger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerConfig {
    /// Data directory
    pub data_dir: PathBuf,

    /// Enable batching
    pub enable_batching: bool,

    /// Batch size
    pub batch_size: usize,

    /// Batch timeout (ms)
    pub batch_timeout_ms: u64,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Persistent peers
    pub persistent_peers: Vec<String>,

    /// Seeds
    pub seeds: Vec<String>,

    /// Private peer IDs
    pub private_peer_ids: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node_id: "node-1".to_string(),
            validator_pubkey: "".to_string(),
            validator_power: 10,
            cometbft: CometBFTConfig {
                rpc_addr: "tcp://0.0.0.0:26657".to_string(),
                p2p_addr: "tcp://0.0.0.0:26656".to_string(),
                home_dir: PathBuf::from("./data/cometbft"),
                chain_id: "deltran-1".to_string(),
                timeout_commit: 5000, // 5 seconds
                block_time: 6000,     // 6 seconds
                max_block_size: 22020096, // ~21 MB
            },
            ledger: LedgerConfig {
                data_dir: PathBuf::from("./data/ledger"),
                enable_batching: true,
                batch_size: 100,
                batch_timeout_ms: 10,
            },
            network: NetworkConfig {
                persistent_peers: vec![],
                seeds: vec![],
                private_peer_ids: vec![],
            },
        }
    }
}

impl Config {
    /// Load from file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| crate::Error::Config(format!("Failed to parse config: {}", e)))?;
        Ok(config)
    }

    /// Load from environment variables
    pub fn from_env() -> crate::Result<Self> {
        let mut config = Config::default();

        if let Ok(node_id) = std::env::var("CONSENSUS_NODE_ID") {
            config.node_id = node_id;
        }

        if let Ok(chain_id) = std::env::var("CONSENSUS_CHAIN_ID") {
            config.cometbft.chain_id = chain_id;
        }

        if let Ok(rpc_addr) = std::env::var("CONSENSUS_RPC_ADDR") {
            config.cometbft.rpc_addr = rpc_addr;
        }

        Ok(config)
    }
}

// Add toml dependency to workspace Cargo.toml
use toml;