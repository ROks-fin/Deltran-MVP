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
                            // ============================================================
                            // INTERNATIONAL: Risk Engine → Liquidity Router → Clearing
                            //   - Risk Engine выбирает путь: instant buy, hedging, clearing
                            //   - Liquidity Router ищет лучшие FX курсы у партнёров
                            //   - Работает с несколькими валютами/юрисдикциями
                            //
                            // LOCAL: Obligation → Clearing (напрямую)
                            //   - Для оптимальной маршрутизации токенов/фиата между банками
                            //   - Без участия Risk/Liquidity (одна юрисдикция, одна валюта)
                            // ============================================================
                            if is_cross_border(&payment) {
                                info!("🌍 INTERNATIONAL payment - routing to Risk Engine for path selection");
                                info!("   Risk Engine → Liquidity Router → Clearing/Settlement");
                                if let Err(e) = publish_to_risk_engine(&nats_for_publish, &payment, &obligation).await {
                                    error!("Failed to route to Risk Engine: {}", e);
                                }
                            } else {
                                info!("🏠 LOCAL payment - routing DIRECTLY to Clearing Engine");
                                info!("   Direct token/fiat routing between banks (same jurisdiction)");
                                if let Err(e) = publish_to_clearing_local(&nats_for_publish, &payment, &obligation).await {
                                    error!("Failed to route to Clearing Engine: {}", e);
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

/// Route INTERNATIONAL payments to Risk Engine for path selection
/// Risk Engine decides: instant buy, hedging (full/partial), or clearing
async fn publish_to_risk_engine(nats_client: &Client, payment: &CanonicalPayment, obligation: &ObligationCreatedEvent) -> anyhow::Result<()> {
    let subject = "deltran.risk.check";

    let debtor_country = extract_country_from_bic(&payment.debtor_agent.bic);
    let creditor_country = extract_country_from_bic(&payment.creditor_agent.bic);

    // Create risk check request for international payment
    let risk_request = serde_json::json!({
        "request_id": Uuid::new_v4(),
        "payment_id": payment.deltran_tx_id,
        "obligation_id": obligation.obligation_id,
        "currency_pair": format!("{}/{}", payment.currency, payment.currency), // Will be updated with actual FX pair
        "amount": payment.settlement_amount,
        "from_currency": payment.currency,
        "to_currency": payment.currency, // Target currency for conversion
        "sender_country": debtor_country,
        "receiver_country": creditor_country,
        "payment_type": "INTERNATIONAL",
        "debtor_bic": payment.debtor_agent.bic.clone(),
        "creditor_bic": payment.creditor_agent.bic.clone(),
    });

    let payload = serde_json::to_vec(&risk_request)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Routed to Risk Engine (international): {} ({} → {})",
          payment.deltran_tx_id, debtor_country, creditor_country);

    Ok(())
}

/// Route LOCAL payments directly to Clearing Engine (bypassing Risk/Liquidity)
/// For optimal token/fiat routing between banks in same jurisdiction
async fn publish_to_clearing_local(nats_client: &Client, payment: &CanonicalPayment, obligation: &ObligationCreatedEvent) -> anyhow::Result<()> {
    let subject = "deltran.clearing.submit.local";

    let jurisdiction = extract_country_from_bic(&payment.creditor_agent.bic);

    // Create local clearing submission - direct token/fiat routing
    let clearing_data = serde_json::json!({
        "payment": payment,
        "obligation": obligation,
        "routing_type": "LOCAL_DIRECT",
        "jurisdiction": jurisdiction,
        "settlement_type": "INSTANT", // Local payments can settle instantly
        "skip_risk_check": true, // Same jurisdiction, no FX risk
        "skip_liquidity_router": true, // No cross-currency optimization needed
    });

    let payload = serde_json::to_vec(&clearing_data)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Routed DIRECTLY to Clearing (local): {} in {} (instant settlement)",
          payment.deltran_tx_id, jurisdiction);

    Ok(())
}

async fn publish_obligation_created(nats_client: &Client, obligation: &ObligationCreatedEvent) -> anyhow::Result<()> {
    let subject = "deltran.events.obligation.created";
    let payload = serde_json::to_vec(obligation)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Published obligation created event: {}", obligation.obligation_id);

    Ok(())
}
