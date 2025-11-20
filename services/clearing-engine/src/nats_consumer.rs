// NATS Consumer for Clearing Engine
// Listens to deltran.clearing.submit and processes multilateral netting

use async_nats::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;
use futures_util::StreamExt;
use rust_decimal::Decimal;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CanonicalPayment {
    pub deltran_tx_id: Uuid,
    pub uetr: Option<Uuid>,
    pub end_to_end_id: String,
    pub instruction_id: String,
    pub instructed_amount: Decimal,
    pub settlement_amount: Decimal,
    pub currency: String,
    pub debtor: Party,
    pub creditor: Party,
    pub debtor_agent: FinancialInstitution,
    pub creditor_agent: FinancialInstitution,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Party {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FinancialInstitution {
    pub bic: String,
    pub name: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ObligationCreatedEvent {
    pub obligation_id: Uuid,
    pub deltran_tx_id: Uuid,
    pub uetr: Option<Uuid>,
    pub amount: Decimal,
    pub currency: String,
    pub debtor_country: String,
    pub creditor_country: String,
}

#[derive(Debug, Deserialize)]
pub struct ClearingSubmission {
    pub payment: CanonicalPayment,
    pub obligation: ObligationCreatedEvent,
}

#[derive(Debug, Serialize)]
pub struct ClearingAcceptedEvent {
    pub obligation_id: Uuid,
    pub window_id: i64,
    pub currency: String,
    pub amount: Decimal,
    pub debtor_country: String,
    pub creditor_country: String,
    pub accepted_at: String,
}

pub async fn start_clearing_consumer(nats_url: &str) -> anyhow::Result<()> {
    info!("🔄 Starting Clearing Engine NATS consumer...");

    // Connect to NATS
    let nats_client = async_nats::connect(nats_url).await?;
    info!("✅ Connected to NATS: {}", nats_url);

    // Subscribe to clearing submission topic
    let mut subscriber = nats_client.subscribe("deltran.clearing.submit").await?;
    info!("📡 Subscribed to: deltran.clearing.submit");

    // Clone for spawned task
    let nats_for_publish = nats_client.clone();

    // Spawn consumer task
    tokio::spawn(async move {
        info!("🔄 Clearing consumer task started");

        while let Some(msg) = subscriber.next().await {
            // Parse ClearingSubmission from message
            match serde_json::from_slice::<ClearingSubmission>(&msg.payload) {
                Ok(submission) => {
                    info!(
                        "🌐 Received clearing request for obligation: {} (Payment: {}, Currency: {}, Amount: {})",
                        submission.obligation.obligation_id,
                        submission.payment.deltran_tx_id,
                        submission.obligation.currency,
                        submission.obligation.amount
                    );

                    // Add to clearing window
                    match add_to_clearing_window(&submission).await {
                        Ok(window_id) => {
                            info!(
                                "✅ Added obligation {} to clearing window {} ({} → {})",
                                submission.obligation.obligation_id,
                                window_id,
                                submission.obligation.debtor_country,
                                submission.obligation.creditor_country
                            );

                            // Publish acceptance event
                            let accepted_event = ClearingAcceptedEvent {
                                obligation_id: submission.obligation.obligation_id,
                                window_id,
                                currency: submission.obligation.currency.clone(),
                                amount: submission.obligation.amount,
                                debtor_country: submission.obligation.debtor_country.clone(),
                                creditor_country: submission.obligation.creditor_country.clone(),
                                accepted_at: chrono::Utc::now().to_rfc3339(),
                            };

                            if let Err(e) = publish_clearing_accepted(&nats_for_publish, &accepted_event).await {
                                error!("Failed to publish clearing accepted event: {}", e);
                            }

                            // Check if window should be closed and cleared
                            if should_trigger_clearing(window_id).await {
                                info!("🔄 Triggering multilateral netting for window {}", window_id);
                                if let Err(e) = trigger_multilateral_netting(window_id, &nats_for_publish).await {
                                    error!("Failed to trigger multilateral netting: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "❌ Failed to add obligation {} to clearing window: {}",
                                submission.obligation.obligation_id, e
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse ClearingSubmission from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️ Clearing consumer task ended");
    });

    info!("✅ Clearing consumer started successfully");

    Ok(())
}

/// Add obligation to current clearing window
async fn add_to_clearing_window(submission: &ClearingSubmission) -> anyhow::Result<i64> {
    // TODO: Implement proper window management
    // For now, use a fixed window ID based on current time (6-hour windows)
    let window_id = (chrono::Utc::now().timestamp() / (6 * 3600)) as i64;

    // TODO: Store obligation in database
    // This would insert into the obligations table with:
    // - obligation_id from submission
    // - window_id
    // - payer_id (extracted from debtor_agent BIC)
    // - payee_id (extracted from creditor_agent BIC)
    // - amount
    // - currency
    // - status: PENDING

    info!(
        "📋 Added obligation {} to window {} ({} {})",
        submission.obligation.obligation_id,
        window_id,
        submission.obligation.amount,
        submission.obligation.currency
    );

    Ok(window_id)
}

/// Check if window should trigger clearing
async fn should_trigger_clearing(window_id: i64) -> bool {
    // TODO: Implement proper triggering logic
    // Options:
    // 1. Time-based: Every 6 hours
    // 2. Volume-based: When obligations reach threshold
    // 3. Manual: API trigger

    // For MVP: trigger on schedule (every 6 hours)
    let current_window = (chrono::Utc::now().timestamp() / (6 * 3600)) as i64;

    // Don't trigger for current window (it's still collecting)
    window_id < current_window
}

/// Trigger multilateral netting for a window
async fn trigger_multilateral_netting(window_id: i64, nats_client: &Client) -> anyhow::Result<()> {
    info!("🔄 Starting multilateral netting for window {}", window_id);

    // TODO: Call ClearingOrchestrator.execute_clearing(window_id)
    // This would:
    // 1. Collect all PENDING obligations for the window
    // 2. Build currency-specific graphs
    // 3. Detect and eliminate cycles (multilateral netting)
    // 4. Calculate bilateral net positions
    // 5. Generate settlement instructions
    // 6. Save to database
    // 7. Publish to Liquidity Router

    // For now, publish a placeholder event
    let netting_event = serde_json::json!({
        "event_type": "clearing.netting.completed",
        "window_id": window_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "status": "SUCCESS",
    });

    nats_client
        .publish(
            "deltran.clearing.completed",
            serde_json::to_vec(&netting_event)?.into(),
        )
        .await?;

    info!("✅ Multilateral netting completed for window {}", window_id);

    Ok(())
}

/// Publish clearing accepted event
async fn publish_clearing_accepted(
    nats_client: &Client,
    event: &ClearingAcceptedEvent,
) -> anyhow::Result<()> {
    let subject = "deltran.events.clearing.accepted";
    let payload = serde_json::to_vec(event)?;

    nats_client.publish(subject, payload.into()).await?;

    info!(
        "📤 Published clearing accepted event: obligation {}",
        event.obligation_id
    );

    Ok(())
}

/// Publish net positions to Liquidity Router
pub async fn publish_to_liquidity_router(
    nats_client: &Client,
    window_id: i64,
    net_positions: &[NetPosition],
) -> anyhow::Result<()> {
    for position in net_positions {
        // Skip balanced positions
        if position.net_direction == "BALANCED" || position.net_amount == rust_decimal::Decimal::ZERO {
            continue;
        }

        let liquidity_request = serde_json::json!({
            "window_id": window_id,
            "net_position_id": position.id,
            "payer_id": position.net_payer_id,
            "payee_id": position.net_receiver_id,
            "amount": position.net_amount,
            "currency": position.currency,
            "bank_pair_hash": position.bank_pair_hash,
        });

        nats_client
            .publish(
                "deltran.liquidity.select",
                serde_json::to_vec(&liquidity_request)?.into(),
            )
            .await?;

        info!(
            "📤 Routed net position {} to Liquidity Router ({} {})",
            position.id, position.net_amount, position.currency
        );
    }

    Ok(())
}

// Re-export NetPosition from models
use crate::models::NetPosition;
