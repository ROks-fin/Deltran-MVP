//! FX rate prediction using ML
//!
//! Foundation for ML-based FX rate forecasting trained on 10 years of historical ticks.
//! This module provides the interface and basic infrastructure. Full ML training will be
//! implemented using external tools (Python/TensorFlow or Rust ML frameworks).

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FX rate at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRate {
    pub currency_pair: String,
    pub rate: Decimal,
    pub timestamp: DateTime<Utc>,
}

/// FX rate prediction with confidence interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxPrediction {
    pub currency_pair: String,
    pub predicted_rate: Decimal,
    pub confidence: f64, // 0.0-1.0
    pub prediction_horizon_hours: i64,
    pub lower_bound: Decimal, // 95% confidence interval
    pub upper_bound: Decimal,
    pub predicted_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
}

impl FxPrediction {
    /// Check if prediction is still valid
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.valid_until
    }

    /// Calculate potential exposure based on prediction
    pub fn calculate_exposure(&self, position_amount: Decimal) -> ExposureEstimate {
        let expected_value = position_amount * self.predicted_rate;
        let worst_case = position_amount * self.lower_bound;
        let best_case = position_amount * self.upper_bound;
        let potential_loss = expected_value - worst_case;
        let potential_gain = best_case - expected_value;

        ExposureEstimate {
            expected_value,
            worst_case,
            best_case,
            potential_loss,
            potential_gain,
            confidence: self.confidence,
        }
    }
}

/// Exposure estimate based on FX prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureEstimate {
    pub expected_value: Decimal,
    pub worst_case: Decimal,
    pub best_case: Decimal,
    pub potential_loss: Decimal,
    pub potential_gain: Decimal,
    pub confidence: f64,
}

/// Historical tick data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickData {
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Option<Decimal>,
}

/// FX predictor using ML models
pub struct FxPredictor {
    // Map: currency_pair -> latest prediction
    predictions_cache: HashMap<String, FxPrediction>,
    // Map: currency_pair -> current spot rate
    spot_rates: HashMap<String, Decimal>,
    // Flag: whether ML models are loaded
    models_loaded: bool,
}

impl FxPredictor {
    /// Create new FX predictor
    pub fn new() -> Self {
        Self {
            predictions_cache: HashMap::new(),
            spot_rates: HashMap::new(),
            models_loaded: false,
        }
    }

    /// Load ML models from file system (stub for future ML integration)
    pub async fn load_models(&mut self, _model_path: &str) -> Result<()> {
        // TODO: Load trained ML models
        // - LSTM/Transformer models for each major currency pair
        // - Feature extractors (technical indicators, volatility, etc.)
        // - Scaling parameters from training

        self.models_loaded = true;
        Ok(())
    }

    /// Update spot rate for a currency pair
    pub fn update_spot_rate(&mut self, currency_pair: &str, rate: Decimal) {
        self.spot_rates.insert(currency_pair.to_string(), rate);
    }

    /// Get current spot rate
    pub fn get_spot_rate(&self, currency_pair: &str) -> Option<Decimal> {
        self.spot_rates.get(currency_pair).copied()
    }

    /// Predict FX rate for given horizon (stub - will use ML models)
    pub async fn predict(
        &mut self,
        currency_pair: &str,
        horizon_hours: i64,
    ) -> Result<FxPrediction> {
        // For MVP: Use simple heuristic based on spot rate
        // In production: Run inference on trained ML model

        let spot_rate = self.spot_rates
            .get(currency_pair)
            .ok_or_else(|| Error::ModelError(format!("No spot rate for {}", currency_pair)))?;

        // Simple volatility estimate (in production: from ML model)
        let volatility = Decimal::from_str_exact("0.02").unwrap(); // 2% volatility

        let predicted_rate = *spot_rate;
        let confidence = 0.75; // In production: from model confidence

        // Confidence interval (±2 std devs for ~95% confidence)
        let interval = *spot_rate * volatility * Decimal::from(2);
        let lower_bound = *spot_rate - interval;
        let upper_bound = *spot_rate + interval;

        let now = Utc::now();
        let prediction = FxPrediction {
            currency_pair: currency_pair.to_string(),
            predicted_rate,
            confidence,
            prediction_horizon_hours: horizon_hours,
            lower_bound,
            upper_bound,
            predicted_at: now,
            valid_until: now + chrono::Duration::hours(horizon_hours),
        };

        // Cache prediction
        self.predictions_cache.insert(currency_pair.to_string(), prediction.clone());

        Ok(prediction)
    }

    /// Get cached prediction if still valid
    pub fn get_cached_prediction(&self, currency_pair: &str) -> Option<&FxPrediction> {
        self.predictions_cache
            .get(currency_pair)
            .filter(|p| p.is_valid())
    }

    /// Assess FX risk for a cross-border payment
    pub async fn assess_fx_risk(
        &mut self,
        source_currency: &str,
        target_currency: &str,
        amount: Decimal,
        settlement_horizon_hours: i64,
    ) -> Result<FxRiskAssessment> {
        let currency_pair = format!("{}/{}", source_currency, target_currency);

        // Get or create prediction
        let prediction = if let Some(cached) = self.get_cached_prediction(&currency_pair) {
            cached.clone()
        } else {
            self.predict(&currency_pair, settlement_horizon_hours).await?
        };

        let exposure = prediction.calculate_exposure(amount);

        // Calculate risk score (0-100)
        let volatility_pct = ((prediction.upper_bound - prediction.lower_bound) / prediction.predicted_rate)
            .to_string()
            .parse::<f64>()
            .unwrap_or(0.0);

        let risk_score = (volatility_pct * 100.0).min(100.0) as u8;

        Ok(FxRiskAssessment {
            currency_pair,
            source_amount: amount,
            prediction,
            exposure,
            risk_score,
            recommendation: Self::generate_recommendation(risk_score, &exposure),
        })
    }

    fn generate_recommendation(risk_score: u8, exposure: &ExposureEstimate) -> String {
        if risk_score >= 80 {
            format!(
                "HIGH RISK: Consider hedging. Potential loss: {} ({}% confidence)",
                exposure.potential_loss,
                (exposure.confidence * 100.0) as u8
            )
        } else if risk_score >= 50 {
            format!(
                "MEDIUM RISK: Monitor closely. Potential loss: {}",
                exposure.potential_loss
            )
        } else {
            format!("LOW RISK: Proceed with standard monitoring")
        }
    }

    /// Train models on historical data (placeholder for ML training pipeline)
    /// This would typically be done offline in Python/TensorFlow
    pub async fn train_on_historical_data(&mut self, _ticks: Vec<TickData>) -> Result<()> {
        // TODO: Implement ML training
        // 1. Feature engineering (technical indicators, lags, etc.)
        // 2. Train LSTM/Transformer on 10 years of ticks
        // 3. Validate on hold-out set
        // 4. Export model weights

        Err(Error::ModelError("Training not yet implemented. Use external ML pipeline.".to_string()))
    }
}

impl Default for FxPredictor {
    fn default() -> Self {
        Self::new()
    }
}

/// FX risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRiskAssessment {
    pub currency_pair: String,
    pub source_amount: Decimal,
    pub prediction: FxPrediction,
    pub exposure: ExposureEstimate,
    pub risk_score: u8, // 0-100
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fx_predictor_basic() {
        let mut predictor = FxPredictor::new();

        // Set spot rate
        let spot_rate = Decimal::from_str_exact("1.1234").unwrap();
        predictor.update_spot_rate("EUR/USD", spot_rate);

        // Get prediction
        let prediction = predictor.predict("EUR/USD", 24).await.unwrap();

        assert_eq!(prediction.currency_pair, "EUR/USD");
        assert!(prediction.is_valid());
        assert!(prediction.lower_bound < prediction.predicted_rate);
        assert!(prediction.upper_bound > prediction.predicted_rate);
    }

    #[tokio::test]
    async fn test_exposure_calculation() {
        let prediction = FxPrediction {
            currency_pair: "EUR/USD".to_string(),
            predicted_rate: Decimal::from_str_exact("1.10").unwrap(),
            confidence: 0.85,
            prediction_horizon_hours: 24,
            lower_bound: Decimal::from_str_exact("1.08").unwrap(),
            upper_bound: Decimal::from_str_exact("1.12").unwrap(),
            predicted_at: Utc::now(),
            valid_until: Utc::now() + chrono::Duration::hours(24),
        };

        let amount = Decimal::from(100_000);
        let exposure = prediction.calculate_exposure(amount);

        assert_eq!(exposure.expected_value, Decimal::from(110_000));
        assert!(exposure.potential_loss > Decimal::ZERO);
        assert!(exposure.potential_gain > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_cached_prediction() {
        let mut predictor = FxPredictor::new();
        predictor.update_spot_rate("GBP/USD", Decimal::from_str_exact("1.25").unwrap());

        // First prediction
        let pred1 = predictor.predict("GBP/USD", 24).await.unwrap();

        // Should get cached version
        let cached = predictor.get_cached_prediction("GBP/USD").unwrap();
        assert_eq!(cached.predicted_at, pred1.predicted_at);
    }
}
