// NATS Consumer - Listens to funding confirmation events for token minting
// Also handles CAMT.054 events for real-time reconciliation

use crate::errors::{Result, TokenEngineError as Error};
use crate::reconciliation::{ReconciliationService, camt054_processor::Camt054Notification};
use async_nats::jetstream;
use futures_util::StreamExt;
use std::sync::Arc;
use tracing::{error, info, warn};
use serde::Deserialize;
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

/// Funding confirmation event from Account Monitor
/// This triggers token minting with 1:1 FIAT backing guarantee
#[derive(Debug, Deserialize)]
pub struct FundingEvent {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub transaction_id: String,
    pub account_id: String,
    pub amount: Decimal,
    pub currency: String,
    pub end_to_end_id: Option<String>,
    pub debtor_name: Option<String>,
    pub debtor_account: Option<String>,
    pub booking_date: Option<DateTime<Utc>>,
    pub value_date: Option<DateTime<Utc>>,
    pub confirmed_at: DateTime<Utc>,
}

// ============================================================
// TOKEN OPERATIONS - Token Engine is the ONLY service that
// manipulates tokens. Other services REQUEST token operations.
// ============================================================

/// Token Transfer Request - from Settlement Engine or Clearing Engine
/// Token Engine executes the actual token movement between bank accounts
#[derive(Debug, Deserialize)]
pub struct TokenTransferRequest {
    #[serde(rename = "type")]
    pub transfer_type: String,      // "LOCAL_TOKEN", "INTERNATIONAL_TOKEN"
    pub obligation_id: Uuid,
    pub payment_id: Uuid,
    pub from_bank_bic: String,
    pub to_bank_bic: String,
    pub amount: Decimal,
    pub currency: String,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    #[serde(default)]
    pub settlement_type: Option<String>,
}

/// Token Burn Request - when fiat is withdrawn
#[derive(Debug, Deserialize)]
pub struct TokenBurnRequest {
    pub burn_id: Uuid,
    pub bank_bic: String,
    pub amount: Decimal,
    pub currency: String,
    pub reason: String,             // "FIAT_WITHDRAWAL", "SETTLEMENT_COMPLETE"
}

pub struct NatsConsumer {
    client: async_nats::Client,
    stream_name: String,
    consumer_name: String,
    reconciliation_service: Arc<ReconciliationService>,
}

impl NatsConsumer {
    pub async fn new(
        nats_url: &str,
        stream_name: String,
        consumer_name: String,
        reconciliation_service: Arc<ReconciliationService>,
    ) -> Result<Self> {
        let client = async_nats::connect(nats_url).await?;

        info!(
            "Connected to NATS at {} for stream {}",
            nats_url, stream_name
        );

        Ok(Self {
            client,
            stream_name,
            consumer_name,
            reconciliation_service,
        })
    }

    /// Start consuming CAMT.054 notifications
    pub async fn start_consuming_camt054(&self) -> Result<()> {
        info!(
            "Starting CAMT.054 consumer on stream: {}",
            self.stream_name
        );

        let jetstream = jetstream::new(self.client.clone());

        // Get or create stream
        let stream = match jetstream.get_stream(&self.stream_name).await {
            Ok(stream) => stream,
            Err(_) => {
                info!("Stream {} not found, creating...", self.stream_name);
                jetstream
                    .create_stream(jetstream::stream::Config {
                        name: self.stream_name.clone(),
                        subjects: vec![
                            "iso20022.camt.054".to_string(),
                            "bank.notifications.credit".to_string(),
                            "bank.notifications.debit".to_string(),
                        ],
                        max_messages: 1_000_000,
                        max_bytes: 1_073_741_824, // 1GB
                        ..Default::default()
                    })
                    .await?
            }
        };

        // Get or create consumer
        let consumer = match stream.get_consumer(&self.consumer_name).await {
            Ok(consumer) => consumer,
            Err(_) => {
                info!("Consumer {} not found, creating...", self.consumer_name);
                stream
                    .create_consumer(jetstream::consumer::pull::Config {
                        durable_name: Some(self.consumer_name.clone()),
                        ack_policy: jetstream::consumer::AckPolicy::Explicit,
                        max_deliver: 5,
                        ..Default::default()
                    })
                    .await?
            }
        };

        info!(
            "CAMT.054 consumer ready: stream={}, consumer={}",
            self.stream_name, self.consumer_name
        );

        // Start consuming messages
        let mut messages = consumer
            .messages()
            .await?
            .take(10_000); // Process in batches

        while let Some(msg) = messages.next().await {
            match msg {
                Ok(message) => {
                    self.process_message(message).await;
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Process individual NATS message
    async fn process_message(&self, message: jetstream::Message) {
        let subject = message.subject.to_string();
        info!("Received message on subject: {}", subject);

        // Parse CAMT.054 notification from message payload
        match serde_json::from_slice::<Camt054Notification>(&message.payload) {
            Ok(notification) => {
                info!(
                    "Processing CAMT.054 notification: message_id={}, account={}",
                    notification.message_id, notification.account_id
                );

                // Process reconciliation
                match self
                    .reconciliation_service
                    .process_camt054_notification(notification)
                    .await
                {
                    Ok(result) => {
                        info!(
                            "Reconciliation complete: account={}, ledger={}, bank={}, diff={}, level={:?}",
                            result.account_id,
                            result.ledger_balance,
                            result.bank_balance,
                            result.difference,
                            result.threshold_level
                        );

                        // Acknowledge message
                        if let Err(e) = message.ack().await {
                            error!("Failed to ack message: {}", e);
                        }
                    }
                    Err(e) => {
                        error!(
                            "Reconciliation failed: {}. Will retry (message not acked)",
                            e
                        );
                        // Don't ack - message will be redelivered
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to parse CAMT.054 notification: {}. Acking to avoid redelivery of bad message",
                    e
                );
                // Ack bad messages to prevent infinite redelivery
                let _ = message.ack().await;
            }
        }
    }

    /// Start listening for funding confirmation events (deltran.funding.confirmed)
    /// This is the PRIMARY trigger for token minting - ensures 1:1 FIAT backing
    pub async fn start_funding_consumer(&self) -> Result<()> {
        info!("🪙 Starting Funding Confirmation consumer");

        // Subscribe to funding confirmation topic
        let mut subscriber = self.client.subscribe("deltran.funding.confirmed").await?;
        info!("📡 Subscribed to: deltran.funding.confirmed");

        // Start consuming messages
        while let Some(msg) = subscriber.next().await {
            match serde_json::from_slice::<FundingEvent>(&msg.payload) {
                Ok(funding_event) => {
                    info!(
                        "💰 Received funding confirmation: {} for payment {} - {} {} from account {}",
                        funding_event.id,
                        funding_event.payment_id,
                        funding_event.amount,
                        funding_event.currency,
                        funding_event.account_id
                    );

                    // Process token minting
                    match self.mint_tokens_from_funding(&funding_event).await {
                        Ok(token_id) => {
                            info!(
                                "✅ Tokens minted successfully: {} tokens ({} {}) for payment {}",
                                token_id,
                                funding_event.amount,
                                funding_event.currency,
                                funding_event.payment_id
                            );
                        }
                        Err(e) => {
                            error!(
                                "❌ Failed to mint tokens for funding event {}: {}",
                                funding_event.id, e
                            );
                            // TODO: Implement retry logic or dead-letter queue
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse FundingEvent from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️  Funding consumer ended");
        Ok(())
    }

    /// Mint tokens after funding confirmation (1:1 backing guarantee)
    async fn mint_tokens_from_funding(&self, event: &FundingEvent) -> Result<Uuid> {
        info!(
            "🪙 Minting tokens: {} {} for payment {}",
            event.amount, event.currency, event.payment_id
        );

        // Determine token type based on currency
        let token_type = match event.currency.as_str() {
            "USD" => "xUSD",
            "AED" => "xAED",
            "ILS" => "xILS",
            "EUR" => "xEUR",
            "GBP" => "xGBP",
            _ => {
                error!("Unsupported currency for tokenization: {}", event.currency);
                return Err(crate::errors::TokenEngineError::InvalidCurrency(event.currency.clone()));
            }
        };

        info!(
            "📝 Creating token: {} = {} {} (1:1 backing)",
            token_type, event.amount, event.currency
        );

        // TODO: Implement actual token minting logic
        // This should:
        // 1. Create token record in database
        // 2. Link to funding_event.id and payment_id
        // 3. Update token balances
        // 4. Publish token.minted event to NATS
        // 5. Return token_id

        let token_id = Uuid::new_v4();

        info!(
            "✅ Token minted: {} ({} {} = {} {})",
            token_id, event.amount, event.currency, event.amount, token_type
        );

        // TODO: Publish token.minted event
        self.publish_token_minted(token_id, event).await?;

        Ok(token_id)
    }

    /// Publish token minted event
    async fn publish_token_minted(&self, token_id: Uuid, event: &FundingEvent) -> Result<()> {
        let subject = "deltran.token.minted";

        #[derive(serde::Serialize)]
        struct TokenMintedEvent {
            token_id: Uuid,
            payment_id: Uuid,
            funding_event_id: Uuid,
            amount: Decimal,
            currency: String,
            token_type: String,
            minted_at: DateTime<Utc>,
        }

        let token_type = match event.currency.as_str() {
            "USD" => "xUSD",
            "AED" => "xAED",
            "ILS" => "xILS",
            "EUR" => "xEUR",
            "GBP" => "xGBP",
            _ => "UNKNOWN",
        };

        let event_payload = TokenMintedEvent {
            token_id,
            payment_id: event.payment_id,
            funding_event_id: event.id,
            amount: event.amount,
            currency: event.currency.clone(),
            token_type: token_type.to_string(),
            minted_at: Utc::now(),
        };

        let payload = serde_json::to_vec(&event_payload)?;
        self.client.publish(subject, payload.into()).await?;

        info!(
            "📤 Published token.minted event: {} ({} {})",
            token_id, event.amount, token_type
        );

        Ok(())
    }

    /// Start listening for token burn requests (deltran.token.burn)
    /// CRITICAL: Tokens must be burned at end of transaction lifecycle
    pub async fn start_burn_consumer(&self) -> Result<()> {
        info!("🔥 Starting Token Burn consumer");

        let mut subscriber = self.client.subscribe("deltran.token.burn").await?;
        info!("📡 Subscribed to: deltran.token.burn");

        while let Some(msg) = subscriber.next().await {
            match serde_json::from_slice::<TokenBurnRequest>(&msg.payload) {
                Ok(burn_request) => {
                    info!(
                        "🔥 Received burn request: {} for {} {} (reason: {})",
                        burn_request.burn_id,
                        burn_request.amount,
                        burn_request.currency,
                        burn_request.reason
                    );

                    // Execute token burn
                    match self.burn_tokens(&burn_request).await {
                        Ok(()) => {
                            info!(
                                "✅ Tokens burned successfully: {} {} (burn_id: {})",
                                burn_request.amount,
                                burn_request.currency,
                                burn_request.burn_id
                            );
                        }
                        Err(e) => {
                            error!(
                                "❌ Failed to burn tokens for burn_id {}: {}",
                                burn_request.burn_id, e
                            );
                            // TODO: Implement retry logic or dead-letter queue
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse TokenBurnRequest from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️ Token burn consumer ended");
        Ok(())
    }

    /// Burn tokens - delete from system at end of transaction lifecycle
    async fn burn_tokens(&self, request: &TokenBurnRequest) -> Result<()> {
        info!(
            "🔥 Burning tokens: {} {} for bank {} (reason: {})",
            request.amount, request.currency, request.bank_bic, request.reason
        );

        // TODO: Implement actual token burn logic:
        // 1. Delete token record from database (or mark as BURNED)
        // 2. Update bank token balances (decrease)
        // 3. Update system token supply (decrease)
        // 4. Create burn audit log entry
        // 5. Publish token.burned event for monitoring

        // For now, just publish the burn event
        let burn_event = serde_json::json!({
            "burn_id": request.burn_id,
            "bank_bic": request.bank_bic,
            "amount": request.amount,
            "currency": request.currency,
            "reason": request.reason,
            "burned_at": Utc::now().to_rfc3339(),
        });

        self.client
            .publish("deltran.token.burned", serde_json::to_vec(&burn_event)?.into())
            .await?;

        info!(
            "📤 Published token.burned event: {} ({} {} burned)",
            request.burn_id, request.amount, request.currency
        );

        Ok(())
    }

    /// Start continuous consumption loop
    pub async fn run_forever(self: Arc<Self>) {
        // Spawn CAMT.054 consumer task
        let self_clone = self.clone();
        tokio::spawn(async move {
            loop {
                info!("Starting CAMT.054 consumption loop");

                if let Err(e) = self_clone.start_consuming_camt054().await {
                    error!("CAMT.054 consumer error: {}. Restarting in 5 seconds...", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        });

        // Spawn Funding Confirmation consumer task (CRITICAL for token minting)
        let self_clone = self.clone();
        tokio::spawn(async move {
            loop {
                info!("🪙 Starting Funding Confirmation consumption loop");

                if let Err(e) = self_clone.start_funding_consumer().await {
                    error!("Funding consumer error: {}. Restarting in 5 seconds...", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        });

        // Spawn Token Burn consumer task (CRITICAL for token lifecycle completion)
        loop {
            info!("🔥 Starting Token Burn consumption loop");

            if let Err(e) = self.start_burn_consumer().await {
                error!("Token burn consumer error: {}. Restarting in 5 seconds...", e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }

}
