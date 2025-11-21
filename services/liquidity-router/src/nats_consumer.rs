// NATS Consumer for Liquidity Router
// Listens to deltran.liquidity.select (international), deltran.liquidity.select.local (local)
// and deltran.settlement.path (instant buy requests from Risk Engine)

use async_nats::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use chrono::Utc;

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

// ======== INSTANT BUY SETTLEMENT PATH TYPES (from Risk Engine) ========

#[derive(Debug, Deserialize, Clone)]
pub struct SettlementPathDecision {
    pub path_type: SettlementPathType,
    pub confidence: f64,
    pub estimated_cost_bps: i32,
    pub estimated_time_ms: u64,
    pub reasoning: String,
    pub fx_provider: Option<String>,
    pub hedge_details: Option<HedgeDetails>,
    pub clearing_details: Option<ClearingDetails>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementPathType {
    InstantBuy,
    FullHedge,
    PartialHedge,
    Clearing,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HedgeDetails {
    pub hedge_ratio: f64,
    pub instrument: String,
    pub notional_amount: Decimal,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClearingDetails {
    pub window_id: Option<Uuid>,
    pub expected_netting_benefit_pct: f64,
    pub time_to_settlement_ms: u64,
}

// ======== FX PROVIDER SELECTION TYPES ========

#[derive(Debug, Serialize, Clone)]
pub struct FxProvider {
    pub provider_code: String,
    pub provider_name: String,
    pub bid_rate: Decimal,
    pub ask_rate: Decimal,
    pub spread_bps: i32,
    pub volume_discount_available: bool,
    pub execution_time_ms: u64,
    pub priority: i32,
}

#[derive(Debug, Serialize)]
pub struct InstantBuyExecution {
    pub execution_id: Uuid,
    pub transaction_id: Option<Uuid>,
    pub from_currency: String,
    pub to_currency: String,
    pub amount: Decimal,
    pub selected_provider: FxProvider,
    pub executed_rate: Decimal,
    pub total_cost_bps: i32,
    pub execution_time_ms: u64,
    pub status: String,
    pub executed_at: String,
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

    // Subscribe to settlement path decisions (from Risk Engine)
    let mut path_sub = nats_client.subscribe("deltran.settlement.path").await?;
    info!("📡 Subscribed to: deltran.settlement.path (instant buy/hedging from Risk Engine)");

    // Clone for spawned tasks
    let nats_for_international = nats_client.clone();
    let nats_for_local = nats_client.clone();
    let nats_for_path = nats_client.clone();

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

    // Spawn settlement path consumer task (instant buy / hedging from Risk Engine)
    tokio::spawn(async move {
        info!("🔄 Settlement path consumer task started (for instant buy)");

        while let Some(msg) = path_sub.next().await {
            match serde_json::from_slice::<SettlementPathDecision>(&msg.payload) {
                Ok(decision) => {
                    info!(
                        "🎯 Received settlement path decision: {:?} (confidence: {:.2})",
                        decision.path_type,
                        decision.confidence
                    );

                    // Handle based on path type
                    match decision.path_type {
                        SettlementPathType::InstantBuy => {
                            // Execute instant FX buy - select best provider
                            match execute_instant_buy(&decision).await {
                                Ok(execution) => {
                                    info!(
                                        "✅ Instant buy executed: {} at rate {} via {}",
                                        execution.execution_id,
                                        execution.executed_rate,
                                        execution.selected_provider.provider_code
                                    );

                                    // Publish execution result
                                    if let Err(e) = publish_instant_buy_result(&nats_for_path, &execution).await {
                                        error!("Failed to publish instant buy result: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to execute instant buy: {}", e);
                                }
                            }
                        }
                        SettlementPathType::FullHedge | SettlementPathType::PartialHedge => {
                            // Forward to hedging execution (mock for now)
                            info!(
                                "🛡️ Hedging request received: {:?} with ratio {:?}",
                                decision.path_type,
                                decision.hedge_details.as_ref().map(|h| h.hedge_ratio)
                            );
                            // In production, this would route to a hedging service
                        }
                        SettlementPathType::Clearing => {
                            // Clearing will be handled by Clearing Engine
                            info!("📊 Clearing path - will be handled by Clearing Engine");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse SettlementPathDecision from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️ Settlement path consumer task ended");
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

// ======== INSTANT BUY FX EXECUTION ========

/// Execute instant FX buy by selecting the best provider
async fn execute_instant_buy(decision: &SettlementPathDecision) -> anyhow::Result<InstantBuyExecution> {
    let start_time = std::time::Instant::now();

    // Get suggested provider from Risk Engine or select best one
    let provider_hint = decision.fx_provider.as_deref().unwrap_or("GLOBAL-FX-POOL");

    info!(
        "💱 Executing instant buy via provider hint: {}",
        provider_hint
    );

    // Get available providers for the corridor
    // In production, this would query the database and real-time rates
    let providers = get_fx_providers(provider_hint).await;

    // Select best provider based on spread and execution time
    let best_provider = providers
        .into_iter()
        .min_by_key(|p| p.spread_bps * 100 + p.execution_time_ms as i32)
        .ok_or_else(|| anyhow::anyhow!("No FX providers available"))?;

    info!(
        "✨ Selected best FX provider: {} (spread: {} bps, time: {} ms)",
        best_provider.provider_code,
        best_provider.spread_bps,
        best_provider.execution_time_ms
    );

    // Execute the trade (mock execution)
    let executed_rate = best_provider.ask_rate; // Buy at ask rate
    let execution_time = start_time.elapsed().as_millis() as u64;

    Ok(InstantBuyExecution {
        execution_id: Uuid::new_v4(),
        transaction_id: None, // Will be linked by caller
        from_currency: "USD".to_string(), // Would come from request
        to_currency: "AED".to_string(),   // Would come from request
        amount: Decimal::from(10000),     // Would come from request
        selected_provider: best_provider,
        executed_rate,
        total_cost_bps: decision.estimated_cost_bps,
        execution_time_ms: execution_time,
        status: "EXECUTED".to_string(),
        executed_at: Utc::now().to_rfc3339(),
    })
}

/// Get available FX providers for a corridor
async fn get_fx_providers(provider_hint: &str) -> Vec<FxProvider> {
    // Mock FX providers - in production, query database and real-time feeds
    let providers = vec![
        FxProvider {
            provider_code: "ENBD-FX".to_string(),
            provider_name: "Emirates NBD FX".to_string(),
            bid_rate: Decimal::from_str_exact("3.6720").unwrap_or(Decimal::from(3)),
            ask_rate: Decimal::from_str_exact("3.6730").unwrap_or(Decimal::from(3)),
            spread_bps: 3,
            volume_discount_available: true,
            execution_time_ms: 300,
            priority: 1,
        },
        FxProvider {
            provider_code: "UAE-EXCHANGE".to_string(),
            provider_name: "UAE Exchange".to_string(),
            bid_rate: Decimal::from_str_exact("3.6710").unwrap_or(Decimal::from(3)),
            ask_rate: Decimal::from_str_exact("3.6740").unwrap_or(Decimal::from(3)),
            spread_bps: 8,
            volume_discount_available: true,
            execution_time_ms: 500,
            priority: 2,
        },
        FxProvider {
            provider_code: "CLS-SETTLEMENT".to_string(),
            provider_name: "CLS Bank Settlement".to_string(),
            bid_rate: Decimal::from_str_exact("3.6722").unwrap_or(Decimal::from(3)),
            ask_rate: Decimal::from_str_exact("3.6728").unwrap_or(Decimal::from(3)),
            spread_bps: 2,
            volume_discount_available: false,
            execution_time_ms: 200,
            priority: 1,
        },
        FxProvider {
            provider_code: "GLOBAL-FX-POOL".to_string(),
            provider_name: "Global FX Pool".to_string(),
            bid_rate: Decimal::from_str_exact("3.6700").unwrap_or(Decimal::from(3)),
            ask_rate: Decimal::from_str_exact("3.6750").unwrap_or(Decimal::from(3)),
            spread_bps: 15,
            volume_discount_available: true,
            execution_time_ms: 800,
            priority: 5,
        },
    ];

    // If provider hint matches, prioritize that provider
    let mut sorted = providers;
    if let Some(pos) = sorted.iter().position(|p| p.provider_code == provider_hint) {
        let preferred = sorted.remove(pos);
        sorted.insert(0, preferred);
    }

    sorted
}

/// Publish instant buy execution result
async fn publish_instant_buy_result(
    nats_client: &Client,
    execution: &InstantBuyExecution,
) -> anyhow::Result<()> {
    let subject = "deltran.liquidity.instant_buy.result";
    let payload = serde_json::to_vec(execution)?;

    nats_client.publish(subject, payload.into()).await?;

    info!(
        "📤 Published instant buy result: {} (rate: {}, provider: {})",
        execution.execution_id,
        execution.executed_rate,
        execution.selected_provider.provider_code
    );

    Ok(())
}

// Helper for Decimal parsing
trait DecimalExt {
    fn from_str_exact(s: &str) -> Option<Decimal>;
}

impl DecimalExt for Decimal {
    fn from_str_exact(s: &str) -> Option<Decimal> {
        s.parse().ok()
    }
}
