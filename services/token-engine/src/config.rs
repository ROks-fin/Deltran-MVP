use config::{ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub nats: NatsConfig,
    pub token: TokenConfig,
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
pub struct TokenConfig {
    pub decimal_precision: u32,
    pub max_mint_amount: String,
    pub max_burn_amount: String,
    pub clearing_window_hours: i64,
    pub supported_currencies: Vec<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

        let mut builder = config::Config::builder()
            // Start with default configuration
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8081)?
            .set_default("server.workers", 4)?
            .set_default("database.max_connections", 10)?
            .set_default("database.min_connections", 2)?
            .set_default("redis.pool_size", 10)?
            .set_default("nats.topic_prefix", "deltran")?
            .set_default("nats.consumer_group", "token-engine")?
            .set_default("token.decimal_precision", 8)?
            .set_default("token.max_mint_amount", "1000000000")?
            .set_default("token.max_burn_amount", "1000000000")?
            .set_default("token.clearing_window_hours", 6)?
            .set_default(
                "token.supported_currencies",
                vec!["INR", "AED", "USD", "EUR", "GBP", "SAR", "OMR", "QAR", "KWD", "BHD"],
            )?;

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
            Environment::with_prefix("TOKEN_ENGINE")
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

        if let Ok(port) = env::var("TOKEN_ENGINE_PORT") {
            builder = builder.set_override("server.port", port)?;
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
            return Err("NATS URL is required".to_string());
        }

        if self.token.supported_currencies.is_empty() {
            return Err("At least one supported currency is required".to_string());
        }

        Ok(())
    }
}