// NATS Consumer for Liquidity Router
// Listens to deltran.liquidity.select (international) and deltran.liquidity.select.local (local)

use async_nats::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;
use futures_util::StreamExt;
use rust_decimal::Decimal;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetPosition {
    pub id: Uuid,
    pub window_id: i64,
    pub bank_pair_hash: String,
    pub bank_a_id: Uuid,
    pub bank_b_id: Uuid,
    pub currency: String,
    pub gross_debit_a_to_b: Decimal,
    pub gross_credit_b_to_a: Decimal,
    pub net_amount: Decimal,
    pub net_direction: String,
    pub net_payer_id: Option<Uuid>,
    pub net_receiver_id: Option<Uuid>,
    pub obligations_netted: i32,
    pub netting_ratio: Decimal,
    pub amount_saved: Decimal,
}

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

#[derive(Debug, Deserialize)]
pub struct LocalLiquidityRequest {
    pub payment: CanonicalPayment,
    pub obligation: ObligationInfo,
    pub payment_type: String,
    pub jurisdiction: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ObligationInfo {
    pub obligation_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub debtor_country: String,
    pub creditor_country: String,
}

#[derive(Debug, Serialize)]
pub struct SettlementInstruction {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub net_position_id: Option<Uuid>,
    pub payer_bank_id: Uuid,
    pub payee_bank_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub instruction_type: String,
    pub selected_bank: BankInfo,
    pub selected_corridor: CorridorInfo,
    pub fx_rate: Option<Decimal>,
    pub estimated_time: String,
    pub priority: i32,
}

#[derive(Debug, Serialize, Clone)]
pub struct BankInfo {
    pub bank_id: Uuid,
    pub bank_name: String,
    pub bic: String,
    pub country: String,
    pub liquidity_score: f64,
    pub sla_score: f64,
    pub cost_score: f64,
    pub total_score: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct CorridorInfo {
    pub corridor_id: String,
    pub from_currency: String,
    pub to_currency: String,
    pub fx_rate: Decimal,
    pub cost_bps: i32,
    pub estimated_time_hours: i32,
}

pub async fn start_liquidity_consumer(nats_url: &str) -> anyhow::Result<()> {
    info!("💰 Starting Liquidity Router NATS consumer...");

    // Connect to NATS
    let nats_client = async_nats::connect(nats_url).await?;
    info!("✅ Connected to NATS: {}", nats_url);

    // Subscribe to international liquidity selection (from Clearing Engine)
    let mut international_sub = nats_client.subscribe("deltran.liquidity.select").await?;
    info!("📡 Subscribed to: deltran.liquidity.select (international)");

    // Subscribe to local liquidity selection (from Obligation Engine)
    let mut local_sub = nats_client.subscribe("deltran.liquidity.select.local").await?;
    info!("📡 Subscribed to: deltran.liquidity.select.local (local)");

    // Clone for spawned tasks
    let nats_for_international = nats_client.clone();
    let nats_for_local = nats_client.clone();

    // Spawn international consumer task
    tokio::spawn(async move {
        info!("🔄 International liquidity consumer task started");

        while let Some(msg) = international_sub.next().await {
            match serde_json::from_slice::<NetPosition>(&msg.payload) {
                Ok(net_position) => {
                    info!(
                        "🌍 Received international net position: {} ({} {} from window {})",
                        net_position.id,
                        net_position.net_amount,
                        net_position.currency,
                        net_position.window_id
                    );

                    // Select optimal corridor and bank for international payment
                    match select_international_liquidity(&net_position).await {
                        Ok(instruction) => {
                            info!(
                                "✅ Selected corridor: {} via bank {} (score: {:.2})",
                                instruction.selected_corridor.corridor_id,
                                instruction.selected_bank.bank_name,
                                instruction.selected_bank.total_score
                            );

                            // Publish to Settlement Engine
                            if let Err(e) = publish_to_settlement(&nats_for_international, &instruction).await {
                                error!("Failed to publish to Settlement Engine: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to select international liquidity: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse NetPosition from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️ International liquidity consumer task ended");
    });

    // Spawn local consumer task
    tokio::spawn(async move {
        info!("🔄 Local liquidity consumer task started");

        while let Some(msg) = local_sub.next().await {
            match serde_json::from_slice::<LocalLiquidityRequest>(&msg.payload) {
                Ok(request) => {
                    info!(
                        "🏠 Received local payment request: {} in {} ({} {})",
                        request.payment.deltran_tx_id,
                        request.jurisdiction,
                        request.payment.settlement_amount,
                        request.payment.currency
                    );

                    // Select optimal local bank
                    match select_local_liquidity(&request).await {
                        Ok(instruction) => {
                            info!(
                                "✅ Selected local bank: {} in {} (score: {:.2})",
                                instruction.selected_bank.bank_name,
                                instruction.selected_bank.country,
                                instruction.selected_bank.total_score
                            );

                            // Publish to Settlement Engine
                            if let Err(e) = publish_to_settlement(&nats_for_local, &instruction).await {
                                error!("Failed to publish to Settlement Engine: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to select local liquidity: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse LocalLiquidityRequest from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️ Local liquidity consumer task ended");
    });

    info!("✅ Liquidity Router consumers started successfully");

    Ok(())
}

/// Select optimal corridor and bank for international payments (from Clearing Engine)
async fn select_international_liquidity(net_position: &NetPosition) -> anyhow::Result<SettlementInstruction> {
    // TODO: Implement real bank selection logic
    // Factors:
    // 1. Liquidity availability
    // 2. FX rate (from Risk Engine)
    // 3. SLA (speed, reliability)
    // 4. Cost (fees, commissions)

    // For now, create a mock selection
    let selected_bank = BankInfo {
        bank_id: Uuid::new_v4(),
        bank_name: "HSBC UAE".to_string(),
        bic: "HSBCAEAA".to_string(),
        country: "AE".to_string(),
        liquidity_score: 0.9,
        sla_score: 0.85,
        cost_score: 0.8,
        total_score: 0.85,
    };

    let selected_corridor = CorridorInfo {
        corridor_id: format!("{}-{}", net_position.currency, net_position.currency),
        from_currency: net_position.currency.clone(),
        to_currency: net_position.currency.clone(),
        fx_rate: Decimal::from(1), // Same currency
        cost_bps: 20, // 0.2%
        estimated_time_hours: 2,
    };

    let instruction = SettlementInstruction {
        id: Uuid::new_v4(),
        payment_id: Uuid::new_v4(), // Will be mapped from net_position
        net_position_id: Some(net_position.id),
        payer_bank_id: net_position.net_payer_id.unwrap_or(Uuid::new_v4()),
        payee_bank_id: net_position.net_receiver_id.unwrap_or(Uuid::new_v4()),
        amount: net_position.net_amount,
        currency: net_position.currency.clone(),
        instruction_type: "INTERNATIONAL_NET".to_string(),
        selected_bank,
        selected_corridor,
        fx_rate: Some(Decimal::from(1)),
        estimated_time: "2 hours".to_string(),
        priority: 1,
    };

    info!(
        "💰 Selected international liquidity: {} {} via {}",
        instruction.amount,
        instruction.currency,
        instruction.selected_bank.bank_name
    );

    Ok(instruction)
}

/// Select optimal local bank for local payments (from Obligation Engine)
async fn select_local_liquidity(request: &LocalLiquidityRequest) -> anyhow::Result<SettlementInstruction> {
    // TODO: Implement real local bank selection logic
    // Factors:
    // 1. Local liquidity availability
    // 2. SLA (fastest local settlement)
    // 3. Cost (local fees)
    // 4. Integration (ISO 20022 vs API)

    let jurisdiction = &request.jurisdiction;

    // Mock selection based on jurisdiction
    let selected_bank = match jurisdiction.as_str() {
        "AE" => BankInfo {
            bank_id: Uuid::new_v4(),
            bank_name: "Emirates NBD".to_string(),
            bic: "EBILAEAD".to_string(),
            country: "AE".to_string(),
            liquidity_score: 0.95,
            sla_score: 0.9,
            cost_score: 0.85,
            total_score: 0.9,
        },
        "FR" => BankInfo {
            bank_id: Uuid::new_v4(),
            bank_name: "BNP Paribas".to_string(),
            bic: "BNPAFRPP".to_string(),
            country: "FR".to_string(),
            liquidity_score: 0.9,
            sla_score: 0.85,
            cost_score: 0.8,
            total_score: 0.85,
        },
        "IL" => BankInfo {
            bank_id: Uuid::new_v4(),
            bank_name: "Bank Hapoalim".to_string(),
            bic: "POALILIT".to_string(),
            country: "IL".to_string(),
            liquidity_score: 0.85,
            sla_score: 0.8,
            cost_score: 0.75,
            total_score: 0.8,
        },
        _ => BankInfo {
            bank_id: Uuid::new_v4(),
            bank_name: "Local Bank".to_string(),
            bic: format!("BANK{}XX", jurisdiction),
            country: jurisdiction.clone(),
            liquidity_score: 0.7,
            sla_score: 0.7,
            cost_score: 0.7,
            total_score: 0.7,
        },
    };

    let selected_corridor = CorridorInfo {
        corridor_id: format!("LOCAL-{}", jurisdiction),
        from_currency: request.payment.currency.clone(),
        to_currency: request.payment.currency.clone(),
        fx_rate: Decimal::from(1), // Same currency for local
        cost_bps: 10, // 0.1% for local
        estimated_time_hours: 1, // Faster for local
    };

    let instruction = SettlementInstruction {
        id: Uuid::new_v4(),
        payment_id: request.payment.deltran_tx_id,
        net_position_id: None, // No netting for local
        payer_bank_id: Uuid::new_v4(), // From payment.debtor_agent
        payee_bank_id: Uuid::new_v4(), // From payment.creditor_agent
        amount: request.payment.settlement_amount,
        currency: request.payment.currency.clone(),
        instruction_type: "LOCAL".to_string(),
        selected_bank,
        selected_corridor,
        fx_rate: None, // No FX for local
        estimated_time: "1 hour".to_string(),
        priority: 2, // Higher priority for local
    };

    info!(
        "🏠 Selected local liquidity: {} {} in {} via {}",
        instruction.amount,
        instruction.currency,
        jurisdiction,
        instruction.selected_bank.bank_name
    );

    Ok(instruction)
}

/// Publish settlement instruction to Settlement Engine
async fn publish_to_settlement(
    nats_client: &Client,
    instruction: &SettlementInstruction,
) -> anyhow::Result<()> {
    let subject = "deltran.settlement.execute";
    let payload = serde_json::to_vec(instruction)?;

    nats_client.publish(subject, payload.into()).await?;

    info!(
        "📤 Published to Settlement Engine: {} ({} {})",
        instruction.id,
        instruction.amount,
        instruction.currency
    );

    Ok(())
}
