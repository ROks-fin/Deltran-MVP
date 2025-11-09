use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub nats: NatsConfig,
    pub settlement: SettlementConfig,
    pub reconciliation: ReconciliationConfig,
    pub banks: BankConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub grpc_port: u16,
    pub http_port: u16,
    pub max_concurrent_settlements: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    pub subject_prefix: String,
    pub stream: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SettlementConfig {
    pub default_timeout_seconds: u64,
    pub max_retry_attempts: u32,
    pub retry_delay_seconds: u64,
    pub fund_lock_expiry_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReconciliationConfig {
    pub schedule_interval_hours: u64,
    pub tolerance_amount: String,  // Decimal
    pub alert_threshold: String,   // Decimal
}

#[derive(Debug, Clone, Deserialize)]
pub struct BankConfig {
    pub mock_enabled: bool,
    pub mock_latency_ms: u64,
    pub mock_success_rate: f64,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| {
                "postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran".to_string()
            });

        let nats_url = env::var("NATS_URL")
            .unwrap_or_else(|_| "nats://nats:4222".to_string());

        let grpc_port = env::var("GRPC_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(50056);

        let http_port = env::var("HTTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8086);

        Ok(Config {
            server: ServerConfig {
                grpc_port,
                http_port,
                max_concurrent_settlements: 100,
            },
            database: DatabaseConfig {
                url: database_url,
                max_connections: 20,
                min_connections: 5,
            },
            nats: NatsConfig {
                url: nats_url,
                subject_prefix: "settlement".to_string(),
                stream: "SETTLEMENT".to_string(),
            },
            settlement: SettlementConfig {
                default_timeout_seconds: 300,  // 5 minutes
                max_retry_attempts: 3,
                retry_delay_seconds: 60,
                fund_lock_expiry_seconds: 600,  // 10 minutes
            },
            reconciliation: ReconciliationConfig {
                schedule_interval_hours: 6,
                tolerance_amount: "0.01".to_string(),
                alert_threshold: "1000.00".to_string(),
            },
            banks: BankConfig {
                mock_enabled: true,
                mock_latency_ms: 500,
                mock_success_rate: 0.95,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        let config = Config::from_env().unwrap();
        assert_eq!(config.server.grpc_port, 50056);
        assert_eq!(config.server.http_port, 8086);
        assert!(config.banks.mock_enabled);
    }
}
