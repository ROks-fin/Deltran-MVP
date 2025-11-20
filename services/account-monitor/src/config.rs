// Configuration for Account Monitor service

use serde::Deserialize;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_port: u16,
    pub database_url: String,
    pub nats_url: String,
    pub monitored_accounts: Vec<AccountConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountConfig {
    pub account_id: String,
    pub currency: String,
    pub api_type: String, // "REST" or "ISO20022"
    pub api_endpoint: String,
    pub api_key: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        let server_port = env::var("ACCOUNT_MONITOR_PORT")
            .unwrap_or_else(|_| "8090".to_string())
            .parse()
            .expect("ACCOUNT_MONITOR_PORT must be a valid port number");

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        let nats_url = env::var("NATS_URL")
            .unwrap_or_else(|_| "nats://localhost:4222".to_string());

        // Load monitored accounts from environment
        let monitored_accounts = Self::load_monitored_accounts();

        Self {
            server_port,
            database_url,
            nats_url,
            monitored_accounts,
        }
    }

    fn load_monitored_accounts() -> Vec<AccountConfig> {
        // Load from environment variable (JSON array)
        // Example: MONITORED_ACCOUNTS='[{"account_id":"AE070331234567890123456","currency":"AED","api_type":"REST","api_endpoint":"https://api.bank1.ae","api_key":"key1"}]'

        let accounts_json = env::var("MONITORED_ACCOUNTS")
            .unwrap_or_else(|_| {
                // Default configuration for development
                r#"[
                    {
                        "account_id": "AE070331234567890123456",
                        "currency": "AED",
                        "api_type": "REST",
                        "api_endpoint": "https://api.example-bank.ae",
                        "api_key": "dev_key_aed"
                    },
                    {
                        "account_id": "IL123456789012345678901",
                        "currency": "ILS",
                        "api_type": "ISO20022",
                        "api_endpoint": "https://api.example-bank.il",
                        "api_key": "dev_key_ils"
                    },
                    {
                        "account_id": "US12345678901234567890",
                        "currency": "USD",
                        "api_type": "REST",
                        "api_endpoint": "https://api.example-bank.us",
                        "api_key": "dev_key_usd"
                    }
                ]"#.to_string()
            });

        serde_json::from_str(&accounts_json)
            .expect("Failed to parse MONITORED_ACCOUNTS JSON")
    }
}
