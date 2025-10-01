//! Message publisher with retry logic

use crate::{
    client::NatsClient,
    message::Message,
    metrics::{MESSAGE_PUBLISH_DURATION, MESSAGE_PUBLISH_TOTAL},
    types::MessageType,
    Error, Result,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Publisher configuration
#[derive(Debug, Clone)]
pub struct PublisherConfig {
    /// Enable JetStream persistence
    pub use_jetstream: bool,

    /// Publish timeout
    pub publish_timeout: Duration,

    /// Max retry attempts
    pub max_retry_attempts: u32,

    /// Initial retry delay
    pub initial_retry_delay: Duration,

    /// Max retry delay
    pub max_retry_delay: Duration,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            use_jetstream: true,
            publish_timeout: Duration::from_secs(5),
            max_retry_attempts: 3,
            initial_retry_delay: Duration::from_millis(100),
            max_retry_delay: Duration::from_secs(2),
        }
    }
}

/// Message publisher
pub struct Publisher {
    client: Arc<NatsClient>,
    config: PublisherConfig,
}

impl Publisher {
    /// Create new publisher
    pub fn new(client: Arc<NatsClient>, config: PublisherConfig) -> Self {
        Self { client, config }
    }

    /// Publish message
    pub async fn publish(&self, message: &Message) -> Result<()> {
        let start = Instant::now();
        let subject = message.subject();

        info!(
            "Publishing message {} to subject: {}",
            message.id, subject
        );

        // Serialize message
        let payload = message.to_bytes()?;

        // Publish with retry
        let result = self.publish_with_retry(&subject, &payload, message.message_type).await;

        // Record metrics
        let duration = start.elapsed().as_secs_f64();
        MESSAGE_PUBLISH_DURATION
            .with_label_values(&[message.message_type.subject_prefix()])
            .observe(duration);

        let status = if result.is_ok() { "success" } else { "error" };
        MESSAGE_PUBLISH_TOTAL
            .with_label_values(&[message.message_type.subject_prefix(), status])
            .inc();

        result
    }

    /// Publish with exponential backoff retry
    async fn publish_with_retry(
        &self,
        subject: &str,
        payload: &[u8],
        message_type: MessageType,
    ) -> Result<()> {
        let mut attempts = 0;
        let mut delay = self.config.initial_retry_delay;

        loop {
            attempts += 1;

            match self.publish_once(subject, payload, message_type).await {
                Ok(_) => {
                    if attempts > 1 {
                        info!("✅ Message published after {} attempts", attempts);
                    }
                    return Ok(());
                }
                Err(e) => {
                    if attempts >= self.config.max_retry_attempts {
                        error!(
                            "❌ Failed to publish after {} attempts: {}",
                            attempts, e
                        );
                        return Err(e);
                    }

                    warn!(
                        "⚠️  Publish failed (attempt {}), retrying in {:?}: {}",
                        attempts, delay, e
                    );
                    tokio::time::sleep(delay).await;

                    // Exponential backoff
                    delay = (delay * 2).min(self.config.max_retry_delay);
                }
            }
        }
    }

    /// Single publish attempt
    async fn publish_once(
        &self,
        subject: &str,
        payload: &[u8],
        message_type: MessageType,
    ) -> Result<()> {
        if self.config.use_jetstream {
            // Publish to JetStream for persistence
            let js = self.client.jetstream().await?;

            // Ensure stream exists
            let stream_name = message_type.stream_name();
            self.client
                .get_or_create_stream(stream_name, vec![format!("{}.*", message_type.subject_prefix())])
                .await?;

            // Publish with acknowledgment
            let ack = js
                .publish(subject.to_string(), bytes::Bytes::copy_from_slice(payload))
                .await
                .map_err(|e| Error::NatsPublish(e.to_string()))?;

            // Wait for acknowledgment
            ack.await
                .map_err(|e| Error::JetStream(format!("Publish ack failed: {}", e)))?;
        } else {
            // Simple publish without persistence
            let client = self.client.client().await?;

            client
                .publish(subject.to_string(), bytes::Bytes::copy_from_slice(payload))
                .await
                .map_err(|e| Error::NatsPublish(e.to_string()))?;

            // Flush to ensure sent
            client
                .flush()
                .await
                .map_err(|e| Error::NatsPublish(format!("Flush failed: {}", e)))?;
        }

        Ok(())
    }

    /// Publish and wait for reply (request-reply pattern)
    pub async fn request(&self, message: &Message, timeout: Duration) -> Result<Message> {
        let subject = message.subject();
        let payload = message.to_bytes()?;

        let client = self.client.client().await?;

        let response = tokio::time::timeout(
            timeout,
            client.request(subject, bytes::Bytes::from(payload)),
        )
        .await
        .map_err(|_| Error::Timeout(timeout.as_millis() as u64))?
        .map_err(|e| Error::NatsPublish(e.to_string()))?;

        Message::from_bytes(&response.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{client::NatsConfig, types::PartitionKey};
    use serde_json::json;

    #[tokio::test]
    async fn test_publisher_creation() {
        let config = NatsConfig::default();
        let client = Arc::new(NatsClient::new(config));
        let pub_config = PublisherConfig::default();

        let publisher = Publisher::new(client, pub_config);
        assert!(publisher.config.use_jetstream);
    }

    #[tokio::test]
    async fn test_publish_config_default() {
        let config = PublisherConfig::default();
        assert_eq!(config.max_retry_attempts, 3);
        assert!(config.use_jetstream);
    }
}
