// NATS Consumer for Obligation Engine
// Listens to deltran.obligation.create and creates payment obligations

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

#[derive(Debug, Serialize)]
pub struct ObligationCreatedEvent {
    pub obligation_id: Uuid,
    pub deltran_tx_id: Uuid,
    pub uetr: Option<Uuid>,
    pub amount: Decimal,
    pub currency: String,
    pub debtor_country: String,
    pub creditor_country: String,
}

pub async fn start_obligation_consumer(nats_url: &str) -> anyhow::Result<()> {
    info!("📋 Starting Obligation Engine NATS consumer...");

    // Connect to NATS
    let nats_client = async_nats::connect(nats_url).await?;
    info!("✅ Connected to NATS: {}", nats_url);

    // Subscribe to obligation creation topic
    let mut subscriber = nats_client.subscribe("deltran.obligation.create").await?;
    info!("📡 Subscribed to: deltran.obligation.create");

    // Clone for spawned task
    let nats_for_publish = nats_client.clone();

    // Spawn consumer task
    tokio::spawn(async move {
        info!("🔄 Obligation consumer task started");

        while let Some(msg) = subscriber.next().await {
            // Parse CanonicalPayment from message
            match serde_json::from_slice::<CanonicalPayment>(&msg.payload) {
                Ok(payment) => {
                    info!("📋 Received obligation creation request for: {} (E2E: {}, UETR: {:?})",
                          payment.deltran_tx_id, payment.end_to_end_id, payment.uetr);

                    // Create obligation
                    match create_obligation(&payment).await {
                        Ok(obligation) => {
                            info!("✅ Obligation created: {} for payment {}",
                                  obligation.obligation_id, payment.deltran_tx_id);

                            // Route based on payment type:
                            // International → Clearing Engine (multilateral netting)
                            // Local → Liquidity Router (select local payout bank)
                            // NOTE: Token Engine будет вызван ПОСЛЕ settlement и camt.054 confirmation
                            if is_cross_border(&payment) {
                                info!("🌍 Cross-border payment - routing to Clearing Engine");
                                if let Err(e) = publish_to_clearing(&nats_for_publish, &payment, &obligation).await {
                                    error!("Failed to route to Clearing Engine: {}", e);
                                }
                            } else {
                                info!("🏠 Local payment - routing to Liquidity Router");
                                if let Err(e) = publish_to_liquidity_router(&nats_for_publish, &payment, &obligation).await {
                                    error!("Failed to route to Liquidity Router: {}", e);
                                }
                            }

                            // Publish obligation created event for analytics
                            if let Err(e) = publish_obligation_created(&nats_for_publish, &obligation).await {
                                error!("Failed to publish obligation created event: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("❌ Failed to create obligation for payment {}: {}",
                                   payment.deltran_tx_id, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse CanonicalPayment from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️ Obligation consumer task ended");
    });

    info!("✅ Obligation consumer started successfully");

    Ok(())
}

async fn create_obligation(payment: &CanonicalPayment) -> anyhow::Result<ObligationCreatedEvent> {
    // Generate obligation ID
    let obligation_id = Uuid::new_v4();

    // Determine countries from BIC codes or other data
    // For now, extract from BIC (first 2 chars after 4-char bank code)
    // Real implementation would use proper BIC lookup
    let debtor_country = extract_country_from_bic(&payment.debtor_agent.bic);
    let creditor_country = extract_country_from_bic(&payment.creditor_agent.bic);

    info!("Creating obligation: {} → {} ({} {})",
          debtor_country, creditor_country, payment.settlement_amount, payment.currency);

    // TODO: Store obligation in database
    // For now, just create the event

    Ok(ObligationCreatedEvent {
        obligation_id,
        deltran_tx_id: payment.deltran_tx_id,
        uetr: payment.uetr,
        amount: payment.settlement_amount,
        currency: payment.currency.clone(),
        debtor_country,
        creditor_country,
    })
}

fn is_cross_border(payment: &CanonicalPayment) -> bool {
    // Determine if payment is cross-border
    let debtor_country = extract_country_from_bic(&payment.debtor_agent.bic);
    let creditor_country = extract_country_from_bic(&payment.creditor_agent.bic);

    debtor_country != creditor_country
}

fn extract_country_from_bic(bic: &str) -> String {
    // BIC format: XXXXYYZZAAA
    // XXXX = bank code (4 chars)
    // YY = country code (2 chars)
    // ZZ = location code (2 chars)
    // AAA = branch code (3 chars, optional)

    if bic.len() >= 6 {
        bic[4..6].to_uppercase()
    } else {
        "XX".to_string() // Unknown
    }
}

async fn publish_to_clearing(nats_client: &Client, payment: &CanonicalPayment, obligation: &ObligationCreatedEvent) -> anyhow::Result<()> {
    let subject = "deltran.clearing.submit";

    // Create clearing submission with obligation info
    let clearing_data = serde_json::json!({
        "payment": payment,
        "obligation": obligation,
    });

    let payload = serde_json::to_vec(&clearing_data)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Routed to Clearing Engine: {} (obligation: {})",
          payment.deltran_tx_id, obligation.obligation_id);

    Ok(())
}

async fn publish_to_token_engine(nats_client: &Client, payment: &CanonicalPayment) -> anyhow::Result<()> {
    let subject = "deltran.token.mint";
    let payload = serde_json::to_vec(payment)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Routed to Token Engine: {}", payment.deltran_tx_id);

    Ok(())
}

async fn publish_to_liquidity_router(nats_client: &Client, payment: &CanonicalPayment, obligation: &ObligationCreatedEvent) -> anyhow::Result<()> {
    let subject = "deltran.liquidity.select.local";

    // For local payments, Liquidity Router selects optimal local payout bank
    let liquidity_request = serde_json::json!({
        "payment": payment,
        "obligation": obligation,
        "payment_type": "LOCAL",
        "jurisdiction": extract_country_from_bic(&payment.creditor_agent.bic),
    });

    let payload = serde_json::to_vec(&liquidity_request)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Routed to Liquidity Router (local): {} in {}",
          payment.deltran_tx_id,
          extract_country_from_bic(&payment.creditor_agent.bic));

    Ok(())
}

async fn publish_obligation_created(nats_client: &Client, obligation: &ObligationCreatedEvent) -> anyhow::Result<()> {
    let subject = "deltran.events.obligation.created";
    let payload = serde_json::to_vec(obligation)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Published obligation created event: {}", obligation.obligation_id);

    Ok(())
}
