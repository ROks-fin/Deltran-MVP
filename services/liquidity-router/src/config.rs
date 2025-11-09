use config::{ConfigError, Environment};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
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
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut builder = config::Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8083)?
            .set_default("server.workers", 4)?
            .set_default("database.max_connections", 10)?;

        builder = builder.add_source(Environment::with_prefix("LIQUIDITY_ROUTER").separator("__"));

        if let Ok(db_url) = env::var("DATABASE_URL") {
            builder = builder.set_override("database.url", db_url)?;
        }

        if let Ok(redis_url) = env::var("REDIS_URL") {
            builder = builder.set_override("redis.url", redis_url)?;
        }

        if let Ok(port) = env::var("LIQUIDITY_ROUTER_PORT") {
            builder = builder.set_override("server.port", port)?;
        }

        builder.build()?.try_deserialize()
    }
}