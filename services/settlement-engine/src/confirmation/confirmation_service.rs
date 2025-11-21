// Confirmation Service - Main orchestrator for settlement confirmations

use crate::confirmation::{Camt054Handler, Camt054Notification, UetrMatcher};
use crate::error::Result;
use async_nats::jetstream;
use tokio_stream::StreamExt;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info};

pub struct ConfirmationService {
    camt054_handler: Arc<Camt054Handler>,
    nats_client: Option<async_nats::Client>,
}

impl ConfirmationService {
    pub fn new(pool: PgPool) -> Self {
        let uetr_matcher = Arc::new(UetrMatcher::new(pool.clone()));
        let camt054_handler = Arc::new(Camt054Handler::new(pool, uetr_matcher));

        Self {
            camt054_handler,
            nats_client: None,
        }
    }

    /// Connect to NATS for consuming confirmation events
    pub async fn connect_nats(&mut self, nats_url: &str) -> Result<()> {
        let client = async_nats::connect(nats_url).await?;
        self.nats_client = Some(client);
        info!("Connected to NATS at {} for confirmations", nats_url);
        Ok(())
    }

    /// Start consuming CAMT.054 notifications from NATS
    pub async fn start_consuming_confirmations(&self) -> Result<()> {
        let client = self.nats_client.as_ref()
            .ok_or_else(|| crate::error::SettlementError::Internal(
                "NATS client not connected".to_string()
            ))?;

        info!("Starting CAMT.054 confirmation consumer...");

        let jetstream = jetstream::new(client.clone());

        // Get or create stream
        let stream = match jetstream.get_stream("settlement-confirmations").await {
            Ok(stream) => stream,
            Err(_) => {
                info!("Creating settlement-confirmations stream");
                jetstream
                    .create_stream(jetstream::stream::Config {
                        name: "settlement-confirmations".to_string(),
                        subjects: vec![
                            "bank.confirmations.camt.054".to_string(),
                            "settlement.confirmations".to_string(),
                        ],
                        max_messages: 1_000_000,
                        max_bytes: 1_073_741_824, // 1GB
                        ..Default::default()
                    })
                    .await?
            }
        };

        // Get or create consumer
        let consumer = match stream.get_consumer("settlement-confirmation-processor").await {
            Ok(consumer) => consumer,
            Err(_) => {
                info!("Creating settlement-confirmation-processor consumer");
                stream
                    .create_consumer(jetstream::consumer::pull::Config {
                        durable_name: Some("settlement-confirmation-processor".to_string()),
                        ack_policy: jetstream::consumer::AckPolicy::Explicit,
                        max_deliver: 5,
                        ..Default::default()
                    })
                    .await?
            }
        };

        info!("CAMT.054 confirmation consumer ready");

        // Start consuming messages
        let mut messages = consumer.messages().await?;

        while let Some(msg) = messages.next().await {
            match msg {
                Ok(message) => {
                    self.process_confirmation_message(message).await;
                }
                Err(e) => {
                    error!("Error receiving confirmation message: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Process individual confirmation message
    async fn process_confirmation_message(&self, message: jetstream::Message) {
        let subject = message.subject.to_string();
        info!("Received confirmation on subject: {}", subject);

        match serde_json::from_slice::<Camt054Notification>(&message.payload) {
            Ok(notification) => {
                info!(
                    "Processing CAMT.054: message_id={}, bank_ref={}",
                    notification.message_id, notification.bank_reference
                );

                match self.camt054_handler.process_notification(notification).await {
                    Ok(result) => {
                        info!(
                            "Confirmation processed: processed={}, settlement_id={:?}, action={}",
                            result.processed, result.settlement_id, result.action_taken
                        );

                        // Acknowledge successful processing
                        if let Err(e) = message.ack().await {
                            error!("Failed to ack message: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to process confirmation: {}. Will retry", e);
                        // Don't ack - message will be redelivered
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse CAMT.054 notification: {}. Acking to avoid redelivery", e);
                // Ack bad messages to prevent infinite redelivery
                let _ = message.ack().await;
            }
        }
    }

    /// Run confirmation service forever
    pub async fn run_forever(self: Arc<Self>) {
        loop {
            info!("Starting confirmation consumption loop");

            if let Err(e) = self.start_consuming_confirmations().await {
                error!("Confirmation consumer error: {}. Restarting in 5 seconds...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}
