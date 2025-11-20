// NATS Consumer for Risk Engine
// Listens to deltran.risk.check and provides FX volatility assessments

use async_nats::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use chrono::Utc;

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
    pub recommended_window: ExecutionWindow,
    pub fx_rate_prediction: FxRatePrediction,
    pub exposure_limit_status: ExposureLimitStatus,
    pub assessed_at: String,
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

    // 5. Determine recommended action
    let recommended_action = determine_recommended_action(risk_score, volatility_score, &exposure_status);

    // 6. Find optimal execution window
    let recommended_window = find_optimal_window(&request.currency_pair, risk_score).await;

    Ok(RiskAssessment {
        assessment_id: Uuid::new_v4(),
        request_id: request.request_id,
        currency_pair: request.currency_pair.clone(),
        amount: request.amount,
        risk_score,
        volatility_score,
        recommended_action,
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
        "📤 Published risk result: {} (score: {:.2}, action: {:?})",
        assessment.assessment_id,
        assessment.risk_score,
        assessment.recommended_action
    );

    Ok(())
}
