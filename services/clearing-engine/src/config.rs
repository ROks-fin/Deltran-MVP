use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub nats: NatsConfig,
    pub clearing: ClearingConfig,
    pub clients: ClientsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub http_port: u16,
    pub grpc_port: u16,
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    pub stream: String,
    pub durable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearingConfig {
    pub window_duration_hours: i64,
    pub grace_period_seconds: i64,
    pub max_obligations_per_window: u32,
    pub auto_settle: bool,
    pub min_netting_efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientsConfig {
    pub obligation_engine_url: String,
    pub settlement_engine_url: String,
    pub risk_engine_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran".to_string());

        let nats_url = env::var("NATS_URL")
            .unwrap_or_else(|_| "nats://nats:4222".to_string());

        Ok(Config {
            server: ServerConfig {
                http_port: env::var("HTTP_PORT")
                    .unwrap_or_else(|_| "8085".to_string())
                    .parse()?,
                grpc_port: env::var("GRPC_PORT")
                    .unwrap_or_else(|_| "50055".to_string())
                    .parse()?,
                host: env::var("HOST")
                    .unwrap_or_else(|_| "0.0.0.0".to_string()),
            },
            database: DatabaseConfig {
                url: database_url,
                max_connections: 20,
                min_connections: 5,
            },
            nats: NatsConfig {
                url: nats_url,
                stream: "CLEARING".to_string(),
                durable: "clearing-engine".to_string(),
            },
            clearing: ClearingConfig {
                window_duration_hours: 6,
                grace_period_seconds: 30,
                max_obligations_per_window: 10000,
                auto_settle: true,
                min_netting_efficiency: 0.5,
            },
            clients: ClientsConfig {
                obligation_engine_url: env::var("OBLIGATION_ENGINE_URL")
                    .unwrap_or_else(|_| "http://obligation-engine:50052".to_string()),
                settlement_engine_url: env::var("SETTLEMENT_ENGINE_URL")
                    .unwrap_or_else(|_| "http://settlement-engine:50056".to_string()),
                risk_engine_url: env::var("RISK_ENGINE_URL")
                    .unwrap_or_else(|_| "http://risk-engine:8084".to_string()),
            },
        })
    }
}
