use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Obligation status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "text", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObligationStatus {
    Pending,     // Created but not yet settled
    Netted,      // Included in netting calculation
    Settled,     // Final settlement completed
    Failed,      // Settlement failed
    Cancelled,   // Cancelled before settlement
}

/// Main Obligation structure - tracks debts between banks
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Obligation {
    pub id: Uuid,
    pub corridor: String,                   // e.g., "INR-AED"
    pub amount_sent: Decimal,              // Amount in source currency
    pub amount_credited: Decimal,          // Amount in target currency
    pub sent_currency: String,
    pub credited_currency: String,
    pub bank_debtor_id: Uuid,             // Bank that owes
    pub bank_creditor_id: Uuid,           // Bank that is owed
    pub status: String,
    pub clearing_window: i64,
    pub transaction_id: Option<Uuid>,     // Link to original transaction
    pub created_at: DateTime<Utc>,
    pub settled_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

/// Create instant obligation request
#[derive(Debug, Deserialize, Serialize, validator::Validate)]
pub struct CreateInstantObligationRequest {
    pub transaction_id: Uuid,
    pub corridor: String,
    pub amount_sent: Decimal,
    pub amount_credited: Decimal,
    pub sent_currency: String,
    pub credited_currency: String,
    pub bank_debtor_id: Uuid,
    pub bank_creditor_id: Uuid,
    pub reference: String,
    pub metadata: Option<serde_json::Value>,
}

/// Net position for a bank in a specific currency
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NetPosition {
    pub bank_id: Uuid,
    pub currency: String,
    pub net_amount: Decimal,              // Positive = to receive, Negative = to pay
    pub gross_inflow: Decimal,
    pub gross_outflow: Decimal,
    pub clearing_window: i64,
    pub calculated_at: DateTime<Utc>,
}

/// Netting result for a clearing window
#[derive(Debug, Serialize, Deserialize)]
pub struct NettingResult {
    pub clearing_window: i64,
    pub total_obligations: usize,
    pub net_positions: Vec<NetPosition>,
    pub netting_efficiency: f64,          // Percentage of reduction in fund movement
    pub gross_amount: Decimal,
    pub net_amount: Decimal,
    pub calculated_at: DateTime<Utc>,
}

/// Settlement instruction
#[derive(Debug, Serialize, Deserialize)]
pub struct SettlementInstruction {
    pub id: Uuid,
    pub clearing_window: i64,
    pub from_bank_id: Uuid,
    pub to_bank_id: Uuid,
    pub currency: String,
    pub amount: Decimal,
    pub instruction_type: SettlementType,
    pub status: SettlementStatus,
    pub created_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SettlementType {
    NetSettlement,      // After netting
    GrossSettlement,    // Direct settlement without netting
    LiquidityTransfer,  // To cover liquidity gaps
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SettlementStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Clearing window information
#[derive(Debug, Serialize, Deserialize)]
pub struct ClearingWindow {
    pub window_id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: ClearingWindowStatus,
    pub total_transactions: i64,
    pub total_obligations: i64,
    pub netting_completed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClearingWindowStatus {
    Open,
    Closing,
    Netting,
    Settling,
    Completed,
}

/// Instant settlement decision
#[derive(Debug, Serialize, Deserialize)]
pub struct InstantSettlementDecision {
    pub can_settle_instant: bool,
    pub confidence_score: f64,
    pub expected_netting_offset: Decimal,
    pub liquidity_available: Decimal,
    pub risk_score: f64,
    pub reason: Option<String>,
}

/// Obligation event for Kafka
#[derive(Debug, Serialize, Deserialize)]
pub struct ObligationEvent {
    pub event_type: ObligationEventType,
    pub obligation_id: Uuid,
    pub corridor: String,
    pub amount_sent: Decimal,
    pub amount_credited: Decimal,
    pub bank_debtor_id: Uuid,
    pub bank_creditor_id: Uuid,
    pub clearing_window: i64,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ObligationEventType {
    Created,
    Netted,
    Settled,
    Failed,
    Cancelled,
}

/// Corridor statistics for ML prediction
#[derive(Debug, Serialize, Deserialize)]
pub struct CorridorStats {
    pub corridor: String,
    pub avg_daily_volume: Decimal,
    pub avg_transaction_size: Decimal,
    pub peak_hours: Vec<i32>,
    pub bidirectional_flow_ratio: f64,    // How balanced the flows are
    pub instant_settlement_rate: f64,
    pub netting_efficiency_avg: f64,
}

/// Request to settle obligations
#[derive(Debug, Deserialize, Serialize)]
pub struct SettleObligationsRequest {
    pub clearing_window: i64,
    pub force_settlement: bool,
}

/// Response for obligation creation
#[derive(Debug, Serialize, Deserialize)]
pub struct ObligationResponse {
    pub obligation: Obligation,
    pub instant_settlement: InstantSettlementDecision,
    pub message: String,
}
