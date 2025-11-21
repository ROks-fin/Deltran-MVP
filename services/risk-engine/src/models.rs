use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===== Risk Score Model =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskScore {
    pub transaction_id: Uuid,
    pub overall_score: f64,         // 0-100
    pub factors: Vec<RiskFactor>,
    pub decision: RiskDecision,
    pub confidence: f64,             // 0-1
    pub explanation: String,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RiskFactor {
    pub name: String,
    pub weight: f64,
    pub score: f64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum RiskDecision {
    Approve,           // Low risk, proceed
    ApproveWithLimit,  // Approve but reduce limit
    Review,            // Manual review needed
    Reject,           // High risk, block
}

// ===== Transaction Risk Request =====
#[derive(Debug, Deserialize, Clone)]
pub struct RiskEvaluationRequest {
    pub transaction_id: Uuid,
    pub sender_bank_id: Uuid,
    pub receiver_bank_id: Uuid,
    pub amount: Decimal,
    pub from_currency: String,
    pub to_currency: String,
    pub sender_country: String,
    pub receiver_country: String,
    pub transaction_type: TransactionType,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TransactionType {
    B2B,      // Business to Business
    B2C,      // Business to Consumer
    C2C,      // Consumer to Consumer
    Internal, // Internal transfer
}

// ===== Dynamic Limits =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DynamicLimit {
    pub bank_id: Uuid,
    pub corridor: String,
    pub current_limit: Decimal,
    pub base_limit: Decimal,
    pub adjustment_factor: f64,
    pub reason: String,
    pub valid_until: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateLimitRequest {
    pub corridor: String,
    pub base_limit: Decimal,
}

// ===== Circuit Breaker =====
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum CircuitState {
    Closed,     // Normal operation
    Open,       // Blocking all requests
    HalfOpen,   // Testing recovery
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CircuitBreakerState {
    pub id: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub failure_threshold: u32,
    pub success_count: u32,
    pub recovery_threshold: u32,
    pub last_failure_time: Option<DateTime<Utc>>,
    pub timeout_duration: i64, // seconds
}

// ===== Risk Metrics =====
#[derive(Debug, Serialize)]
pub struct RiskMetrics {
    pub total_evaluated: u64,
    pub approved: u64,
    pub rejected: u64,
    pub under_review: u64,
    pub average_score: f64,
    pub high_risk_corridors: Vec<CorridorRisk>,
}

#[derive(Debug, Serialize)]
pub struct CorridorRisk {
    pub corridor: String,
    pub risk_level: String,
    pub transaction_count: u64,
    pub rejection_rate: f64,
}

// ===== Velocity Check =====
#[derive(Debug, Serialize, Deserialize)]
pub struct VelocityResult {
    pub hourly_count: i64,
    pub daily_count: i64,
    pub hourly_score: f64,
    pub daily_score: f64,
    pub overall_score: f64,
}

// ===== API Response =====
#[derive(Debug, Serialize)]
pub struct RiskEvaluationResponse {
    pub transaction_id: Uuid,
    pub overall_score: f64,
    pub decision: RiskDecision,
    pub confidence: f64,
    pub factors: Vec<RiskFactor>,
    pub explanation: String,
    pub calculated_at: DateTime<Utc>,
}

impl From<RiskScore> for RiskEvaluationResponse {
    fn from(score: RiskScore) -> Self {
        RiskEvaluationResponse {
            transaction_id: score.transaction_id,
            overall_score: score.overall_score,
            decision: score.decision,
            confidence: score.confidence,
            factors: score.factors,
            explanation: score.explanation,
            calculated_at: score.calculated_at,
        }
    }
}

// ===== Health Check =====
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

// ===== Error Response =====
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

// ===== Settlement Path Selection =====
/// Settlement path options based on risk assessment and market conditions
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SettlementPath {
    /// Instant liquidity buy - Liquidity Engine selects best FX rate and executes immediately
    InstantBuy {
        fx_provider: String,
        estimated_rate: Decimal,
        estimated_cost_bps: i32,
    },
    /// Hedging - Full or partial hedging through derivatives
    Hedging {
        hedge_type: HedgeType,
        hedge_ratio: Decimal,  // 0.0 - 1.0 (partial to full)
        instrument: String,
    },
    /// Standard clearing - Net through clearing window
    Clearing {
        clearing_window_id: Option<Uuid>,
        expected_netting_benefit: Decimal,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum HedgeType {
    Full,      // 100% hedge
    Partial,   // Partial hedge based on risk
    Dynamic,   // Dynamic hedging adjusted over time
}

/// Settlement path recommendation with full analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettlementPathRecommendation {
    pub transaction_id: Uuid,
    pub recommended_path: SettlementPath,
    pub alternative_paths: Vec<SettlementPathOption>,
    pub reasoning: String,
    pub confidence: f64,
    pub estimated_total_cost_bps: i32,
    pub estimated_execution_time_ms: u64,
    pub market_conditions: MarketConditions,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettlementPathOption {
    pub path: SettlementPath,
    pub score: f64,          // 0-100, higher is better
    pub cost_bps: i32,
    pub execution_time_ms: u64,
    pub risk_factors: Vec<String>,
}

/// Market conditions snapshot for path selection
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketConditions {
    pub fx_volatility: FxVolatility,
    pub liquidity_depth: LiquidityDepth,
    pub clearing_window_status: ClearingWindowStatus,
    pub counterparty_positions: CounterpartyPositions,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum FxVolatility {
    Low,      // < 0.5% daily
    Normal,   // 0.5% - 1.5% daily
    High,     // > 1.5% daily
    Extreme,  // > 3% daily
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum LiquidityDepth {
    Deep,     // Large volumes available
    Normal,   // Standard market depth
    Thin,     // Limited liquidity
    Stressed, // Market stress conditions
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ClearingWindowStatus {
    Open,          // Window accepting obligations
    ClosingSoon,   // < 15 minutes to close
    Closed,        // Window closed
    NotAvailable,  // No window for corridor
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CounterpartyPositions {
    pub has_offsetting_flow: bool,
    pub potential_netting_amount: Decimal,
    pub counterparty_count: u32,
}
