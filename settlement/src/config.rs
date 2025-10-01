//! Configuration for settlement engine

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Settlement engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Service name
    pub service_name: String,

    /// Service version
    pub service_version: String,

    /// Ledger data directory
    pub ledger_data_dir: PathBuf,

    /// Settlement window configuration
    pub window: WindowConfig,

    /// Netting configuration
    pub netting: NettingConfig,

    /// ISO 20022 output configuration
    pub iso20022: Iso20022Config,

    /// Metrics listen address
    pub metrics_listen_addr: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            service_name: "settlement-engine".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            ledger_data_dir: PathBuf::from("./data/ledger"),
            window: WindowConfig::default(),
            netting: NettingConfig::default(),
            iso20022: Iso20022Config::default(),
            metrics_listen_addr: "0.0.0.0:9091".to_string(),
        }
    }
}

/// Settlement window configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window duration in seconds (default: 6 hours = 21600s)
    pub duration_seconds: u64,

    /// Window schedule (cron format)
    /// Default: "0 0 0,6,12,18 * * *" = 00:00, 06:00, 12:00, 18:00 UTC
    pub schedule: String,

    /// Minimum payments for settlement
    pub min_payments: usize,

    /// Maximum payments per window
    pub max_payments: usize,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            duration_seconds: 21600, // 6 hours
            schedule: "0 0 0,6,12,18 * * *".to_string(),
            min_payments: 10,
            max_payments: 10000,
        }
    }
}

/// Netting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NettingConfig {
    /// Minimum netting ratio (0.0 - 1.0)
    /// If actual netting < min_ratio, skip netting
    pub min_netting_ratio: f64,

    /// Maximum iterations for min-cost flow
    pub max_iterations: usize,

    /// Enable bilateral netting optimization
    pub enable_bilateral_optimization: bool,
}

impl Default for NettingConfig {
    fn default() -> Self {
        Self {
            min_netting_ratio: 0.2, // 20% reduction minimum
            max_iterations: 1000,
            enable_bilateral_optimization: true,
        }
    }
}

/// ISO 20022 output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iso20022Config {
    /// Output directory for ISO 20022 files
    pub output_dir: PathBuf,

    /// Message type (pacs.008 = FIToFI Customer Credit Transfer)
    pub message_type: String,

    /// Sender BIC
    pub sender_bic: String,

    /// Pretty print XML
    pub pretty_print: bool,
}

impl Default for Iso20022Config {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./data/settlement/iso20022"),
            message_type: "pacs.008.001.08".to_string(),
            sender_bic: "DELTRAEAD".to_string(), // DelTran ADGM
            pretty_print: true,
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

        if let Ok(dir) = std::env::var("SETTLEMENT_LEDGER_DIR") {
            config.ledger_data_dir = PathBuf::from(dir);
        }

        if let Ok(schedule) = std::env::var("SETTLEMENT_SCHEDULE") {
            config.window.schedule = schedule;
        }

        if let Ok(addr) = std::env::var("SETTLEMENT_METRICS_ADDR") {
            config.metrics_listen_addr = addr;
        }

        Ok(config)
    }
}