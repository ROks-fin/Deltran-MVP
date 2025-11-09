use crate::models::{
    CorridorStats, LiquidityPrediction, LiquiditySource, LiquiditySourceType,
};
use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::info;

/// ML-based liquidity predictor
pub struct LiquidityPredictor {
    corridor_history: HashMap<String, CorridorStats>,
}

impl LiquidityPredictor {
    pub fn new() -> Self {
        LiquidityPredictor {
            corridor_history: HashMap::new(),
        }
    }

    /// Load historical data for a corridor
    pub fn load_corridor_stats(&mut self, stats: CorridorStats) {
        self.corridor_history
            .insert(stats.corridor.clone(), stats);
    }

    /// Predict if instant settlement is feasible
    pub fn predict_instant_settlement(
        &self,
        corridor: &str,
        amount: Decimal,
    ) -> LiquidityPrediction {
        info!(
            "Predicting instant settlement for corridor {} amount {}",
            corridor, amount
        );

        // Get historical stats
        let stats = self.corridor_history.get(corridor);

        // ML prediction - simplified heuristic model
        // In production, this would use actual ML model (TensorFlow/PyTorch)
        let (can_settle, confidence, expected_counterflow, counterflow_probability) =
            if let Some(stats) = stats {
                self.ml_predict(amount, stats)
            } else {
                // No historical data - conservative prediction
                (false, 0.3, Decimal::ZERO, 0.2)
            };

        // Calculate liquidity gap
        let liquidity_gap = if expected_counterflow >= amount {
            Decimal::ZERO
        } else {
            amount - expected_counterflow
        };

        // Recommend liquidity sources
        let recommended_sources = self.recommend_liquidity_sources(liquidity_gap, corridor);

        LiquidityPrediction {
            can_instant_settle: can_settle,
            confidence,
            expected_counterflow,
            counterflow_probability,
            liquidity_gap,
            recommended_sources,
            predicted_at: Utc::now(),
        }
    }

    /// ML prediction algorithm (simplified)
    fn ml_predict(
        &self,
        amount: Decimal,
        stats: &CorridorStats,
    ) -> (bool, f64, Decimal, f64) {
        // Feature extraction
        let avg_volume = stats.avg_daily_volume;
        let bidirectional_ratio = stats.bidirectional_flow_ratio;
        let netting_efficiency = stats.netting_efficiency_avg;

        // Heuristic model (replaces actual ML in MVP)
        // Rule 1: Amount should be less than 50% of average daily volume
        let volume_score = if amount <= avg_volume * Decimal::new(5, 1) {
            0.8
        } else if amount <= avg_volume {
            0.5
        } else {
            0.2
        };

        // Rule 2: Bidirectional flow ratio (closer to 1.0 = better)
        let flow_score = bidirectional_ratio.min(1.0);

        // Rule 3: Historical netting efficiency
        let netting_score = netting_efficiency;

        // Combined confidence score
        let confidence = (volume_score + flow_score + netting_score) / 3.0;

        // Predict expected counterflow
        let expected_counterflow = amount * Decimal::new((netting_efficiency * 100.0) as i64, 2);

        // Probability of counterflow occurring
        let counterflow_probability = bidirectional_ratio * netting_efficiency;

        // Decision threshold
        let can_settle = confidence >= 0.75 && counterflow_probability >= 0.6;

        (can_settle, confidence, expected_counterflow, counterflow_probability)
    }

    /// Recommend liquidity sources to cover gap
    fn recommend_liquidity_sources(
        &self,
        gap: Decimal,
        _corridor: &str,
    ) -> Vec<LiquiditySource> {
        let mut sources = Vec::new();

        if gap <= Decimal::ZERO {
            // No gap - netting covers everything
            sources.push(LiquiditySource {
                source_type: LiquiditySourceType::Netting,
                amount_available: gap.abs(),
                cost_bps: 0,
                execution_time_seconds: 0,
            });
            return sources;
        }

        // Primary: Partner banks (cheapest)
        sources.push(LiquiditySource {
            source_type: LiquiditySourceType::PartnerBank,
            amount_available: gap / Decimal::from(2),
            cost_bps: 5, // 0.05%
            execution_time_seconds: 60,
        });

        // Secondary: Interbank market
        sources.push(LiquiditySource {
            source_type: LiquiditySourceType::InterbankMarket,
            amount_available: gap / Decimal::from(2),
            cost_bps: 10, // 0.10%
            execution_time_seconds: 300,
        });

        // Tertiary: Market makers
        sources.push(LiquiditySource {
            source_type: LiquiditySourceType::MarketMaker,
            amount_available: gap,
            cost_bps: 20, // 0.20%
            execution_time_seconds: 600,
        });

        // Emergency: Credit line
        sources.push(LiquiditySource {
            source_type: LiquiditySourceType::CreditLine,
            amount_available: gap * Decimal::from(2),
            cost_bps: 30, // 0.30%
            execution_time_seconds: 1800,
        });

        sources
    }

    /// Update model with new transaction data
    pub fn update_with_transaction(
        &mut self,
        corridor: &str,
        amount: Decimal,
        had_counterflow: bool,
    ) {
        // In production, this would update ML model weights
        info!(
            "Updating model: corridor={}, amount={}, counterflow={}",
            corridor, amount, had_counterflow
        );
    }
}