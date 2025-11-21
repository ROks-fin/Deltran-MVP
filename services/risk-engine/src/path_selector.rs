// Settlement Path Selector Module
// Determines optimal settlement path: Instant Buy, Hedging, or Clearing

use crate::errors::{RiskError, RiskResult};
use crate::models::*;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sqlx::PgPool;
use tracing::info;
use uuid::Uuid;

/// Settlement Path Selector - Chooses optimal path based on risk and market conditions
pub struct PathSelector {
    // Thresholds for path selection
    instant_buy_threshold: Decimal,      // Below this amount, prefer instant buy
    hedging_volatility_threshold: f64,   // Above this FX volatility, prefer hedging
    clearing_benefit_threshold: Decimal, // Min netting benefit to prefer clearing
}

impl PathSelector {
    pub fn new() -> Self {
        PathSelector {
            instant_buy_threshold: dec!(100000),      // $100k
            hedging_volatility_threshold: 1.5,        // 1.5% daily volatility
            clearing_benefit_threshold: dec!(0.002),  // 0.2% (20 bps)
        }
    }

    /// Select optimal settlement path for a transaction
    pub async fn select_path(
        &self,
        transaction_id: Uuid,
        amount: Decimal,
        from_currency: &str,
        to_currency: &str,
        risk_score: &RiskScore,
        pool: &PgPool,
    ) -> RiskResult<SettlementPathRecommendation> {
        // 1. Gather market conditions
        let market_conditions = self.get_market_conditions(from_currency, to_currency, pool).await?;

        // 2. Check counterparty positions for netting potential
        let counterparty = self.check_counterparty_positions(from_currency, to_currency, pool).await?;

        // 3. Score each path
        let mut paths = Vec::new();

        // Score Instant Buy path
        let instant_buy_score = self.score_instant_buy(
            amount,
            &market_conditions,
            risk_score,
        );
        paths.push(instant_buy_score);

        // Score Hedging path
        let hedging_score = self.score_hedging(
            amount,
            &market_conditions,
            risk_score,
        );
        paths.push(hedging_score);

        // Score Clearing path
        let clearing_score = self.score_clearing(
            amount,
            &market_conditions,
            &counterparty,
            risk_score,
        );
        paths.push(clearing_score);

        // 4. Sort by score (highest first)
        paths.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let recommended = paths.remove(0);
        let reasoning = self.generate_reasoning(&recommended, &market_conditions);

        info!(
            "Path selected for txn {}: {:?} (score: {:.2})",
            transaction_id, recommended.path, recommended.score
        );

        Ok(SettlementPathRecommendation {
            transaction_id,
            recommended_path: recommended.path.clone(),
            alternative_paths: paths,
            reasoning,
            confidence: self.calculate_confidence(&recommended, &market_conditions),
            estimated_total_cost_bps: recommended.cost_bps,
            estimated_execution_time_ms: recommended.execution_time_ms,
            market_conditions: MarketConditions {
                fx_volatility: market_conditions.volatility.clone(),
                liquidity_depth: market_conditions.liquidity.clone(),
                clearing_window_status: market_conditions.clearing_status.clone(),
                counterparty_positions: counterparty,
            },
            calculated_at: Utc::now(),
        })
    }

    /// Score Instant Buy path
    fn score_instant_buy(
        &self,
        amount: Decimal,
        market: &InternalMarketConditions,
        risk_score: &RiskScore,
    ) -> SettlementPathOption {
        let mut score: f64 = 50.0; // Base score
        let mut risk_factors = Vec::new();

        // Favor instant buy for smaller amounts
        if amount <= self.instant_buy_threshold {
            score += 30.0;
            risk_factors.push("Small transaction size favors instant execution".to_string());
        } else if amount <= self.instant_buy_threshold * dec!(5) {
            score += 15.0;
        } else {
            score -= 10.0;
            risk_factors.push("Large amount may face slippage".to_string());
        }

        // Favor when liquidity is deep
        match market.liquidity {
            LiquidityDepth::Deep => {
                score += 20.0;
                risk_factors.push("Deep market liquidity".to_string());
            }
            LiquidityDepth::Normal => score += 10.0,
            LiquidityDepth::Thin => {
                score -= 20.0;
                risk_factors.push("Thin liquidity - potential slippage".to_string());
            }
            LiquidityDepth::Stressed => {
                score -= 40.0;
                risk_factors.push("Stressed market - avoid instant buy".to_string());
            }
        }

        // Disfavor when volatility is high
        match market.volatility {
            FxVolatility::Low => score += 15.0,
            FxVolatility::Normal => score += 5.0,
            FxVolatility::High => {
                score -= 15.0;
                risk_factors.push("High FX volatility".to_string());
            }
            FxVolatility::Extreme => {
                score -= 30.0;
                risk_factors.push("Extreme volatility - instant buy risky".to_string());
            }
        }

        // Favor for low-risk transactions
        if risk_score.overall_score < 25.0 {
            score += 10.0;
        }

        let cost_bps = market.estimated_spread_bps + (amount.to_string().parse::<f64>().unwrap_or(0.0) / 100000.0 * 2.0) as i32;

        SettlementPathOption {
            path: SettlementPath::InstantBuy {
                fx_provider: market.best_fx_provider.clone(),
                estimated_rate: market.estimated_rate,
                estimated_cost_bps: cost_bps,
            },
            score: score.max(0.0).min(100.0),
            cost_bps,
            execution_time_ms: 500, // Fast execution
            risk_factors,
        }
    }

    /// Score Hedging path
    fn score_hedging(
        &self,
        amount: Decimal,
        market: &InternalMarketConditions,
        risk_score: &RiskScore,
    ) -> SettlementPathOption {
        let mut score: f64 = 30.0; // Base score (hedging is more complex)
        let mut risk_factors = Vec::new();

        // Favor hedging for larger amounts
        if amount > self.instant_buy_threshold * dec!(5) {
            score += 25.0;
            risk_factors.push("Large amount benefits from hedging".to_string());
        } else if amount > self.instant_buy_threshold {
            score += 15.0;
        }

        // Strongly favor when volatility is high
        match market.volatility {
            FxVolatility::Low => score -= 10.0,
            FxVolatility::Normal => {}
            FxVolatility::High => {
                score += 30.0;
                risk_factors.push("High volatility - hedging recommended".to_string());
            }
            FxVolatility::Extreme => {
                score += 40.0;
                risk_factors.push("Extreme volatility - full hedge recommended".to_string());
            }
        }

        // Determine hedge type based on conditions
        let (hedge_type, hedge_ratio) = match (market.volatility.clone(), amount > self.instant_buy_threshold * dec!(10)) {
            (FxVolatility::Extreme, _) => (HedgeType::Full, dec!(1.0)),
            (FxVolatility::High, true) => (HedgeType::Full, dec!(1.0)),
            (FxVolatility::High, false) => (HedgeType::Partial, dec!(0.75)),
            (FxVolatility::Normal, true) => (HedgeType::Partial, dec!(0.5)),
            _ => (HedgeType::Dynamic, dec!(0.3)),
        };

        // Higher risk score favors hedging
        if risk_score.overall_score > 50.0 {
            score += 15.0;
            risk_factors.push("Elevated risk score suggests hedging".to_string());
        }

        // Hedging cost calculation (roughly 5-15 bps depending on hedge ratio)
        let base_hedge_cost = 5 + (hedge_ratio.to_string().parse::<f64>().unwrap_or(0.5) * 10.0) as i32;

        SettlementPathOption {
            path: SettlementPath::Hedging {
                hedge_type,
                hedge_ratio,
                instrument: format!("{}/{} Forward", market.from_currency, market.to_currency),
            },
            score: score.max(0.0).min(100.0),
            cost_bps: base_hedge_cost,
            execution_time_ms: 2000, // Hedging takes longer
            risk_factors,
        }
    }

    /// Score Clearing path
    fn score_clearing(
        &self,
        amount: Decimal,
        market: &InternalMarketConditions,
        counterparty: &CounterpartyPositions,
        _risk_score: &RiskScore,
    ) -> SettlementPathOption {
        let mut score: f64 = 40.0; // Base score
        let mut risk_factors = Vec::new();

        // Check if clearing window is available
        match market.clearing_status {
            ClearingWindowStatus::Open => {
                score += 20.0;
                risk_factors.push("Clearing window open".to_string());
            }
            ClearingWindowStatus::ClosingSoon => {
                score += 5.0;
                risk_factors.push("Window closing soon - limited time".to_string());
            }
            ClearingWindowStatus::Closed | ClearingWindowStatus::NotAvailable => {
                score -= 50.0; // Heavily penalize if no window
                risk_factors.push("No clearing window available".to_string());
            }
        }

        // Favor clearing if counterparty has offsetting flow
        if counterparty.has_offsetting_flow {
            let netting_ratio = counterparty.potential_netting_amount / amount;
            if netting_ratio >= self.clearing_benefit_threshold {
                score += 35.0;
                risk_factors.push(format!(
                    "Potential netting benefit: {} with {} counterparties",
                    counterparty.potential_netting_amount, counterparty.counterparty_count
                ));
            }
        }

        // Larger amounts benefit more from clearing
        if amount > self.instant_buy_threshold * dec!(3) {
            score += 15.0;
            risk_factors.push("Large amount benefits from multilateral netting".to_string());
        }

        // Low volatility favors clearing (can wait for window)
        match market.volatility {
            FxVolatility::Low => {
                score += 15.0;
                risk_factors.push("Low volatility - safe to wait for clearing".to_string());
            }
            FxVolatility::Normal => score += 5.0,
            _ => {
                score -= 10.0;
                risk_factors.push("Elevated volatility while waiting for clearing".to_string());
            }
        }

        // Clearing cost is typically lower due to netting
        let netting_discount = if counterparty.has_offsetting_flow { 5 } else { 0 };
        let clearing_cost = (10 - netting_discount).max(2);

        SettlementPathOption {
            path: SettlementPath::Clearing {
                clearing_window_id: market.current_window_id,
                expected_netting_benefit: counterparty.potential_netting_amount,
            },
            score: score.max(0.0).min(100.0),
            cost_bps: clearing_cost,
            execution_time_ms: market.time_to_window_close_ms.unwrap_or(300_000), // Time to clearing
            risk_factors,
        }
    }

    /// Get current market conditions
    #[allow(unused_variables)]
    async fn get_market_conditions(
        &self,
        from_currency: &str,
        to_currency: &str,
        pool: &PgPool,
    ) -> RiskResult<InternalMarketConditions> {
        // In production, this would fetch real-time market data
        // For now, simulate based on corridor

        let (volatility, liquidity, spread) = match (from_currency, to_currency) {
            ("USD", "EUR") | ("EUR", "USD") => (FxVolatility::Low, LiquidityDepth::Deep, 5),
            ("USD", "GBP") | ("GBP", "USD") => (FxVolatility::Low, LiquidityDepth::Deep, 6),
            ("INR", "AED") | ("AED", "INR") => (FxVolatility::Normal, LiquidityDepth::Normal, 15),
            ("INR", "USD") | ("USD", "INR") => (FxVolatility::Normal, LiquidityDepth::Normal, 12),
            _ => (FxVolatility::Normal, LiquidityDepth::Normal, 20),
        };

        // Check for active clearing window
        let window_result: Option<(Uuid, i64)> = sqlx::query_as(
            r#"SELECT id, EXTRACT(EPOCH FROM (close_time - NOW()))::BIGINT as seconds_remaining
               FROM clearing_windows
               WHERE status = 'OPEN'
               AND region = 'Global'
               ORDER BY close_time ASC
               LIMIT 1"#,
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        let (clearing_status, window_id, time_to_close) = match window_result {
            Some((id, remaining)) if remaining > 900 => (ClearingWindowStatus::Open, Some(id), Some(remaining as u64 * 1000)),
            Some((id, remaining)) if remaining > 0 => (ClearingWindowStatus::ClosingSoon, Some(id), Some(remaining as u64 * 1000)),
            _ => (ClearingWindowStatus::Closed, None, None),
        };

        // Get estimated FX rate (simplified)
        let estimated_rate = self.get_estimated_rate(from_currency, to_currency);

        Ok(InternalMarketConditions {
            from_currency: from_currency.to_string(),
            to_currency: to_currency.to_string(),
            volatility,
            liquidity,
            clearing_status,
            current_window_id: window_id,
            time_to_window_close_ms: time_to_close,
            estimated_rate,
            estimated_spread_bps: spread,
            best_fx_provider: "GlobalFX".to_string(), // Would come from Liquidity Router
        })
    }

    /// Check counterparty positions for netting potential
    async fn check_counterparty_positions(
        &self,
        from_currency: &str,
        to_currency: &str,
        pool: &PgPool,
    ) -> RiskResult<CounterpartyPositions> {
        // Check for offsetting obligations in current window
        let result: Option<(i64, rust_decimal::Decimal)> = sqlx::query_as(
            r#"SELECT COUNT(DISTINCT creditor_bank_id) as counterparties,
                      COALESCE(SUM(amount), 0) as potential_netting
               FROM obligations o
               JOIN clearing_windows w ON o.window_id = w.id
               WHERE w.status = 'OPEN'
               AND o.currency = $1
               AND o.status = 'PENDING'"#,
        )
        .bind(to_currency)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        match result {
            Some((count, amount)) if count > 0 => Ok(CounterpartyPositions {
                has_offsetting_flow: true,
                potential_netting_amount: amount,
                counterparty_count: count as u32,
            }),
            _ => Ok(CounterpartyPositions {
                has_offsetting_flow: false,
                potential_netting_amount: dec!(0),
                counterparty_count: 0,
            }),
        }
    }

    /// Get estimated FX rate
    fn get_estimated_rate(&self, from: &str, to: &str) -> Decimal {
        match (from, to) {
            ("INR", "AED") => dec!(0.044),
            ("AED", "INR") => dec!(22.73),
            ("USD", "AED") => dec!(3.67),
            ("AED", "USD") => dec!(0.27),
            ("INR", "USD") => dec!(0.012),
            ("USD", "INR") => dec!(83.33),
            ("EUR", "USD") => dec!(1.08),
            ("USD", "EUR") => dec!(0.93),
            _ => dec!(1.0),
        }
    }

    /// Generate human-readable reasoning
    fn generate_reasoning(&self, selected: &SettlementPathOption, market: &InternalMarketConditions) -> String {
        let path_name = match &selected.path {
            SettlementPath::InstantBuy { .. } => "Instant Buy",
            SettlementPath::Hedging { hedge_type, .. } => match hedge_type {
                HedgeType::Full => "Full Hedging",
                HedgeType::Partial => "Partial Hedging",
                HedgeType::Dynamic => "Dynamic Hedging",
            },
            SettlementPath::Clearing { .. } => "Clearing/Netting",
        };

        let factors: String = selected.risk_factors.join("; ");

        format!(
            "{} selected (score: {:.1}/100, cost: {} bps). Market: {} volatility, {} liquidity. Factors: {}",
            path_name,
            selected.score,
            selected.cost_bps,
            format!("{:?}", market.volatility).to_lowercase(),
            format!("{:?}", market.liquidity).to_lowercase(),
            factors
        )
    }

    /// Calculate confidence in recommendation
    fn calculate_confidence(&self, selected: &SettlementPathOption, market: &InternalMarketConditions) -> f64 {
        let mut confidence = 0.5;

        // Higher score = higher confidence
        confidence += (selected.score - 50.0) / 100.0;

        // Clear market conditions = higher confidence
        match (&market.volatility, &market.liquidity) {
            (FxVolatility::Low, LiquidityDepth::Deep) => confidence += 0.2,
            (FxVolatility::Normal, LiquidityDepth::Normal) => confidence += 0.1,
            (FxVolatility::Extreme, _) | (_, LiquidityDepth::Stressed) => confidence -= 0.2,
            _ => {}
        }

        confidence.max(0.1).min(0.99)
    }

    /// Save path recommendation to database
    pub async fn save_recommendation(
        &self,
        recommendation: &SettlementPathRecommendation,
        pool: &PgPool,
    ) -> RiskResult<()> {
        let path_json = serde_json::to_value(&recommendation.recommended_path)
            .map_err(|e| RiskError::InternalError(e.to_string()))?;
        let market_json = serde_json::to_value(&recommendation.market_conditions)
            .map_err(|e| RiskError::InternalError(e.to_string()))?;

        sqlx::query(
            r#"INSERT INTO settlement_path_recommendations
               (transaction_id, recommended_path, confidence, cost_bps, execution_time_ms, market_conditions, reasoning, calculated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT (transaction_id) DO UPDATE
               SET recommended_path = $2, confidence = $3, cost_bps = $4,
                   execution_time_ms = $5, market_conditions = $6, reasoning = $7, calculated_at = $8"#,
        )
        .bind(recommendation.transaction_id)
        .bind(path_json)
        .bind(recommendation.confidence)
        .bind(recommendation.estimated_total_cost_bps)
        .bind(recommendation.estimated_execution_time_ms as i64)
        .bind(market_json)
        .bind(&recommendation.reasoning)
        .bind(recommendation.calculated_at)
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Internal market conditions (not exposed in API)
struct InternalMarketConditions {
    from_currency: String,
    to_currency: String,
    volatility: FxVolatility,
    liquidity: LiquidityDepth,
    clearing_status: ClearingWindowStatus,
    current_window_id: Option<Uuid>,
    time_to_window_close_ms: Option<u64>,
    estimated_rate: Decimal,
    estimated_spread_bps: i32,
    best_fx_provider: String,
}
