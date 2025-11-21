// NATS Consumer for Risk Engine
// Listens to deltran.risk.check and provides FX volatility assessments
// Enhanced with Settlement Path Selection: Instant Buy, Hedging, Clearing

use async_nats::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal_macros::dec;
use chrono::Utc;
// Settlement path types are defined locally in this module

#[derive(Debug, Deserialize)]
pub struct RiskCheckRequest {
    pub request_id: Uuid,
    pub payment_id: Option<Uuid>,
    pub net_position_id: Option<Uuid>,
    pub currency_pair: String, // e.g., "USD/AED", "EUR/ILS"
    pub amount: Decimal,
    pub from_currency: String,
    pub to_currency: String,
    pub execution_window_hours: Option<i32>, // Preferred execution timeframe
}

#[derive(Debug, Serialize)]
pub struct RiskAssessment {
    pub assessment_id: Uuid,
    pub request_id: Uuid,
    pub currency_pair: String,
    pub amount: Decimal,
    pub risk_score: f64, // 0-100 (0=safe, 100=very risky)
    pub volatility_score: f64, // 0-100 (current market volatility)
    pub recommended_action: RecommendedAction,
    pub settlement_path: SettlementPathDecision, // NEW: Settlement path selection
    pub recommended_window: ExecutionWindow,
    pub fx_rate_prediction: FxRatePrediction,
    pub exposure_limit_status: ExposureLimitStatus,
    pub assessed_at: String,
}

/// Settlement Path Decision - determines how transaction will be settled
#[derive(Debug, Serialize, Clone)]
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

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementPathType {
    InstantBuy,  // Liquidity Engine buys FX at best rate immediately
    FullHedge,   // Full hedge of FX exposure
    PartialHedge, // Partial hedge with clearing
    Clearing,    // Standard multilateral clearing
}

#[derive(Debug, Serialize, Clone)]
pub struct HedgeDetails {
    pub hedge_ratio: f64,       // 0.0 - 1.0
    pub instrument: String,     // e.g., "USD/AED Forward 1M"
    pub notional_amount: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClearingDetails {
    pub window_id: Option<Uuid>,
    pub expected_netting_benefit_pct: f64,
    pub time_to_settlement_ms: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RecommendedAction {
    ExecuteNow,        // Low volatility, execute immediately
    WaitForWindow,     // Wait for optimal window
    Hedge,             // High volatility, hedge recommended
    Split,             // Split into smaller amounts
    Hold,              // Too risky, hold until volatility decreases
}

#[derive(Debug, Serialize)]
pub struct ExecutionWindow {
    pub start_time: String,
    pub end_time: String,
    pub confidence_level: f64, // 0-1 (probability this is optimal)
    pub expected_volatility: f64,
}

#[derive(Debug, Serialize)]
pub struct FxRatePrediction {
    pub current_rate: Decimal,
    pub predicted_rate_1h: Decimal,
    pub predicted_rate_6h: Decimal,
    pub predicted_rate_24h: Decimal,
    pub confidence_1h: f64,
    pub confidence_6h: f64,
    pub confidence_24h: f64,
}

#[derive(Debug, Serialize)]
pub struct ExposureLimitStatus {
    pub current_exposure: Decimal,
    pub exposure_limit: Decimal,
    pub utilization_pct: f64,
    pub remaining_capacity: Decimal,
    pub within_limits: bool,
}

pub async fn start_risk_consumer(nats_url: &str) -> anyhow::Result<()> {
    info!("⚠️  Starting Risk Engine NATS consumer...");

    // Connect to NATS
    let nats_client = async_nats::connect(nats_url).await?;
    info!("✅ Connected to NATS: {}", nats_url);

    // Subscribe to risk check topic
    let mut subscriber = nats_client.subscribe("deltran.risk.check").await?;
    info!("📡 Subscribed to: deltran.risk.check");

    // Clone for spawned task
    let nats_for_publish = nats_client.clone();

    // Spawn consumer task
    tokio::spawn(async move {
        info!("🔄 Risk assessment consumer task started");

        while let Some(msg) = subscriber.next().await {
            match serde_json::from_slice::<RiskCheckRequest>(&msg.payload) {
                Ok(request) => {
                    info!(
                        "⚠️  Received risk check request: {} for {} {} ({})",
                        request.request_id,
                        request.amount,
                        request.currency_pair,
                        request.from_currency
                    );

                    // Perform risk assessment
                    match assess_risk(&request).await {
                        Ok(assessment) => {
                            info!(
                                "✅ Risk assessment complete: {} - Risk score: {:.2}, Action: {:?}",
                                assessment.assessment_id,
                                assessment.risk_score,
                                assessment.recommended_action
                            );

                            // Publish assessment result
                            if let Err(e) = publish_risk_result(&nats_for_publish, &assessment).await {
                                error!("Failed to publish risk result: {}", e);
                            }
                        }
                        Err(e) => {
                            error!(
                                "❌ Failed to assess risk for request {}: {}",
                                request.request_id, e
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse RiskCheckRequest from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️  Risk assessment consumer task ended");
    });

    info!("✅ Risk consumer started successfully");

    Ok(())
}

/// Perform comprehensive risk assessment
async fn assess_risk(request: &RiskCheckRequest) -> anyhow::Result<RiskAssessment> {
    info!(
        "🔍 Assessing risk for {} {} ({})",
        request.amount,
        request.currency_pair,
        request.request_id
    );

    // 1. Calculate volatility score (based on 15-year minutely data)
    let volatility_score = calculate_volatility_score(&request.currency_pair).await;

    // 2. Predict FX rates for different horizons
    let fx_prediction = predict_fx_rates(&request.currency_pair, request.amount).await;

    // 3. Check exposure limits
    let exposure_status = check_exposure_limits(&request.from_currency, request.amount).await;

    // 4. Calculate overall risk score
    let risk_score = calculate_risk_score(volatility_score, &fx_prediction, &exposure_status);

    // 5. Determine recommended action (legacy)
    let recommended_action = determine_recommended_action(risk_score, volatility_score, &exposure_status);

    // 6. NEW: Select optimal settlement path (Instant Buy, Hedging, or Clearing)
    let settlement_path = select_settlement_path(
        request.amount,
        &request.from_currency,
        &request.to_currency,
        volatility_score,
        risk_score,
        &exposure_status,
        &fx_prediction,
    ).await;

    info!(
        "🎯 Settlement path selected: {:?} (confidence: {:.2})",
        settlement_path.path_type,
        settlement_path.confidence
    );

    // 7. Find optimal execution window
    let recommended_window = find_optimal_window(&request.currency_pair, risk_score).await;

    Ok(RiskAssessment {
        assessment_id: Uuid::new_v4(),
        request_id: request.request_id,
        currency_pair: request.currency_pair.clone(),
        amount: request.amount,
        risk_score,
        volatility_score,
        recommended_action,
        settlement_path,
        recommended_window,
        fx_rate_prediction: fx_prediction,
        exposure_limit_status: exposure_status,
        assessed_at: Utc::now().to_rfc3339(),
    })
}

/// Calculate volatility score based on historical data
async fn calculate_volatility_score(currency_pair: &str) -> f64 {
    // TODO: Implement real volatility calculation using 15-year minutely data
    // For now, use mock data based on currency pair

    let score = match currency_pair {
        "USD/AED" => 15.0,  // Very stable (pegged)
        "USD/ILS" => 35.0,  // Moderate volatility
        "EUR/USD" => 25.0,  // Low-moderate volatility
        "EUR/ILS" => 45.0,  // Higher volatility
        "GBP/USD" => 40.0,  // Moderate-high volatility
        _ => 50.0,          // Unknown, assume moderate-high
    };

    info!("📊 Volatility score for {}: {:.2}", currency_pair, score);

    score
}

/// Predict FX rates using ML models
async fn predict_fx_rates(currency_pair: &str, amount: Decimal) -> FxRatePrediction {
    // TODO: Implement real ML-based prediction
    // For now, use mock predictions with slight variations

    let base_rate = match currency_pair {
        "USD/AED" => Decimal::from_str_exact("3.6725").unwrap(),
        "USD/ILS" => Decimal::from_str_exact("3.75").unwrap(),
        "EUR/USD" => Decimal::from_str_exact("1.08").unwrap(),
        "EUR/ILS" => Decimal::from_str_exact("4.05").unwrap(),
        _ => Decimal::from(1),
    };

    // Simulate slight variations based on volatility
    let variation = Decimal::from_str_exact("0.005").unwrap();

    FxRatePrediction {
        current_rate: base_rate,
        predicted_rate_1h: base_rate + variation,
        predicted_rate_6h: base_rate - variation * Decimal::from(2),
        predicted_rate_24h: base_rate + variation * Decimal::from(3),
        confidence_1h: 0.95,
        confidence_6h: 0.85,
        confidence_24h: 0.70,
    }
}

/// Check exposure limits for currency
async fn check_exposure_limits(currency: &str, amount: Decimal) -> ExposureLimitStatus {
    // TODO: Implement real exposure tracking from database
    // For now, use mock limits

    let exposure_limit = match currency {
        "USD" => Decimal::from(50_000_000), // $50M limit
        "EUR" => Decimal::from(40_000_000), // €40M limit
        "AED" => Decimal::from(100_000_000), // AED 100M limit
        "ILS" => Decimal::from(150_000_000), // ILS 150M limit
        _ => Decimal::from(10_000_000),      // Default $10M
    };

    // Mock current exposure (would come from database)
    let current_exposure = match currency {
        "USD" => Decimal::from(25_000_000), // $25M current
        "EUR" => Decimal::from(15_000_000), // €15M current
        "AED" => Decimal::from(45_000_000), // AED 45M current
        "ILS" => Decimal::from(80_000_000), // ILS 80M current
        _ => Decimal::from(5_000_000),
    };

    let new_exposure = current_exposure + amount;
    let utilization_pct = (new_exposure / exposure_limit * Decimal::from(100))
        .to_f64()
        .unwrap_or(0.0);
    let within_limits = new_exposure <= exposure_limit;
    let remaining_capacity = exposure_limit - new_exposure;

    ExposureLimitStatus {
        current_exposure: new_exposure,
        exposure_limit,
        utilization_pct,
        remaining_capacity,
        within_limits,
    }
}

/// Calculate overall risk score
fn calculate_risk_score(
    volatility_score: f64,
    fx_prediction: &FxRatePrediction,
    exposure_status: &ExposureLimitStatus,
) -> f64 {
    // Weighted risk calculation
    let mut risk = 0.0;

    // 40% weight on volatility
    risk += volatility_score * 0.4;

    // 30% weight on FX prediction uncertainty
    let fx_uncertainty = (1.0 - fx_prediction.confidence_6h) * 100.0;
    risk += fx_uncertainty * 0.3;

    // 30% weight on exposure utilization
    let exposure_risk = exposure_status.utilization_pct * 0.3;
    risk += exposure_risk;

    // Cap at 100
    risk.min(100.0)
}

/// Determine recommended action based on risk
fn determine_recommended_action(
    risk_score: f64,
    volatility_score: f64,
    exposure_status: &ExposureLimitStatus,
) -> RecommendedAction {
    // Check exposure first
    if !exposure_status.within_limits {
        return RecommendedAction::Hold;
    }

    // Based on risk score
    match risk_score {
        r if r < 20.0 => RecommendedAction::ExecuteNow,
        r if r < 40.0 => {
            if volatility_score < 30.0 {
                RecommendedAction::ExecuteNow
            } else {
                RecommendedAction::WaitForWindow
            }
        }
        r if r < 60.0 => RecommendedAction::WaitForWindow,
        r if r < 80.0 => {
            if exposure_status.utilization_pct > 80.0 {
                RecommendedAction::Hold
            } else {
                RecommendedAction::Hedge
            }
        }
        _ => RecommendedAction::Hold,
    }
}

/// Find optimal execution window
async fn find_optimal_window(currency_pair: &str, risk_score: f64) -> ExecutionWindow {
    // TODO: Implement ML-based window prediction
    // For now, use simple heuristics

    let now = Utc::now();
    let (hours_offset, window_hours, confidence, expected_volatility) = if risk_score < 30.0 {
        (0, 1, 0.9, 15.0) // Execute now, low volatility expected
    } else if risk_score < 50.0 {
        (2, 2, 0.8, 25.0) // Wait 2 hours, moderate volatility
    } else if risk_score < 70.0 {
        (6, 4, 0.7, 35.0) // Wait 6 hours, higher volatility
    } else {
        (12, 6, 0.6, 50.0) // Wait 12 hours, high volatility
    };

    let start_time = now + chrono::Duration::hours(hours_offset);
    let end_time = start_time + chrono::Duration::hours(window_hours);

    ExecutionWindow {
        start_time: start_time.to_rfc3339(),
        end_time: end_time.to_rfc3339(),
        confidence_level: confidence,
        expected_volatility,
    }
}

/// Publish risk assessment result
async fn publish_risk_result(
    nats_client: &Client,
    assessment: &RiskAssessment,
) -> anyhow::Result<()> {
    let subject = "deltran.risk.result";
    let payload = serde_json::to_vec(assessment)?;

    nats_client.publish(subject, payload.into()).await?;

    info!(
        "📤 Published risk result: {} (score: {:.2}, action: {:?}, path: {:?})",
        assessment.assessment_id,
        assessment.risk_score,
        assessment.recommended_action,
        assessment.settlement_path.path_type
    );

    // Also publish settlement path decision to dedicated topic for Liquidity Engine
    let path_subject = "deltran.settlement.path";
    let path_payload = serde_json::to_vec(&assessment.settlement_path)?;
    nats_client.publish(path_subject, path_payload.into()).await?;

    Ok(())
}

/// Select optimal settlement path based on market conditions and risk assessment
/// Decision Tree:
/// 1. INSTANT BUY: Low volatility + small amount + good liquidity
///    -> Liquidity Engine selects best FX rate and executes immediately
/// 2. HEDGING (Full/Partial): High volatility OR large exposure
///    -> Hedge FX risk before settlement
/// 3. CLEARING: Moderate conditions + potential netting benefit
///    -> Wait for clearing window for multilateral netting
async fn select_settlement_path(
    amount: Decimal,
    from_currency: &str,
    to_currency: &str,
    volatility_score: f64,
    risk_score: f64,
    exposure_status: &ExposureLimitStatus,
    fx_prediction: &FxRatePrediction,
) -> SettlementPathDecision {
    // Thresholds for decision
    const SMALL_AMOUNT_USD: f64 = 100_000.0;
    const LARGE_AMOUNT_USD: f64 = 1_000_000.0;
    const LOW_VOLATILITY: f64 = 25.0;
    const HIGH_VOLATILITY: f64 = 50.0;
    const EXTREME_VOLATILITY: f64 = 75.0;

    let amount_f64 = amount.to_f64().unwrap_or(0.0);

    // DECISION TREE

    // Path 1: INSTANT BUY - Low risk, small amounts, stable conditions
    if volatility_score < LOW_VOLATILITY
        && amount_f64 < SMALL_AMOUNT_USD
        && risk_score < 30.0
    {
        return SettlementPathDecision {
            path_type: SettlementPathType::InstantBuy,
            confidence: calculate_path_confidence(volatility_score, risk_score, true),
            estimated_cost_bps: estimate_instant_buy_cost(from_currency, to_currency, amount_f64),
            estimated_time_ms: 500, // Sub-second execution
            reasoning: format!(
                "Instant buy selected: Low volatility ({:.1}), small amount ({:.0}), low risk ({:.1}). \
                 Liquidity Engine will select best FX rate from available providers.",
                volatility_score, amount_f64, risk_score
            ),
            fx_provider: Some(select_best_fx_provider(from_currency, to_currency)),
            hedge_details: None,
            clearing_details: None,
        };
    }

    // Path 2: FULL HEDGE - Extreme volatility or very large amounts
    if volatility_score >= EXTREME_VOLATILITY
        || (amount_f64 > LARGE_AMOUNT_USD && volatility_score >= HIGH_VOLATILITY)
        || exposure_status.utilization_pct > 80.0
    {
        return SettlementPathDecision {
            path_type: SettlementPathType::FullHedge,
            confidence: calculate_path_confidence(volatility_score, risk_score, false),
            estimated_cost_bps: estimate_hedge_cost(1.0), // Full hedge
            estimated_time_ms: 2000, // Hedge takes ~2 seconds to execute
            reasoning: format!(
                "Full hedge required: {} volatility ({:.1}), {} exposure ({:.1}% utilization). \
                 100% of FX exposure will be hedged using forward contracts.",
                if volatility_score >= EXTREME_VOLATILITY { "Extreme" } else { "High" },
                volatility_score,
                if exposure_status.utilization_pct > 80.0 { "High" } else { "Normal" },
                exposure_status.utilization_pct
            ),
            fx_provider: None,
            hedge_details: Some(HedgeDetails {
                hedge_ratio: 1.0,
                instrument: format!("{}/{} Forward 1M", from_currency, to_currency),
                notional_amount: amount,
            }),
            clearing_details: None,
        };
    }

    // Path 3: PARTIAL HEDGE - Moderate-high volatility with large amounts
    if volatility_score >= HIGH_VOLATILITY && amount_f64 > SMALL_AMOUNT_USD {
        let hedge_ratio = calculate_optimal_hedge_ratio(volatility_score, amount_f64);

        return SettlementPathDecision {
            path_type: SettlementPathType::PartialHedge,
            confidence: calculate_path_confidence(volatility_score, risk_score, false),
            estimated_cost_bps: estimate_hedge_cost(hedge_ratio),
            estimated_time_ms: 3000, // Partial hedge + clearing setup
            reasoning: format!(
                "Partial hedge selected: Moderate-high volatility ({:.1}), significant amount ({:.0}). \
                 {:.0}% hedged via forwards, remainder through clearing for netting benefit.",
                volatility_score, amount_f64, hedge_ratio * 100.0
            ),
            fx_provider: None,
            hedge_details: Some(HedgeDetails {
                hedge_ratio,
                instrument: format!("{}/{} Forward 1M", from_currency, to_currency),
                notional_amount: amount * Decimal::from_f64_retain(hedge_ratio).unwrap_or(dec!(0.5)),
            }),
            clearing_details: Some(ClearingDetails {
                window_id: None, // Will be assigned by Clearing Engine
                expected_netting_benefit_pct: estimate_netting_benefit(amount_f64),
                time_to_settlement_ms: 300_000, // ~5 minutes to clearing
            }),
        };
    }

    // Path 4: CLEARING - Default path for moderate conditions
    // Benefits from multilateral netting, lower costs
    SettlementPathDecision {
        path_type: SettlementPathType::Clearing,
        confidence: calculate_path_confidence(volatility_score, risk_score, true),
        estimated_cost_bps: estimate_clearing_cost(amount_f64),
        estimated_time_ms: estimate_clearing_time(amount_f64),
        reasoning: format!(
            "Clearing selected: Moderate volatility ({:.1}), acceptable risk ({:.1}). \
             Transaction will be netted with counterparty flows for optimal settlement cost.",
            volatility_score, risk_score
        ),
        fx_provider: None,
        hedge_details: None,
        clearing_details: Some(ClearingDetails {
            window_id: None,
            expected_netting_benefit_pct: estimate_netting_benefit(amount_f64),
            time_to_settlement_ms: estimate_clearing_time(amount_f64),
        }),
    }
}

/// Calculate confidence in path selection
fn calculate_path_confidence(volatility: f64, risk: f64, is_favorable: bool) -> f64 {
    let base = if is_favorable { 0.85 } else { 0.70 };

    // Higher volatility = lower confidence
    let vol_penalty = (volatility / 100.0) * 0.2;

    // Higher risk = lower confidence
    let risk_penalty = (risk / 100.0) * 0.1;

    (base - vol_penalty - risk_penalty).max(0.5).min(0.99)
}

/// Estimate instant buy cost in basis points
fn estimate_instant_buy_cost(from: &str, to: &str, amount: f64) -> i32 {
    let base_spread = match (from, to) {
        ("USD", "AED") | ("AED", "USD") => 3,  // Pegged, tight spread
        ("EUR", "USD") | ("USD", "EUR") => 5,  // Major pair
        ("INR", "AED") | ("AED", "INR") => 15, // Emerging market
        _ => 20,
    };

    // Add slippage for larger amounts
    let slippage = (amount / 100_000.0 * 2.0) as i32;

    base_spread + slippage.min(10)
}

/// Select best FX provider for the corridor
fn select_best_fx_provider(from: &str, to: &str) -> String {
    // In production, this would query Liquidity Router for best rate
    match (from, to) {
        ("USD", "AED") | ("AED", "USD") => "ENBD-FX".to_string(),
        ("EUR", "USD") | ("USD", "EUR") => "CLS-Settlement".to_string(),
        ("INR", "AED") | ("AED", "INR") => "UAE-Exchange".to_string(),
        ("INR", "USD") | ("USD", "INR") => "HDFC-FX".to_string(),
        _ => "GlobalFX-Pool".to_string(),
    }
}

/// Estimate hedge cost in basis points
fn estimate_hedge_cost(hedge_ratio: f64) -> i32 {
    // Base cost 5 bps + ratio-based component
    5 + (hedge_ratio * 10.0) as i32
}

/// Calculate optimal hedge ratio based on conditions
fn calculate_optimal_hedge_ratio(volatility: f64, amount: f64) -> f64 {
    let vol_factor = (volatility - 50.0) / 50.0; // 0 at 50%, 1 at 100%
    let amount_factor = (amount / 1_000_000.0).min(1.0);

    (0.5 + vol_factor * 0.3 + amount_factor * 0.2).min(0.9).max(0.3)
}

/// Estimate netting benefit percentage
fn estimate_netting_benefit(amount: f64) -> f64 {
    // Larger amounts benefit more from netting
    let base_benefit = 15.0; // 15% base netting benefit
    let size_bonus = (amount / 500_000.0 * 10.0).min(20.0);

    base_benefit + size_bonus
}

/// Estimate clearing cost in basis points
fn estimate_clearing_cost(amount: f64) -> i32 {
    let base = 8; // 8 bps base
    let netting_discount = (amount / 200_000.0) as i32;

    (base - netting_discount.min(5)).max(3)
}

/// Estimate time to clearing settlement in milliseconds
fn estimate_clearing_time(amount: f64) -> u64 {
    // Base: 5 minutes, larger amounts may get priority
    let base_ms: u64 = 300_000;
    let priority_reduction = if amount > 500_000.0 { 60_000 } else { 0 };

    base_ms - priority_reduction
}
