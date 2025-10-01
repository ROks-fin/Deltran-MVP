//! Shared types for adapters

use chrono::{DateTime, Utc};
use protocol_core::{NetTransfer, SettlementInstruction};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Adapter type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdapterType {
    /// SWIFT network
    Swift,
    /// Local ACH
    Ach,
    /// RTGS (Real-Time Gross Settlement)
    Rtgs,
    /// CBDC bridge
    Cbdc,
    /// Custom/Bank-specific
    Custom,
}

impl std::fmt::Display for AdapterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterType::Swift => write!(f, "SWIFT"),
            AdapterType::Ach => write!(f, "ACH"),
            AdapterType::Rtgs => write!(f, "RTGS"),
            AdapterType::Cbdc => write!(f, "CBDC"),
            AdapterType::Custom => write!(f, "CUSTOM"),
        }
    }
}

/// Transfer request (to send to bank)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    /// Transfer ID
    pub transfer_id: Uuid,
    /// Instruction (from settlement)
    pub instruction: SettlementInstruction,
    /// Corridor ID
    pub corridor_id: String,
    /// Adapter type to use
    pub adapter_type: AdapterType,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Retry count
    pub retry_count: u32,
}

/// Transfer response (from bank)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResponse {
    /// Transfer ID
    pub transfer_id: Uuid,
    /// Status
    pub status: TransferStatus,
    /// External reference (bank's transaction ID)
    pub external_reference: Option<String>,
    /// Message
    pub message: Option<String>,
    /// Completed at
    pub completed_at: DateTime<Utc>,
}

/// Transfer status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    /// Accepted by bank
    Accepted,
    /// In progress
    Pending,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Rejected (e.g., insufficient funds)
    Rejected,
}

/// Corridor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorConfig {
    /// Corridor ID (e.g., "UAE-IND")
    pub corridor_id: String,
    /// Source country
    pub source_country: String,
    /// Destination country
    pub dest_country: String,
    /// Adapter type
    pub adapter_type: AdapterType,
    /// Adapter-specific config (JSON)
    pub adapter_config: serde_json::Value,
    /// Circuit breaker enabled
    pub circuit_breaker_enabled: bool,
    /// Circuit breaker threshold
    pub circuit_breaker_threshold: u32,
    /// Kill switch enabled
    pub kill_switch_enabled: bool,
    /// DLQ enabled
    pub dlq_enabled: bool,
    /// Max retry attempts
    pub max_retry_attempts: u32,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Healthy
    Healthy,
    /// Degraded (some failures but operational)
    Degraded,
    /// Unhealthy (circuit breaker open or kill switch active)
    Unhealthy,
}

/// Adapter health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterHealth {
    /// Corridor ID
    pub corridor_id: String,
    /// Adapter type
    pub adapter_type: AdapterType,
    /// Status
    pub status: HealthStatus,
    /// Last check
    pub last_check: DateTime<Utc>,
    /// Total requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Circuit breaker state
    pub circuit_breaker_open: bool,
    /// Kill switch active
    pub kill_switch_active: bool,
    /// DLQ size
    pub dlq_size: usize,
}

impl AdapterHealth {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        self.successful_requests as f64 / self.total_requests as f64
    }

    /// Calculate failure rate
    pub fn failure_rate(&self) -> f64 {
        1.0 - self.success_rate()
    }
}