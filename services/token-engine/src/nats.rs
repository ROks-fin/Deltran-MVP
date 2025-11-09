use crate::errors::{Result, TokenEngineError};
use crate::models::TokenEvent;
use async_nats::Client;
use serde_json;
use tracing::info;

pub struct NatsProducer {
    client: Client,
    topic_prefix: String,
}

impl NatsProducer {
    pub async fn new(url: &str, topic_prefix: &str) -> Result<Self> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| TokenEngineError::Nats(e.to_string()))?;

        info!("Connected to NATS at {}", url);

        Ok(NatsProducer {
            client,
            topic_prefix: topic_prefix.to_string(),
        })
    }

    pub async fn publish_token_event(&self, event: &TokenEvent) -> Result<()> {
        let subject = format!("{}.token.events", self.topic_prefix);
        let payload = serde_json::to_vec(event)
            .map_err(|e| TokenEngineError::Nats(format!("Serialization error: {}", e)))?;

        self.client
            .publish(subject.clone(), payload.into())
            .await
            .map_err(|e| TokenEngineError::Nats(format!("Failed to publish event: {}", e)))?;

        info!(
            "Published token event: {:?} for token {} to subject {}",
            event.event_type, event.token_id, subject
        );

        Ok(())
    }

    pub async fn publish_batch_events(&self, events: Vec<TokenEvent>) -> Result<()> {
        let subject = format!("{}.token.events", self.topic_prefix);

        for event in &events {
            let payload = serde_json::to_vec(event)
                .map_err(|e| TokenEngineError::Nats(format!("Serialization error: {}", e)))?;

            self.client
                .publish(subject.clone(), payload.into())
                .await
                .map_err(|e| TokenEngineError::Nats(format!("Failed to publish event: {}", e)))?;
        }

        info!("Published batch of {} token events", events.len());
        Ok(())
    }
}
