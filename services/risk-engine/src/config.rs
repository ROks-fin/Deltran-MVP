use config::{ConfigError, Environment};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub risk: RiskConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RiskConfig {
    pub low_risk_threshold: f64,
    pub medium_risk_threshold: f64,
    pub high_risk_threshold: f64,
    pub failure_threshold: u32,
    pub recovery_threshold: u32,
    pub circuit_timeout_seconds: i64,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut builder = config::Config::builder()
            // Server defaults
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8084)?
            .set_default("server.workers", 4)?
            // Database defaults
            .set_default("database.url", "postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran")?
            .set_default("database.max_connections", 20)?
            // Redis defaults
            .set_default("redis.url", "redis://redis:6379")?
            .set_default("redis.ttl_seconds", 3600)?
            // Risk thresholds
            .set_default("risk.low_risk_threshold", 25.0)?
            .set_default("risk.medium_risk_threshold", 50.0)?
            .set_default("risk.high_risk_threshold", 75.0)?
            .set_default("risk.failure_threshold", 5)?
            .set_default("risk.recovery_threshold", 3)?
            .set_default("risk.circuit_timeout_seconds", 60)?;

        builder = builder.add_source(Environment::with_prefix("RISK_ENGINE").separator("__"));

        // Override from environment variables
        if let Ok(port) = env::var("SERVICE_PORT") {
            builder = builder.set_override("server.port", port)?;
        }

        if let Ok(db_url) = env::var("DATABASE_URL") {
            builder = builder.set_override("database.url", db_url)?;
        }

        if let Ok(redis_url) = env::var("REDIS_URL") {
            builder = builder.set_override("redis.url", redis_url)?;
        }

        builder.build()?.try_deserialize()
    }
}