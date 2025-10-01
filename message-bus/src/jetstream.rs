//! JetStream integration for persistent messaging
//!
//! Provides exactly-once delivery semantics with:
//! - Persistent streams per corridor
//! - Consumer groups for load balancing
//! - Dead Letter Queue for failed messages
//! - Deduplication via idempotency keys

use async_nats::jetstream::{
    consumer::PullConsumer,
    stream::{Config as StreamConfig, RetentionPolicy, StorageType},
    Context as JetStreamContext,
};
use std::time::Duration;
use tracing::{info, warn, error};

use crate::{Error, Result, PartitionKey};

/// JetStream manager for DelTran
pub struct JetStreamManager {
    context: JetStreamContext,
    stream_prefix: String,
}

impl JetStreamManager {
    /// Create new JetStream manager
    pub async fn new(nats_url: &str, stream_prefix: &str) -> Result<Self> {
        info!("Connecting to NATS JetStream at {}", nats_url);

        let client = async_nats::connect(nats_url)
            .await
            .map_err(|e| Error::Connection(e.to_string()))?;

        let context = async_nats::jetstream::new(client);

        Ok(Self {
            context,
            stream_prefix: stream_prefix.to_string(),
        })
    }

    /// Initialize streams for all corridors
    pub async fn init_streams(&self, corridors: Vec<&str>) -> Result<()> {
        info!("Initializing JetStream streams for {} corridors", corridors.len());

        for corridor in corridors {
            self.create_corridor_stream(corridor).await?;
        }

        // Create DLQ stream
        self.create_dlq_stream().await?;

        info!("JetStream streams initialized successfully");
        Ok(())
    }

    /// Create stream for a specific corridor
    async fn create_corridor_stream(&self, corridor: &str) -> Result<()> {
        let stream_name = format!("{}_corridor_{}", self.stream_prefix, corridor);
        let subjects = vec![
            format!("payments.{}.>", corridor),
            format!("settlements.{}.>", corridor),
            format!("netting.{}.>", corridor),
        ];

        info!("Creating stream: {} with subjects: {:?}", stream_name, subjects);

        let config = StreamConfig {
            name: stream_name.clone(),
            description: Some(format!("DelTran corridor: {}", corridor)),
            subjects: subjects.clone(),
            retention: RetentionPolicy::WorkQueue,
            max_messages: 10_000_000, // 10M messages
            max_bytes: 10_737_418_240, // 10GB
            max_age: Duration::from_secs(7 * 24 * 3600), // 7 days
            storage: StorageType::File,
            num_replicas: 3, // 3x replication for durability
            duplicate_window: Duration::from_secs(300), // 5 min deduplication
            ..Default::default()
        };

        match self.context.get_or_create_stream(config).await {
            Ok(_) => {
                info!("Stream {} ready", stream_name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to create stream {}: {}", stream_name, e);
                Err(Error::StreamCreation(e.to_string()))
            }
        }
    }

    /// Create Dead Letter Queue stream
    async fn create_dlq_stream(&self) -> Result<()> {
        let stream_name = format!("{}_dlq", self.stream_prefix);

        info!("Creating DLQ stream: {}", stream_name);

        let config = StreamConfig {
            name: stream_name.clone(),
            description: Some("Dead Letter Queue for failed messages".to_string()),
            subjects: vec!["dlq.>".to_string()],
            retention: RetentionPolicy::Limits,
            max_messages: 1_000_000,
            max_bytes: 1_073_741_824, // 1GB
            max_age: Duration::from_secs(30 * 24 * 3600), // 30 days
            storage: StorageType::File,
            num_replicas: 3,
            ..Default::default()
        };

        match self.context.get_or_create_stream(config).await {
            Ok(_) => {
                info!("DLQ stream {} ready", stream_name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to create DLQ stream: {}", e);
                Err(Error::StreamCreation(e.to_string()))
            }
        }
    }

    /// Create consumer for a corridor
    pub async fn create_consumer(
        &self,
        corridor: &str,
        consumer_name: &str,
        service: &str,
    ) -> Result<PullConsumer> {
        let stream_name = format!("{}_corridor_{}", self.stream_prefix, corridor);

        info!(
            "Creating consumer {} for stream {} (service: {})",
            consumer_name, stream_name, service
        );

        let consumer_config = async_nats::jetstream::consumer::pull::Config {
            durable_name: Some(consumer_name.to_string()),
            description: Some(format!("Consumer for {} service", service)),
            filter_subject: format!("*.{}.{}", corridor, service),
            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
            ack_wait: Duration::from_secs(30),
            max_deliver: 5, // 5 attempts before DLQ
            deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
            replay_policy: async_nats::jetstream::consumer::ReplayPolicy::Instant,
            ..Default::default()
        };

        let consumer = self
            .context
            .create_consumer_on_stream(consumer_config, &stream_name)
            .await
            .map_err(|e| Error::ConsumerCreation(e.to_string()))?;

        info!("Consumer {} created successfully", consumer_name);
        Ok(consumer)
    }

    /// Publish message to corridor stream
    pub async fn publish(
        &self,
        subject: &str,
        partition_key: &PartitionKey,
        payload: Vec<u8>,
        idempotency_key: &str,
    ) -> Result<()> {
        // Add message ID header for deduplication
        let message = async_nats::jetstream::message::Message {
            payload: payload.into(),
            headers: Some({
                let mut headers = async_nats::HeaderMap::new();
                headers.insert("Nats-Msg-Id", idempotency_key);
                headers.insert("Corridor-Id", &partition_key.corridor_id);
                headers.insert("Bank-Id", &partition_key.bank_id);
                headers
            }),
            ..Default::default()
        };

        self.context
            .publish_with_headers(subject, message.headers.unwrap(), message.payload)
            .await
            .map_err(|e| Error::Publish(e.to_string()))?
            .await
            .map_err(|e| Error::Publish(e.to_string()))?;

        Ok(())
    }

    /// Get stream info
    pub async fn get_stream_info(&self, corridor: &str) -> Result<async_nats::jetstream::stream::Info> {
        let stream_name = format!("{}_corridor_{}", self.stream_prefix, corridor);

        self.context
            .get_stream(&stream_name)
            .await
            .map_err(|e| Error::StreamNotFound(e.to_string()))?
            .info()
            .await
            .map_err(|e| Error::StreamInfo(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires NATS server
    async fn test_jetstream_initialization() {
        let manager = JetStreamManager::new("nats://localhost:4222", "test")
            .await
            .expect("Failed to connect");

        manager
            .init_streams(vec!["UAE_IN", "IL_UAE"])
            .await
            .expect("Failed to init streams");
    }
}
