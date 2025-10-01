//! NATS client wrapper with connection management

use crate::{Error, Result};
use async_nats::jetstream::{self, stream::Config as StreamConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// NATS client configuration
#[derive(Debug, Clone)]
pub struct NatsConfig {
    /// NATS server URLs
    pub urls: Vec<String>,

    /// Connection name
    pub name: String,

    /// Max reconnect attempts (None = infinite)
    pub max_reconnect_attempts: Option<usize>,

    /// Reconnect delay
    pub reconnect_delay: Duration,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Enable JetStream
    pub enable_jetstream: bool,
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            urls: vec!["nats://localhost:4222".to_string()],
            name: "deltran".to_string(),
            max_reconnect_attempts: None,
            reconnect_delay: Duration::from_secs(2),
            connection_timeout: Duration::from_secs(5),
            enable_jetstream: true,
        }
    }
}

/// NATS client wrapper
pub struct NatsClient {
    config: NatsConfig,
    client: Arc<RwLock<Option<async_nats::Client>>>,
    jetstream: Arc<RwLock<Option<jetstream::Context>>>,
}

impl NatsClient {
    /// Create new NATS client
    pub fn new(config: NatsConfig) -> Self {
        Self {
            config,
            client: Arc::new(RwLock::new(None)),
            jetstream: Arc::new(RwLock::new(None)),
        }
    }

    /// Connect to NATS server
    pub async fn connect(&self) -> Result<()> {
        info!("Connecting to NATS servers: {:?}", self.config.urls);

        let options = async_nats::ConnectOptions::new()
            .name(&self.config.name)
            .connection_timeout(self.config.connection_timeout)
            .retry_on_initial_connect();

        let client = async_nats::connect_with_options(
            self.config.urls.join(","),
            options,
        )
        .await
        .map_err(|e| Error::NatsConnection(e.to_string()))?;

        info!("✅ Connected to NATS");

        // Store client
        *self.client.write().await = Some(client.clone());

        // Initialize JetStream if enabled
        if self.config.enable_jetstream {
            let js = jetstream::new(client);
            *self.jetstream.write().await = Some(js);
            info!("✅ JetStream initialized");
        }

        Ok(())
    }

    /// Get underlying NATS client
    pub async fn client(&self) -> Result<async_nats::Client> {
        self.client
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::NatsConnection("Not connected".to_string()))
    }

    /// Get JetStream context
    pub async fn jetstream(&self) -> Result<jetstream::Context> {
        self.jetstream
            .read()
            .await
            .clone()
            .ok_or_else(|| Error::JetStream("JetStream not initialized".to_string()))
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        self.client.read().await.is_some()
    }

    /// Disconnect from NATS
    pub async fn disconnect(&self) -> Result<()> {
        if let Some(client) = self.client.write().await.take() {
            // Flush pending messages
            client
                .flush()
                .await
                .map_err(|e| Error::NatsConnection(e.to_string()))?;

            info!("Disconnected from NATS");
        }

        *self.jetstream.write().await = None;
        Ok(())
    }

    /// Create or get JetStream stream
    pub async fn get_or_create_stream(
        &self,
        stream_name: &str,
        subjects: Vec<String>,
    ) -> Result<jetstream::stream::Stream> {
        let js = self.jetstream().await?;

        // Try to get existing stream
        match js.get_stream(stream_name).await {
            Ok(stream) => {
                info!("Using existing JetStream stream: {}", stream_name);
                Ok(stream)
            }
            Err(_) => {
                // Create new stream
                info!("Creating JetStream stream: {}", stream_name);

                let config = StreamConfig {
                    name: stream_name.to_string(),
                    subjects,
                    max_messages: 1_000_000,
                    max_bytes: 1_073_741_824, // 1 GB
                    max_age: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
                    retention: jetstream::stream::RetentionPolicy::Limits,
                    storage: jetstream::stream::StorageType::File,
                    num_replicas: 1,
                    ..Default::default()
                };

                js.create_stream(config)
                    .await
                    .map_err(|e| Error::JetStream(e.to_string()))
            }
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        let client = self.client().await?;

        // Try to publish to a test subject
        client
            .publish("deltran.health".to_string(), bytes::Bytes::from("ping"))
            .await
            .map_err(|e| Error::NatsConnection(format!("Health check failed: {}", e)))?;

        client
            .flush()
            .await
            .map_err(|e| Error::NatsConnection(format!("Flush failed: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nats_config_default() {
        let config = NatsConfig::default();
        assert_eq!(config.urls.len(), 1);
        assert_eq!(config.name, "deltran");
        assert!(config.enable_jetstream);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = NatsConfig::default();
        let client = NatsClient::new(config);
        assert!(!client.is_connected().await);
    }
}
