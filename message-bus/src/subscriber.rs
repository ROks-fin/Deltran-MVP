//! Message subscriber with consumer groups

use crate::{
    client::NatsClient,
    message::Message,
    metrics::{MESSAGE_PROCESS_DURATION, MESSAGE_RECEIVE_TOTAL},
    types::MessageType,
    Error, Result,
};
use async_nats::jetstream::{self, consumer};
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Message handler trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle incoming message
    async fn handle(&self, message: Message) -> Result<()>;
}

/// Subscriber configuration
#[derive(Debug, Clone)]
pub struct SubscriberConfig {
    /// Consumer group name (for load balancing)
    pub consumer_group: String,

    /// Durable consumer name
    pub durable_name: String,

    /// Max concurrent messages
    pub max_concurrent: usize,

    /// Acknowledgment wait time
    pub ack_wait: Duration,

    /// Max delivery attempts
    pub max_deliver: i64,

    /// Use JetStream (vs core NATS)
    pub use_jetstream: bool,
}

impl Default for SubscriberConfig {
    fn default() -> Self {
        Self {
            consumer_group: "deltran-workers".to_string(),
            durable_name: "deltran-consumer".to_string(),
            max_concurrent: 10,
            ack_wait: Duration::from_secs(30),
            max_deliver: 3,
            use_jetstream: true,
        }
    }
}

/// Message subscriber
pub struct Subscriber {
    client: Arc<NatsClient>,
    config: SubscriberConfig,
    message_type: MessageType,
}

impl Subscriber {
    /// Create new subscriber
    pub fn new(
        client: Arc<NatsClient>,
        config: SubscriberConfig,
        message_type: MessageType,
    ) -> Self {
        Self {
            client,
            config,
            message_type,
        }
    }

    /// Subscribe and process messages
    pub async fn subscribe<H>(&self, handler: Arc<H>) -> Result<()>
    where
        H: MessageHandler + 'static,
    {
        if self.config.use_jetstream {
            self.subscribe_jetstream(handler).await
        } else {
            self.subscribe_core(handler).await
        }
    }

    /// Subscribe using JetStream (durable, acknowledged)
    async fn subscribe_jetstream<H>(&self, handler: Arc<H>) -> Result<()>
    where
        H: MessageHandler + 'static,
    {
        let js = self.client.jetstream().await?;
        let stream_name = self.message_type.stream_name();

        info!(
            "Subscribing to JetStream stream: {} (consumer: {})",
            stream_name, self.config.consumer_group
        );

        // Ensure stream exists
        let subject_filter = format!("{}.*", self.message_type.subject_prefix());
        self.client
            .get_or_create_stream(stream_name, vec![subject_filter.clone()])
            .await?;

        // Create or get consumer
        let consumer_config = consumer::pull::Config {
            durable_name: Some(self.config.durable_name.clone()),
            filter_subject: subject_filter,
            ack_policy: consumer::AckPolicy::Explicit,
            ack_wait: self.config.ack_wait,
            max_deliver: self.config.max_deliver,
            deliver_policy: consumer::DeliverPolicy::All,
            ..Default::default()
        };

        let consumer = js
            .get_stream(stream_name)
            .await
            .map_err(|e| Error::JetStream(e.to_string()))?
            .create_consumer(consumer_config)
            .await
            .map_err(|e| Error::ConsumerGroup(e.to_string()))?;

        info!("✅ JetStream consumer created");

        // Process messages
        let mut messages = consumer
            .messages()
            .await
            .map_err(|e| Error::NatsSubscribe(e.to_string()))?;

        while let Some(msg) = messages.next().await {
            let msg = msg.map_err(|e| Error::NatsSubscribe(e.to_string()))?;

            // Parse message
            match Message::from_bytes(&msg.payload) {
                Ok(message) => {
                    let start = Instant::now();

                    // Record receive
                    MESSAGE_RECEIVE_TOTAL
                        .with_label_values(&[self.message_type.subject_prefix(), "success"])
                        .inc();

                    // Handle message
                    match handler.handle(message.clone()).await {
                        Ok(_) => {
                            // Acknowledge
                            if let Err(e) = msg.ack().await {
                                error!("Failed to ack message {}: {}", message.id, e);
                            }

                            // Record processing time
                            let duration = start.elapsed().as_secs_f64();
                            MESSAGE_PROCESS_DURATION
                                .with_label_values(&[self.message_type.subject_prefix()])
                                .observe(duration);
                        }
                        Err(e) => {
                            error!("Error handling message {}: {}", message.id, e);

                            // Negative acknowledgment (will be redelivered)
                            if let Err(nak_err) = msg.ack_with(jetstream::AckKind::Nak(None)).await
                            {
                                error!("Failed to nak message {}: {}", message.id, nak_err);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse message: {}", e);

                    MESSAGE_RECEIVE_TOTAL
                        .with_label_values(&[self.message_type.subject_prefix(), "parse_error"])
                        .inc();

                    // Terminate bad message (won't be redelivered)
                    if let Err(term_err) = msg.ack_with(jetstream::AckKind::Term).await {
                        error!("Failed to terminate bad message: {}", term_err);
                    }
                }
            }
        }

        Ok(())
    }

    /// Subscribe using core NATS (no persistence)
    async fn subscribe_core<H>(&self, handler: Arc<H>) -> Result<()>
    where
        H: MessageHandler + 'static,
    {
        let client = self.client.client().await?;
        let subject = format!("{}.*", self.message_type.subject_prefix());

        info!("Subscribing to core NATS subject: {}", subject);

        let mut subscriber = client
            .subscribe(subject.clone())
            .await
            .map_err(|e| Error::NatsSubscribe(e.to_string()))?;

        info!("✅ Subscribed to {}", subject);

        while let Some(msg) = subscriber.next().await {
            match Message::from_bytes(&msg.payload) {
                Ok(message) => {
                    let start = Instant::now();

                    MESSAGE_RECEIVE_TOTAL
                        .with_label_values(&[self.message_type.subject_prefix(), "success"])
                        .inc();

                    if let Err(e) = handler.handle(message).await {
                        error!("Error handling message: {}", e);
                    }

                    let duration = start.elapsed().as_secs_f64();
                    MESSAGE_PROCESS_DURATION
                        .with_label_values(&[self.message_type.subject_prefix()])
                        .observe(duration);
                }
                Err(e) => {
                    error!("Failed to parse message: {}", e);
                    MESSAGE_RECEIVE_TOTAL
                        .with_label_values(&[self.message_type.subject_prefix(), "parse_error"])
                        .inc();
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::NatsConfig;

    #[tokio::test]
    async fn test_subscriber_config_default() {
        let config = SubscriberConfig::default();
        assert_eq!(config.consumer_group, "deltran-workers");
        assert!(config.use_jetstream);
        assert_eq!(config.max_deliver, 3);
    }

    #[tokio::test]
    async fn test_subscriber_creation() {
        let client_config = NatsConfig::default();
        let client = Arc::new(NatsClient::new(client_config));
        let config = SubscriberConfig::default();

        let subscriber = Subscriber::new(client, config, MessageType::PaymentInstruction);
        assert_eq!(subscriber.message_type, MessageType::PaymentInstruction);
    }
}
