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

// ============================================================
// MULTI-CURRENCY FX OPTIMIZATION (for International Payments)
// ============================================================

/// FX Partner - liquidity provider with quotes
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FxPartner {
    pub partner_id: Uuid,
    pub partner_code: String,
    pub partner_name: String,
    pub jurisdiction: String,           // Country/region
    pub supported_pairs: Vec<String>,   // ["USD/AED", "EUR/USD", ...]
    pub is_active: bool,
    pub priority: i32,                  // Lower = higher priority
}

/// FX Quote from a partner
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FxQuote {
    pub quote_id: Uuid,
    pub partner_id: Uuid,
    pub partner_code: String,
    pub currency_pair: String,          // "USD/AED"
    pub bid_rate: Decimal,              // We sell at this rate
    pub ask_rate: Decimal,              // We buy at this rate
    pub spread_bps: i32,
    pub min_amount: Decimal,
    pub max_amount: Decimal,
    pub available_liquidity: Decimal,
    pub valid_until: DateTime<Utc>,
    pub execution_time_ms: u64,
}

/// Multi-hop FX route (e.g., INR → USD → AED)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FxRoute {
    pub route_id: Uuid,
    pub from_currency: String,
    pub to_currency: String,
    pub hops: Vec<FxHop>,
    pub total_rate: Decimal,            // Combined rate
    pub total_cost_bps: i32,
    pub total_execution_time_ms: u64,
    pub confidence: f64,
}

/// Single hop in FX route
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FxHop {
    pub hop_number: i32,
    pub from_currency: String,
    pub to_currency: String,
    pub quote: FxQuote,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
}

/// FX Optimization Request (from Risk Engine)
#[derive(Debug, Deserialize, Clone)]
pub struct FxOptimizationRequest {
    pub request_id: Uuid,
    pub payment_id: Uuid,
    pub obligation_id: Uuid,
    pub from_currency: String,
    pub to_currency: String,
    pub amount: Decimal,
    pub sender_jurisdiction: String,
    pub receiver_jurisdiction: String,
    pub settlement_path: String,        // "INSTANT_BUY", "CLEARING", etc.
    pub max_cost_bps: Option<i32>,      // Max acceptable cost
    pub max_execution_time_ms: Option<u64>,
}

/// FX Optimization Result
#[derive(Debug, Serialize, Clone)]
pub struct FxOptimizationResult {
    pub request_id: Uuid,
    pub payment_id: Uuid,
    pub best_route: FxRoute,
    pub alternative_routes: Vec<FxRoute>,
    pub savings_vs_direct_bps: i32,     // How much cheaper than direct
    pub optimization_notes: String,
    pub executed: bool,
    pub execution_id: Option<Uuid>,
    pub calculated_at: DateTime<Utc>,
}

/// Partner Liquidity Snapshot
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PartnerLiquiditySnapshot {
    pub partner_id: Uuid,
    pub partner_code: String,
    pub jurisdiction: String,
    pub currencies: Vec<CurrencyLiquidity>,
    pub as_of: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurrencyLiquidity {
    pub currency: String,
    pub available_to_buy: Decimal,
    pub available_to_sell: Decimal,
    pub daily_limit: Decimal,
    pub used_today: Decimal,
}