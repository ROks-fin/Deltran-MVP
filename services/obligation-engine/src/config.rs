use config::{ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub nats: NatsConfig,
    pub obligation: ObligationConfig,
    pub token_engine: TokenEngineConfig,
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
    pub min_connections: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NatsConfig {
    pub url: String,
    pub topic_prefix: String,
    pub consumer_group: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ObligationConfig {
    pub clearing_window_hours: i64,
    pub instant_settlement_threshold: String,  // Max amount for instant settlement
    pub netting_efficiency_target: f64,        // Target efficiency (0.7 = 70%)
    pub liquidity_confidence_threshold: f64,   // Min confidence for instant settlement
    pub max_corridor_exposure: String,         // Max exposure per corridor
    pub supported_corridors: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenEngineConfig {
    pub base_url: String,
    pub timeout_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

        let mut builder = config::Config::builder()
            // Start with default configuration
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8082)?
            .set_default("server.workers", 4)?
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 2)?
            .set_default("redis.pool_size", 10)?
            .set_default("nats.topic_prefix", "deltran")?
            .set_default("nats.consumer_group", "obligation-engine")?
            .set_default("obligation.clearing_window_hours", 6)?
            .set_default("obligation.instant_settlement_threshold", "100000")?
            .set_default("obligation.netting_efficiency_target", 0.7)?
            .set_default("obligation.liquidity_confidence_threshold", 0.75)?
            .set_default("obligation.max_corridor_exposure", "10000000")?
            .set_default(
                "obligation.supported_corridors",
                vec![
                    "INR-AED", "AED-INR", "USD-AED", "AED-USD", "INR-USD", "USD-INR",
                    "EUR-USD", "USD-EUR", "GBP-USD", "USD-GBP", "SAR-AED", "AED-SAR"
                ],
            )?
            .set_default("token_engine.base_url", "http://localhost:8081")?
            .set_default("token_engine.timeout_secs", 30)?;

        // Add environment-specific config file if it exists
        if let Ok(config_file) = env::var("CONFIG_FILE") {
            builder = builder.add_source(File::with_name(&config_file).required(false));
        } else {
            builder = builder.add_source(
                File::with_name(&format!("config/{}", environment)).required(false),
            );
        }

        // Override with environment variables
        builder = builder.add_source(
            Environment::with_prefix("OBLIGATION_ENGINE")
                .separator("__")
                .list_separator(","),
        );

        // Special handling for common env vars
        if let Ok(db_url) = env::var("DATABASE_URL") {
            builder = builder.set_override("database.url", db_url)?;
        }

        if let Ok(redis_url) = env::var("REDIS_URL") {
            builder = builder.set_override("redis.url", redis_url)?;
        }

        if let Ok(nats_url) = env::var("NATS_URL") {
            builder = builder.set_override("nats.url", nats_url)?;
        }

        if let Ok(port) = env::var("OBLIGATION_ENGINE_PORT") {
            builder = builder.set_override("server.port", port)?;
        }

        if let Ok(token_url) = env::var("TOKEN_ENGINE_URL") {
            builder = builder.set_override("token_engine.base_url", token_url)?;
        }

        builder.build()?.try_deserialize()
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate configuration
        if self.server.port == 0 {
            return Err("Server port cannot be 0".to_string());
        }

        if self.database.url.is_empty() {
            return Err("Database URL is required".to_string());
        }

        if self.redis.url.is_empty() {
            return Err("Redis URL is required".to_string());
        }

        if self.nats.url.is_empty() {
            return Err("NATS URL are required".to_string());
        }

        if self.obligation.supported_corridors.is_empty() {
            return Err("At least one supported corridor is required".to_string());
        }

        if self.obligation.netting_efficiency_target <= 0.0
            || self.obligation.netting_efficiency_target > 1.0
        {
            return Err("Netting efficiency target must be between 0 and 1".to_string());
        }

        Ok(())
    }
}