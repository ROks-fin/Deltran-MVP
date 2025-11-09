use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub sanctions: SanctionsConfig,
    pub aml: AmlConfig,
    pub pep: PepConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub sanctions_ttl_hours: i64,
    pub pep_ttl_hours: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SanctionsConfig {
    pub auto_update_enabled: bool,
    pub update_interval_hours: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AmlConfig {
    pub low_risk_max: f64,
    pub medium_risk_max: f64,
    pub high_risk_min: f64,
    pub ctr_threshold_usd: f64,
    pub sar_risk_threshold: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PepConfig {
    pub include_family_members: bool,
    pub include_close_associates: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran".to_string());

        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let server_port = env::var("SERVICE_PORT")
            .unwrap_or_else(|_| "8086".to_string())
            .parse::<u16>()
            .unwrap_or(8086);

        Ok(Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: server_port,
            },
            database: DatabaseConfig {
                url: database_url,
            },
            redis: RedisConfig {
                url: redis_url,
                sanctions_ttl_hours: 24,
                pep_ttl_hours: 48,
            },
            sanctions: SanctionsConfig {
                auto_update_enabled: false,
                update_interval_hours: 6,
            },
            aml: AmlConfig {
                low_risk_max: 30.0,
                medium_risk_max: 60.0,
                high_risk_min: 60.0,
                ctr_threshold_usd: 10000.0,
                sar_risk_threshold: 70.0,
            },
            pep: PepConfig {
                include_family_members: true,
                include_close_associates: true,
            },
        })
    }
}
