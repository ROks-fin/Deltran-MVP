// NATS Consumer for Settlement Engine
// Listens to deltran.settlement.execute and executes payouts

use async_nats::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;
use rust_decimal::Decimal;
use chrono::Utc;
use tokio_stream::StreamExt;

#[derive(Debug, Deserialize)]
pub struct SettlementInstruction {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub net_position_id: Option<Uuid>,
    pub payer_bank_id: Uuid,
    pub payee_bank_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub instruction_type: String, // "INTERNATIONAL_NET", "LOCAL", etc.
    pub selected_bank: BankInfo,
    pub selected_corridor: CorridorInfo,
    pub fx_rate: Option<Decimal>,
    pub estimated_time: String,
    pub priority: i32,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
pub struct CorridorInfo {
    pub corridor_id: String,
    pub from_currency: String,
    pub to_currency: String,
    pub fx_rate: Decimal,
    pub cost_bps: i32,
    pub estimated_time_hours: i32,
}

#[derive(Debug, Serialize)]
pub struct SettlementResult {
    pub settlement_id: Uuid,
    pub instruction_id: Uuid,
    pub payment_id: Uuid,
    pub status: SettlementStatus,
    pub amount: Decimal,
    pub currency: String,
    pub execution_method: String, // "ISO20022", "API", "SWIFT"
    pub confirmation_reference: Option<String>,
    pub executed_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SettlementStatus {
    Pending,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize)]
pub struct Pacs008Message {
    pub message_id: String,
    pub creation_date_time: String,
    pub number_of_transactions: i32,
    pub control_sum: Decimal,
    pub debtor_bic: String,
    pub creditor_bic: String,
    pub amount: Decimal,
    pub currency: String,
    pub end_to_end_id: String,
    pub instruction_id: String,
}

pub async fn start_settlement_consumer(nats_url: &str) -> anyhow::Result<()> {
    info!("💸 Starting Settlement Engine NATS consumer...");

    // Connect to NATS
    let nats_client = async_nats::connect(nats_url).await?;
    info!("✅ Connected to NATS: {}", nats_url);

    // Subscribe to settlement execution topic
    let mut subscriber = nats_client.subscribe("deltran.settlement.execute").await?;
    info!("📡 Subscribed to: deltran.settlement.execute");

    // Clone for spawned task
    let nats_for_publish = nats_client.clone();

    // Spawn consumer task
    tokio::spawn(async move {
        info!("🔄 Settlement consumer task started");

        loop {
            match subscriber.next().await {
                Some(msg) => {
                    match serde_json::from_slice::<SettlementInstruction>(&msg.payload) {
                Ok(instruction) => {
                    info!(
                        "💸 Received settlement instruction: {} ({} {} via {} - type: {})",
                        instruction.id,
                        instruction.amount,
                        instruction.currency,
                        instruction.selected_bank.bank_name,
                        instruction.instruction_type
                    );

                    // Execute settlement
                    match execute_settlement(&instruction).await {
                        Ok(result) => {
                            match result.status {
                                SettlementStatus::Completed => {
                                    info!(
                                        "✅ Settlement completed: {} via {} (ref: {:?})",
                                        result.settlement_id,
                                        result.execution_method,
                                        result.confirmation_reference
                                    );
                                }
                                SettlementStatus::Failed => {
                                    error!(
                                        "❌ Settlement failed: {} - {}",
                                        result.settlement_id,
                                        result.error_message.clone().unwrap_or_default()
                                    );
                                }
                                _ => {
                                    info!("⏳ Settlement in progress: {}", result.settlement_id);
                                }
                            }

                            // Publish settlement result
                            if let Err(e) = publish_settlement_completed(&nats_for_publish, &result).await {
                                error!("Failed to publish settlement result: {}", e);
                            }
                        }
                        Err(e) => {
                            error!(
                                "❌ Failed to execute settlement {}: {}",
                                instruction.id, e
                            );

                            // Publish failure result
                            let failed_result = SettlementResult {
                                settlement_id: Uuid::new_v4(),
                                instruction_id: instruction.id,
                                payment_id: instruction.payment_id,
                                status: SettlementStatus::Failed,
                                amount: instruction.amount,
                                currency: instruction.currency.clone(),
                                execution_method: "UNKNOWN".to_string(),
                                confirmation_reference: None,
                                executed_at: Utc::now().to_rfc3339(),
                                completed_at: None,
                                error_message: Some(e.to_string()),
                            };

                            if let Err(e) = publish_settlement_completed(&nats_for_publish, &failed_result).await {
                                error!("Failed to publish failure result: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse SettlementInstruction from NATS message: {}", e);
                }
            }
                }
                None => {
                    warn!("⚠️ NATS subscription ended");
                    break;
                }
            }
        }

        warn!("⚠️ Settlement consumer task ended");
    });

    info!("✅ Settlement consumer started successfully");

    Ok(())
}

/// Execute settlement based on instruction type and bank capabilities
async fn execute_settlement(instruction: &SettlementInstruction) -> anyhow::Result<SettlementResult> {
    let settlement_id = Uuid::new_v4();

    info!(
        "🚀 Executing settlement {} for payment {} ({} {})",
        settlement_id,
        instruction.payment_id,
        instruction.amount,
        instruction.currency
    );

    // Determine execution method based on instruction type and bank
    let execution_method = determine_execution_method(instruction);

    let result = match execution_method.as_str() {
        "ISO20022" => execute_iso20022_settlement(instruction, settlement_id).await?,
        "SWIFT" => execute_swift_settlement(instruction, settlement_id).await?,
        "API" => execute_api_settlement(instruction, settlement_id).await?,
        _ => {
            return Err(anyhow::anyhow!("Unknown execution method: {}", execution_method));
        }
    };

    info!(
        "✅ Settlement {} executed via {} (status: {:?})",
        settlement_id,
        execution_method,
        result.status
    );

    Ok(result)
}

/// Determine optimal execution method
fn determine_execution_method(instruction: &SettlementInstruction) -> String {
    // Priority order:
    // 1. ISO 20022 (standard, preferred)
    // 2. SWIFT (for international)
    // 3. API (for local, fast)

    match instruction.instruction_type.as_str() {
        "LOCAL" => {
            // Local payments prefer API for speed
            if instruction.selected_bank.country == "AE" || instruction.selected_bank.country == "IL" {
                "API".to_string()
            } else {
                "ISO20022".to_string()
            }
        }
        "INTERNATIONAL_NET" | "CROSS_BORDER" => {
            // International prefers ISO 20022 or SWIFT
            "ISO20022".to_string()
        }
        _ => "ISO20022".to_string(),
    }
}

/// Execute settlement via ISO 20022 (pacs.008)
async fn execute_iso20022_settlement(
    instruction: &SettlementInstruction,
    settlement_id: Uuid,
) -> anyhow::Result<SettlementResult> {
    info!("📄 Generating ISO 20022 pacs.008 message");

    // Generate pacs.008 (FIToFICstmrCdtTrf)
    let pacs008 = Pacs008Message {
        message_id: format!("STLMT{}", settlement_id.to_string().replace("-", "")[..16].to_uppercase()),
        creation_date_time: Utc::now().to_rfc3339(),
        number_of_transactions: 1,
        control_sum: instruction.amount,
        debtor_bic: format!("BANK{}XX", instruction.payer_bank_id.to_string()[..6].to_uppercase()),
        creditor_bic: instruction.selected_bank.bic.clone(),
        amount: instruction.amount,
        currency: instruction.currency.clone(),
        end_to_end_id: format!("E2E{}", instruction.payment_id.to_string()[..20].to_uppercase()),
        instruction_id: instruction.id.to_string(),
    };

    info!("📤 Sending pacs.008: {} → {}", pacs008.debtor_bic, pacs008.creditor_bic);

    // TODO: Send to actual SWIFT/ISO 20022 network
    // For now, simulate successful execution
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Simulate confirmation (in reality, would wait for camt.054 or pacs.002)
    let confirmation_ref = format!("CONF{}", settlement_id.to_string()[..8].to_uppercase());

    info!("✅ Received confirmation: {}", confirmation_ref);

    Ok(SettlementResult {
        settlement_id,
        instruction_id: instruction.id,
        payment_id: instruction.payment_id,
        status: SettlementStatus::Completed,
        amount: instruction.amount,
        currency: instruction.currency.clone(),
        execution_method: "ISO20022".to_string(),
        confirmation_reference: Some(confirmation_ref),
        executed_at: Utc::now().to_rfc3339(),
        completed_at: Some(Utc::now().to_rfc3339()),
        error_message: None,
    })
}

/// Execute settlement via SWIFT
async fn execute_swift_settlement(
    instruction: &SettlementInstruction,
    settlement_id: Uuid,
) -> anyhow::Result<SettlementResult> {
    info!("📨 Sending SWIFT MT103 message");

    // TODO: Send to actual SWIFT network
    // For now, simulate successful execution
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let confirmation_ref = format!("SWIFT{}", settlement_id.to_string()[..8].to_uppercase());

    info!("✅ SWIFT confirmation: {}", confirmation_ref);

    Ok(SettlementResult {
        settlement_id,
        instruction_id: instruction.id,
        payment_id: instruction.payment_id,
        status: SettlementStatus::Completed,
        amount: instruction.amount,
        currency: instruction.currency.clone(),
        execution_method: "SWIFT".to_string(),
        confirmation_reference: Some(confirmation_ref),
        executed_at: Utc::now().to_rfc3339(),
        completed_at: Some(Utc::now().to_rfc3339()),
        error_message: None,
    })
}

/// Execute settlement via Bank API
async fn execute_api_settlement(
    instruction: &SettlementInstruction,
    settlement_id: Uuid,
) -> anyhow::Result<SettlementResult> {
    info!("🔌 Calling bank API for settlement");

    // TODO: Call actual bank API
    // Example: POST /api/v1/transfers
    // For now, simulate successful execution
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let confirmation_ref = format!("API{}", settlement_id.to_string()[..10].to_uppercase());

    info!("✅ API confirmation: {}", confirmation_ref);

    Ok(SettlementResult {
        settlement_id,
        instruction_id: instruction.id,
        payment_id: instruction.payment_id,
        status: SettlementStatus::Completed,
        amount: instruction.amount,
        currency: instruction.currency.clone(),
        execution_method: "API".to_string(),
        confirmation_reference: Some(confirmation_ref),
        executed_at: Utc::now().to_rfc3339(),
        completed_at: Some(Utc::now().to_rfc3339()),
        error_message: None,
    })
}

/// Publish settlement completion event
async fn publish_settlement_completed(
    nats_client: &Client,
    result: &SettlementResult,
) -> anyhow::Result<()> {
    let subject = "deltran.settlement.completed";
    let payload = serde_json::to_vec(result)?;

    nats_client.publish(subject, payload.into()).await?;

    info!(
        "📤 Published settlement completed: {} (status: {:?})",
        result.settlement_id,
        result.status
    );

    // Also publish funding confirmation for Token Engine
    if matches!(result.status, SettlementStatus::Completed) {
        let funding_event = serde_json::json!({
            "settlement_id": result.settlement_id,
            "payment_id": result.payment_id,
            "amount": result.amount,
            "currency": result.currency,
            "confirmation_reference": result.confirmation_reference,
            "completed_at": result.completed_at,
        });

        nats_client
            .publish(
                "deltran.funding.confirmed",
                serde_json::to_vec(&funding_event)?.into(),
            )
            .await?;

        info!("📤 Published funding confirmation for Token Engine");

        // CRITICAL: Burn tokens after settlement completes (end of transaction lifecycle)
        // Token lifecycle: MINT (funding confirmed) → USE (settlement) → BURN (settlement complete)
        let burn_request = serde_json::json!({
            "burn_id": uuid::Uuid::new_v4(),
            "settlement_id": result.settlement_id,
            "payment_id": result.payment_id,
            "bank_bic": "SETTLEMENT",  // TODO: extract from instruction
            "amount": result.amount,
            "currency": result.currency,
            "reason": "SETTLEMENT_COMPLETE",
            "burned_at": chrono::Utc::now().to_rfc3339(),
        });

        nats_client
            .publish(
                "deltran.token.burn",
                serde_json::to_vec(&burn_request)?.into(),
            )
            .await?;

        info!("🔥 Published token burn request for settlement {} - transaction lifecycle complete", result.settlement_id);
    }

    Ok(())
}
