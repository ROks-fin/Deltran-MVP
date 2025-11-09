use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Liquidity prediction request
#[derive(Debug, Deserialize, Serialize)]
pub struct LiquidityPredictionRequest {
    pub corridor: String,
    pub amount: Decimal,
    pub bank_id: Uuid,
    pub time_horizon_hours: i32, // How far ahead to predict
}

/// Liquidity prediction result
#[derive(Debug, Serialize, Deserialize)]
pub struct LiquidityPrediction {
    pub can_instant_settle: bool,
    pub confidence: f64,
    pub expected_counterflow: Decimal,
    pub counterflow_probability: f64,
    pub liquidity_gap: Decimal,
    pub recommended_sources: Vec<LiquiditySource>,
    pub predicted_at: DateTime<Utc>,
}

/// Liquidity source options
#[derive(Debug, Serialize, Deserialize)]
pub struct LiquiditySource {
    pub source_type: LiquiditySourceType,
    pub amount_available: Decimal,
    pub cost_bps: i32, // Cost in basis points
    pub execution_time_seconds: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LiquiditySourceType {
    Netting,           // From counterflows (0 cost)
    PartnerBank,       // From partner banks
    InterbankMarket,   // From interbank market
    MarketMaker,       // From market makers
    CreditLine,        // Emergency credit line
}

/// Corridor statistics for ML model
#[derive(Debug, Serialize, Deserialize)]
pub struct CorridorStats {
    pub corridor: String,
    pub avg_daily_volume: Decimal,
    pub avg_transaction_size: Decimal,
    pub peak_hours: Vec<i32>,
    pub bidirectional_flow_ratio: f64,
    pub instant_settlement_rate: f64,
    pub netting_efficiency_avg: f64,
    pub last_30_days_transactions: i64,
}

/// Optimization path
#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizationPath {
    pub from_currency: String,
    pub to_currency: String,
    pub route: Vec<String>, // e.g., ["INR", "USD", "AED"]
    pub total_cost_bps: i32,
    pub estimated_time_seconds: i32,
    pub fx_rates: Vec<Decimal>,
}

/// Liquidity position for a bank
#[derive(Debug, Serialize, Deserialize)]
pub struct LiquidityPosition {
    pub bank_id: Uuid,
    pub currency: String,
    pub available: Decimal,
    pub committed: Decimal,
    pub projected_inflow_24h: Decimal,
    pub projected_outflow_24h: Decimal,
    pub min_required: Decimal,
    pub as_of: DateTime<Utc>,
}