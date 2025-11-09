use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ===== CLEARING WINDOW =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClearingWindow {
    pub id: i64,
    pub window_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub cutoff_time: DateTime<Utc>,
    pub status: String,
    pub region: String,
    pub transactions_count: i32,
    pub obligations_count: i32,
    pub total_gross_value: Decimal,
    pub total_net_value: Decimal,
    pub saved_amount: Decimal,
    pub netting_efficiency: Decimal,
    pub settlement_instructions: Option<Vec<Uuid>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub processed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub grace_period_seconds: i32,
    pub grace_period_started: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowStatus {
    Scheduled,
    Open,
    Closing,
    Closed,
    Processing,
    Settling,
    Completed,
    Failed,
    RolledBack,
}

impl WindowStatus {
    pub fn as_str(&self) -> &str {
        match self {
            WindowStatus::Scheduled => "Scheduled",
            WindowStatus::Open => "Open",
            WindowStatus::Closing => "Closing",
            WindowStatus::Closed => "Closed",
            WindowStatus::Processing => "Processing",
            WindowStatus::Settling => "Settling",
            WindowStatus::Completed => "Completed",
            WindowStatus::Failed => "Failed",
            WindowStatus::RolledBack => "RolledBack",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Scheduled" => WindowStatus::Scheduled,
            "Open" => WindowStatus::Open,
            "Closing" => WindowStatus::Closing,
            "Closed" => WindowStatus::Closed,
            "Processing" => WindowStatus::Processing,
            "Settling" => WindowStatus::Settling,
            "Completed" => WindowStatus::Completed,
            "Failed" => WindowStatus::Failed,
            "RolledBack" => WindowStatus::RolledBack,
            _ => WindowStatus::Open,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClearingRegion {
    Global,
    ADGM,
    Europe,
    Americas,
    AsiaPacific,
}

impl ClearingRegion {
    pub fn as_str(&self) -> &str {
        match self {
            ClearingRegion::Global => "Global",
            ClearingRegion::ADGM => "ADGM",
            ClearingRegion::Europe => "Europe",
            ClearingRegion::Americas => "Americas",
            ClearingRegion::AsiaPacific => "AsiaPacific",
        }
    }
}

// ===== WINDOW EVENTS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WindowEvent {
    pub id: Uuid,
    pub window_id: i64,
    pub event_type: String,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub event_data: serde_json::Value,
    pub triggered_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ===== ATOMIC OPERATIONS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AtomicOperation {
    pub operation_id: Uuid,
    pub window_id: i64,
    pub operation_type: String,
    pub state: String,
    pub parent_operation_id: Option<Uuid>,
    pub checkpoints: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub rolled_back_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub rollback_data: Option<serde_json::Value>,
    pub rollback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AtomicOperationType {
    WindowClose,
    ObligationCollection,
    NettingCalculation,
    InstructionGeneration,
    SettlementInitiation,
    WindowOpen,
}

impl AtomicOperationType {
    pub fn as_str(&self) -> &str {
        match self {
            AtomicOperationType::WindowClose => "WindowClose",
            AtomicOperationType::ObligationCollection => "ObligationCollection",
            AtomicOperationType::NettingCalculation => "NettingCalculation",
            AtomicOperationType::InstructionGeneration => "InstructionGeneration",
            AtomicOperationType::SettlementInitiation => "SettlementInitiation",
            AtomicOperationType::WindowOpen => "WindowOpen",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AtomicState {
    Pending,
    InProgress,
    Committed,
    RolledBack,
    Failed,
}

impl AtomicState {
    pub fn as_str(&self) -> &str {
        match self {
            AtomicState::Pending => "Pending",
            AtomicState::InProgress => "InProgress",
            AtomicState::Committed => "Committed",
            AtomicState::RolledBack => "RolledBack",
            AtomicState::Failed => "Failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Pending" => AtomicState::Pending,
            "InProgress" => AtomicState::InProgress,
            "Committed" => AtomicState::Committed,
            "RolledBack" => AtomicState::RolledBack,
            "Failed" => AtomicState::Failed,
            _ => AtomicState::Pending,
        }
    }
}

// ===== CHECKPOINTS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OperationCheckpoint {
    pub id: Uuid,
    pub operation_id: Uuid,
    pub checkpoint_name: String,
    pub checkpoint_order: i32,
    pub checkpoint_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ===== NET POSITIONS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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
    pub created_at: DateTime<Utc>,
}

// ===== SETTLEMENT INSTRUCTIONS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SettlementInstruction {
    pub id: Uuid,
    pub window_id: i64,
    pub net_position_id: Option<Uuid>,
    pub payer_bank_id: Uuid,
    pub payee_bank_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub instruction_type: String,
    pub priority: i32,
    pub deadline: DateTime<Utc>,
    pub status: String,
    pub sent_to_settlement_at: Option<DateTime<Utc>>,
    pub settlement_id: Option<Uuid>,
    pub instruction_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ===== CLEARING METRICS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClearingMetrics {
    pub id: Uuid,
    pub window_id: i64,
    pub processing_started_at: DateTime<Utc>,
    pub processing_completed_at: Option<DateTime<Utc>>,
    pub processing_duration_ms: Option<i64>,
    pub obligations_collected: i32,
    pub obligations_netted: i32,
    pub net_positions_calculated: i32,
    pub gross_value: Decimal,
    pub net_value: Decimal,
    pub efficiency_percent: Decimal,
    pub total_saved: Decimal,
    pub instructions_generated: i32,
    pub instructions_sent: i32,
    pub errors_count: i32,
    pub warnings_count: i32,
    pub created_at: DateTime<Utc>,
}

// ===== WINDOW LOCKS =====

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WindowLock {
    pub window_id: i64,
    pub locked_by: String,
    pub locked_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub lock_token: Uuid,
}

// ===== CLEARING RESULT =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearingResult {
    pub window_id: i64,
    pub status: ClearingStatus,
    pub obligations_processed: u32,
    pub net_positions: Vec<NetPosition>,
    pub settlement_instructions: Vec<Uuid>,
    pub total_saved: Decimal,
    pub efficiency: f64,
    pub errors: Vec<String>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClearingStatus {
    Success,
    PartialSuccess,
    Failed,
}
