//! Configuration for the ledger

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Ledger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Data directory for RocksDB
    pub data_dir: PathBuf,

    /// Service name
    pub service_name: String,

    /// Service version
    pub service_version: String,

    /// gRPC listen address
    pub grpc_listen_addr: String,

    /// Metrics listen address
    pub metrics_listen_addr: String,

    /// RocksDB configuration
    pub rocksdb: RocksDBConfig,

    /// Batching configuration
    pub batching: BatchingConfig,

    /// Snapshot configuration
    pub snapshot: SnapshotConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data/ledger"),
            service_name: "ledger-core".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            grpc_listen_addr: "0.0.0.0:50051".to_string(),
            metrics_listen_addr: "0.0.0.0:9090".to_string(),
            rocksdb: RocksDBConfig::default(),
            batching: BatchingConfig::default(),
            snapshot: SnapshotConfig::default(),
        }
    }
}

/// RocksDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksDBConfig {
    /// Write buffer size (MB)
    pub write_buffer_size_mb: usize,

    /// Max write buffers
    pub max_write_buffer_number: i32,

    /// Target file size (MB)
    pub target_file_size_mb: u64,

    /// Max background jobs (compaction + flush)
    pub max_background_jobs: i32,

    /// Level 0 file num compaction trigger
    pub level0_file_num_compaction_trigger: i32,

    /// Enable statistics
    pub enable_statistics: bool,
}

impl Default for RocksDBConfig {
    fn default() -> Self {
        Self {
            write_buffer_size_mb: 256,      // 256 MB
            max_write_buffer_number: 4,
            target_file_size_mb: 256,       // 256 MB
            max_background_jobs: 4,
            level0_file_num_compaction_trigger: 4,
            enable_statistics: true,
        }
    }
}

/// Batching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchingConfig {
    /// Maximum batch size (events)
    pub max_batch_size: usize,

    /// Batch timeout (milliseconds)
    pub batch_timeout_ms: u64,

    /// Enable batching
    pub enabled: bool,
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,         // 100 events per batch
            batch_timeout_ms: 10,        // 10ms timeout
            enabled: true,
        }
    }
}

/// Snapshot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Snapshot every N blocks
    pub snapshot_interval_blocks: u64,

    /// S3 bucket for snapshots
    pub s3_bucket: Option<String>,

    /// Enable compression
    pub compress: bool,

    /// Compression level (zstd)
    pub compression_level: i32,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            snapshot_interval_blocks: 10_000, // Every 10k blocks (~3.5 days)
            s3_bucket: None,
            compress: true,
            compression_level: 3,              // Balanced compression
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

        if let Ok(data_dir) = std::env::var("LEDGER_DATA_DIR") {
            config.data_dir = PathBuf::from(data_dir);
        }

        if let Ok(addr) = std::env::var("LEDGER_GRPC_ADDR") {
            config.grpc_listen_addr = addr;
        }

        if let Ok(addr) = std::env::var("LEDGER_METRICS_ADDR") {
            config.metrics_listen_addr = addr;
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.service_name, "ledger-core");
        assert_eq!(config.grpc_listen_addr, "0.0.0.0:50051");
        assert!(config.batching.enabled);
    }
}