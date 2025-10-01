// fx/quote_aggregator.rs
// Quote Aggregation Engine with Weighted Scoring and Multi-MM Support

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::config::{FXConfig, QuoteScoringWeights};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub request_id: Uuid,
    pub payment_id: Option<Uuid>,
    pub from_currency: String,
    pub to_currency: String,
    pub amount: Decimal,
    pub quote_type: QuoteType,
    pub requester_id: String,
    pub max_slippage_bps: Option<u32>,
    pub preferred_market_makers: Vec<Uuid>,
    pub allow_split_execution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuoteType {
    Spot,
    Tom,
    Forward,
    Swap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub request_id: Uuid,
    pub best_quote: Option<Quote>,
    pub all_quotes: Vec<Quote>,
    pub market_depth: MarketDepth,
    pub expires_at: chrono::NaiveDateTime,
    pub ttl_seconds: u32,
    pub execution_plan: Option<ExecutionPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub quote_id: Uuid,
    pub market_maker_id: Uuid,
    pub market_maker_name: String,
    pub from_currency: String,
    pub to_currency: String,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub rate: Decimal,
    pub inverse_rate: Decimal,
    pub mid_rate: Option<Decimal>,
    pub spread_bps: u32,
    pub markup_bps: u32,
    pub quoted_at: chrono::NaiveDateTime,
    pub expires_at: chrono::NaiveDateTime,
    pub ttl_seconds: u32,
    pub available_liquidity: Decimal,
    pub liquidity_tier: LiquidityTier,
    pub risk_score: f32,
    pub risk_reason: Option<String>,
    pub score: f32, // Weighted scoring result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiquidityTier {
    Tier1, // > $10M
    Tier2, // $1M - $10M
    Tier3, // $100K - $1M
    Tier4, // < $100K
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDepth {
    pub currency_pair: String,
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub mid_rate: Decimal,
    pub spread_bps: u32,
    pub timestamp: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthLevel {
    pub rate: Decimal,
    pub amount: Decimal,
    pub market_maker_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub plan_id: Uuid,
    pub legs: Vec<ExecutionLeg>,
    pub total_from_amount: Decimal,
    pub total_to_amount: Decimal,
    pub weighted_avg_rate: Decimal,
    pub total_spread_bps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLeg {
    pub leg_id: Uuid,
    pub quote_id: Uuid,
    pub market_maker_id: Uuid,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub rate: Decimal,
    pub sequence: u32,
    pub allocation_percentage: f32,
}

// Market Maker Adapter trait
#[async_trait::async_trait]
pub trait MarketMakerAdapter: Send + Sync {
    async fn request_quote(&self, request: &QuoteRequest) -> Result<Quote, MMAdapterError>;
    async fn health_check(&self) -> Result<bool, MMAdapterError>;
    fn get_mm_id(&self) -> Uuid;
    fn get_mm_name(&self) -> String;
}

#[derive(Debug)]
pub enum MMAdapterError {
    Timeout,
    Unavailable,
    InvalidResponse,
    NetworkError(String),
}

// Quote Aggregator Engine
pub struct QuoteAggregator {
    config: Arc<FXConfig>,
    market_makers: Arc<RwLock<HashMap<Uuid, Arc<dyn MarketMakerAdapter>>>>,
    mm_performance: Arc<RwLock<HashMap<Uuid, MMPerformance>>>,
}

#[derive(Debug, Clone)]
struct MMPerformance {
    fill_rate: f32,
    avg_response_time_ms: f32,
    uptime_percent: f32,
    reject_rate: f32,
}

impl QuoteAggregator {
    pub fn new(config: Arc<FXConfig>) -> Self {
        Self {
            config,
            market_makers: Arc::new(RwLock::new(HashMap::new())),
            mm_performance: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a market maker
    pub async fn register_market_maker(&self, adapter: Arc<dyn MarketMakerAdapter>) {
        let mm_id = adapter.get_mm_id();
        self.market_makers.write().await.insert(mm_id, adapter);

        // Initialize performance tracking
        self.mm_performance.write().await.insert(
            mm_id,
            MMPerformance {
                fill_rate: 1.0,
                avg_response_time_ms: 100.0,
                uptime_percent: 100.0,
                reject_rate: 0.0,
            },
        );

        info!("Registered market maker: {}", mm_id);
    }

    /// Request quotes from all market makers
    pub async fn request_quotes(&self, request: QuoteRequest) -> Result<QuoteResponse, AggregatorError> {
        let start_time = std::time::Instant::now();

        info!(
            "Requesting quotes for {} {} -> {} from {} market makers",
            request.amount,
            request.from_currency,
            request.to_currency,
            self.market_makers.read().await.len()
        );

        // Determine which MMs to query
        let target_mms = self.select_market_makers(&request).await;

        if target_mms.is_empty() {
            return Err(AggregatorError::NoMarketMakersAvailable);
        }

        // Request quotes in parallel with timeout
        let is_exotic = self.is_exotic_pair(&request.from_currency, &request.to_currency);
        let timeout_ms = if is_exotic {
            self.config.market_makers.rfq_timeout_exotic_ms
        } else {
            self.config.market_makers.rfq_timeout_ms
        };

        let quote_futures: Vec<_> = target_mms
            .iter()
            .map(|mm| self.request_quote_from_mm(mm.clone(), &request, timeout_ms))
            .collect();

        let quote_results = futures::future::join_all(quote_futures).await;

        // Collect successful quotes
        let mut all_quotes: Vec<Quote> = quote_results
            .into_iter()
            .filter_map(|result| result.ok())
            .collect();

        if all_quotes.is_empty() {
            return Err(AggregatorError::AllMarketMakersRejected);
        }

        info!("Received {} quotes in {}ms", all_quotes.len(), start_time.elapsed().as_millis());

        // Validate quotes against mid-market reference
        if self.config.market_makers.validate_against_mid_ref {
            self.validate_quotes_against_mid(&mut all_quotes, &request).await?;
        }

        // Filter stale quotes
        self.filter_stale_quotes(&mut all_quotes);

        // Score quotes
        self.score_quotes(&mut all_quotes).await;

        // Sort by score (highest first)
        all_quotes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Select best quote
        let best_quote = all_quotes.first().cloned();

        // Build execution plan if split execution needed
        let execution_plan = if request.allow_split_execution
            && self.config.market_makers.allow_split_execution
            && self.should_split_execution(&all_quotes, &request)
        {
            Some(self.build_execution_plan(&all_quotes, &request))
        } else {
            None
        };

        // Build market depth
        let market_depth = self.build_market_depth(&all_quotes, &request);

        // Calculate expiry
        let min_ttl = all_quotes
            .iter()
            .map(|q| q.ttl_seconds)
            .min()
            .unwrap_or(10);

        let expires_at = chrono::Utc::now().naive_utc()
            + chrono::Duration::seconds(min_ttl as i64);

        Ok(QuoteResponse {
            request_id: request.request_id,
            best_quote,
            all_quotes,
            market_depth,
            expires_at,
            ttl_seconds: min_ttl,
            execution_plan,
        })
    }

    /// Select which market makers to query
    async fn select_market_makers(&self, request: &QuoteRequest) -> Vec<Arc<dyn MarketMakerAdapter>> {
        let mms = self.market_makers.read().await;

        // Start with preferred MMs if specified
        let mut selected: Vec<Arc<dyn MarketMakerAdapter>> = if !request.preferred_market_makers.is_empty() {
            request
                .preferred_market_makers
                .iter()
                .filter_map(|mm_id| mms.get(mm_id).cloned())
                .collect()
        } else {
            mms.values().cloned().collect()
        };

        // Filter out suspended MMs
        let performance = self.mm_performance.read().await;
        if self.config.market_makers.auto_suspension.enabled {
            selected.retain(|mm| {
                let mm_id = mm.get_mm_id();
                if let Some(perf) = performance.get(&mm_id) {
                    perf.uptime_percent >= self.config.market_makers.auto_suspension.min_uptime_percent
                        && perf.reject_rate <= self.config.market_makers.auto_suspension.max_reject_rate_percent
                } else {
                    true
                }
            });
        }

        selected
    }

    /// Request quote from single MM with timeout
    async fn request_quote_from_mm(
        &self,
        mm: Arc<dyn MarketMakerAdapter>,
        request: &QuoteRequest,
        timeout_ms: u64,
    ) -> Result<Quote, MMAdapterError> {
        let mm_name = mm.get_mm_name();

        match timeout(Duration::from_millis(timeout_ms), mm.request_quote(request)).await {
            Ok(Ok(quote)) => {
                info!("Received quote from {} (rate: {})", mm_name, quote.rate);
                Ok(quote)
            }
            Ok(Err(e)) => {
                warn!("MM {} returned error: {:?}", mm_name, e);
                Err(e)
            }
            Err(_) => {
                warn!("MM {} timed out after {}ms", mm_name, timeout_ms);
                Err(MMAdapterError::Timeout)
            }
        }
    }

    /// Validate quotes against mid-market reference
    async fn validate_quotes_against_mid(
        &self,
        quotes: &mut Vec<Quote>,
        request: &QuoteRequest,
    ) -> Result<(), AggregatorError> {
        // In production, fetch mid rate from Bloomberg/Refinitiv/ECB
        // For MVP, calculate from quotes
        let mid_rate = self.calculate_mid_rate_from_quotes(quotes);

        if mid_rate.is_zero() {
            return Ok(()); // Skip validation if no mid available
        }

        let max_deviation_bps = self.config.market_makers.max_deviation_from_mid_bps as f32;

        quotes.retain(|quote| {
            let deviation_bps = ((quote.rate - mid_rate) / mid_rate * Decimal::new(10000, 0))
                .abs()
                .to_string()
                .parse::<f32>()
                .unwrap_or(0.0);

            if deviation_bps > max_deviation_bps {
                warn!(
                    "Filtering quote from {} - deviation {}bps exceeds limit {}bps",
                    quote.market_maker_name, deviation_bps, max_deviation_bps
                );
                false
            } else {
                true
            }
        });

        Ok(())
    }

    /// Calculate mid rate from quotes (median)
    fn calculate_mid_rate_from_quotes(&self, quotes: &[Quote]) -> Decimal {
        if quotes.is_empty() {
            return Decimal::ZERO;
        }

        let mut rates: Vec<Decimal> = quotes.iter().map(|q| q.rate).collect();
        rates.sort();

        if rates.len() % 2 == 0 {
            (rates[rates.len() / 2 - 1] + rates[rates.len() / 2]) / Decimal::TWO
        } else {
            rates[rates.len() / 2]
        }
    }

    /// Filter stale quotes
    fn filter_stale_quotes(&self, quotes: &mut Vec<Quote>) {
        let now = chrono::Utc::now().naive_utc();

        quotes.retain(|quote| {
            let age_ms = (now - quote.quoted_at).num_milliseconds() as u64;
            let threshold = self.config.market_makers.stale_quote_threshold_ms;

            if age_ms > threshold {
                warn!("Filtering stale quote from {} (age: {}ms)", quote.market_maker_name, age_ms);
                false
            } else {
                true
            }
        });
    }

    /// Score quotes using weighted scoring
    async fn score_quotes(&self, quotes: &mut Vec<Quote>) {
        let weights = &self.config.market_makers.quote_scoring;
        let performance = self.mm_performance.read().await;

        for quote in quotes.iter_mut() {
            // Normalize components to 0-1 scale

            // Rate component (lower rate = better for buying from_currency)
            let rate_score = 1.0 - (quote.rate.to_string().parse::<f32>().unwrap_or(1.0) / 2.0).min(1.0);

            // Liquidity component (higher liquidity = better)
            let liquidity_score = match quote.liquidity_tier {
                LiquidityTier::Tier1 => 1.0,
                LiquidityTier::Tier2 => 0.75,
                LiquidityTier::Tier3 => 0.5,
                LiquidityTier::Tier4 => 0.25,
            };

            // Latency component (from MM performance)
            let latency_score = if let Some(perf) = performance.get(&quote.market_maker_id) {
                (1000.0 - perf.avg_response_time_ms.min(1000.0)) / 1000.0
            } else {
                0.5
            };

            // Fill rate component (from MM performance)
            let fill_rate_score = if let Some(perf) = performance.get(&quote.market_maker_id) {
                perf.fill_rate
            } else {
                0.5
            };

            // Weighted score
            quote.score = weights.w_rate * rate_score
                + weights.w_liquidity * liquidity_score
                + weights.w_latency * latency_score
                + weights.w_fill_rate * fill_rate_score;
        }
    }

    /// Determine if split execution is needed
    fn should_split_execution(&self, quotes: &[Quote], request: &QuoteRequest) -> bool {
        if quotes.len() < 2 {
            return false;
        }

        // Check if any single quote can fill the entire amount
        let can_single_fill = quotes.iter().any(|q| q.available_liquidity >= request.amount);

        if can_single_fill {
            return false;
        }

        // Check if split across top N quotes can fill
        let top_n = self.config.market_makers.max_split_mms as usize;
        let total_liquidity: Decimal = quotes.iter().take(top_n).map(|q| q.available_liquidity).sum();

        total_liquidity >= request.amount
    }

    /// Build execution plan for split orders
    fn build_execution_plan(&self, quotes: &[Quote], request: &QuoteRequest) -> ExecutionPlan {
        let plan_id = Uuid::new_v4();
        let mut legs = Vec::new();
        let mut remaining_amount = request.amount;
        let max_split = self.config.market_makers.max_split_mms as usize;

        for (idx, quote) in quotes.iter().take(max_split).enumerate() {
            if remaining_amount.is_zero() {
                break;
            }

            let allocated_amount = remaining_amount.min(quote.available_liquidity);
            let allocated_to_amount = allocated_amount * quote.rate;
            let allocation_pct = (allocated_amount / request.amount * Decimal::new(100, 0))
                .to_string()
                .parse::<f32>()
                .unwrap_or(0.0);

            legs.push(ExecutionLeg {
                leg_id: Uuid::new_v4(),
                quote_id: quote.quote_id,
                market_maker_id: quote.market_maker_id,
                from_amount: allocated_amount,
                to_amount: allocated_to_amount,
                rate: quote.rate,
                sequence: idx as u32 + 1,
                allocation_percentage: allocation_pct,
            });

            remaining_amount -= allocated_amount;
        }

        let total_from_amount: Decimal = legs.iter().map(|l| l.from_amount).sum();
        let total_to_amount: Decimal = legs.iter().map(|l| l.to_amount).sum();
        let weighted_avg_rate = if total_from_amount.is_zero() {
            Decimal::ZERO
        } else {
            total_to_amount / total_from_amount
        };

        ExecutionPlan {
            plan_id,
            legs,
            total_from_amount,
            total_to_amount,
            weighted_avg_rate,
            total_spread_bps: 0, // TODO: Calculate weighted spread
        }
    }

    /// Build market depth from quotes
    fn build_market_depth(&self, quotes: &[Quote], request: &QuoteRequest) -> MarketDepth {
        let currency_pair = format!("{}/{}", request.from_currency, request.to_currency);

        // Group by rate level
        let mut rate_map: HashMap<Decimal, (Decimal, u32)> = HashMap::new();

        for quote in quotes {
            let entry = rate_map.entry(quote.rate).or_insert((Decimal::ZERO, 0));
            entry.0 += quote.available_liquidity;
            entry.1 += 1;
        }

        let mut asks: Vec<DepthLevel> = rate_map
            .into_iter()
            .map(|(rate, (amount, count))| DepthLevel {
                rate,
                amount,
                market_maker_count: count,
            })
            .collect();

        asks.sort_by(|a, b| a.rate.cmp(&b.rate));

        let mid_rate = self.calculate_mid_rate_from_quotes(quotes);
        let spread_bps = if !asks.is_empty() && !mid_rate.is_zero() {
            ((asks[0].rate - mid_rate) / mid_rate * Decimal::new(10000, 0))
                .abs()
                .to_string()
                .parse::<u32>()
                .unwrap_or(0)
        } else {
            0
        };

        MarketDepth {
            currency_pair,
            bids: vec![], // TODO: Implement bid side for two-way quotes
            asks: asks.into_iter().take(self.config.rate_discovery.market_depth_levels as usize).collect(),
            mid_rate,
            spread_bps,
            timestamp: chrono::Utc::now().naive_utc(),
        }
    }

    /// Check if currency pair is exotic
    fn is_exotic_pair(&self, from: &str, to: &str) -> bool {
        let g10_currencies = ["USD", "EUR", "GBP", "JPY", "CHF", "AUD", "NZD", "CAD", "SEK", "NOK"];

        !(g10_currencies.contains(&from) && g10_currencies.contains(&to))
    }
}

#[derive(Debug)]
pub enum AggregatorError {
    NoMarketMakersAvailable,
    AllMarketMakersRejected,
    ValidationFailed(String),
    ConfigError(String),
}

impl std::fmt::Display for AggregatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregatorError::NoMarketMakersAvailable => write!(f, "No market makers available"),
            AggregatorError::AllMarketMakersRejected => write!(f, "All market makers rejected quote request"),
            AggregatorError::ValidationFailed(e) => write!(f, "Validation failed: {}", e),
            AggregatorError::ConfigError(e) => write!(f, "Config error: {}", e),
        }
    }
}

impl std::error::Error for AggregatorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_scoring() {
        // Test quote scoring calculation
    }

    #[test]
    fn test_split_execution_plan() {
        // Test execution plan generation
    }
}
