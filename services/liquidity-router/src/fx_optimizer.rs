// FX Optimizer Module for Liquidity Router
// Finds best FX rates across multiple partners and jurisdictions
// Supports multi-hop routes (e.g., INR → USD → AED)

use crate::models::*;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

/// FX Optimizer - finds best rates across partners
pub struct FxOptimizer {
    partners: Vec<FxPartner>,
    quotes_cache: HashMap<String, Vec<FxQuote>>, // currency_pair -> quotes
}

impl FxOptimizer {
    pub fn new() -> Self {
        let partners = Self::init_partners();
        let quotes_cache = Self::init_quotes(&partners);

        FxOptimizer {
            partners,
            quotes_cache,
        }
    }

    /// Initialize FX partners (in production, load from database)
    fn init_partners() -> Vec<FxPartner> {
        vec![
            FxPartner {
                partner_id: Uuid::new_v4(),
                partner_code: "ENBD-FX".to_string(),
                partner_name: "Emirates NBD FX".to_string(),
                jurisdiction: "AE".to_string(),
                supported_pairs: vec![
                    "USD/AED".to_string(), "EUR/AED".to_string(),
                    "GBP/AED".to_string(), "INR/AED".to_string(),
                ],
                is_active: true,
                priority: 1,
            },
            FxPartner {
                partner_id: Uuid::new_v4(),
                partner_code: "HDFC-FX".to_string(),
                partner_name: "HDFC Bank FX".to_string(),
                jurisdiction: "IN".to_string(),
                supported_pairs: vec![
                    "USD/INR".to_string(), "EUR/INR".to_string(),
                    "GBP/INR".to_string(), "AED/INR".to_string(),
                ],
                is_active: true,
                priority: 1,
            },
            FxPartner {
                partner_id: Uuid::new_v4(),
                partner_code: "CITI-FX".to_string(),
                partner_name: "Citibank FX".to_string(),
                jurisdiction: "US".to_string(),
                supported_pairs: vec![
                    "EUR/USD".to_string(), "GBP/USD".to_string(),
                    "USD/JPY".to_string(), "USD/CHF".to_string(),
                    "USD/INR".to_string(), "USD/AED".to_string(),
                ],
                is_active: true,
                priority: 2,
            },
            FxPartner {
                partner_id: Uuid::new_v4(),
                partner_code: "DB-FX".to_string(),
                partner_name: "Deutsche Bank FX".to_string(),
                jurisdiction: "DE".to_string(),
                supported_pairs: vec![
                    "EUR/USD".to_string(), "EUR/GBP".to_string(),
                    "EUR/CHF".to_string(), "EUR/INR".to_string(),
                ],
                is_active: true,
                priority: 2,
            },
            FxPartner {
                partner_id: Uuid::new_v4(),
                partner_code: "HAPOALIM-FX".to_string(),
                partner_name: "Bank Hapoalim FX".to_string(),
                jurisdiction: "IL".to_string(),
                supported_pairs: vec![
                    "USD/ILS".to_string(), "EUR/ILS".to_string(),
                ],
                is_active: true,
                priority: 1,
            },
        ]
    }

    /// Initialize quotes cache (in production, fetch from partners via API)
    fn init_quotes(partners: &[FxPartner]) -> HashMap<String, Vec<FxQuote>> {
        let mut cache = HashMap::new();
        let valid_until = Utc::now() + chrono::Duration::minutes(5);

        // Sample quotes for each partner
        for partner in partners {
            for pair in &partner.supported_pairs {
                let (bid, ask, spread) = Self::get_sample_rate(pair);

                let quote = FxQuote {
                    quote_id: Uuid::new_v4(),
                    partner_id: partner.partner_id,
                    partner_code: partner.partner_code.clone(),
                    currency_pair: pair.clone(),
                    bid_rate: bid,
                    ask_rate: ask,
                    spread_bps: spread,
                    min_amount: dec!(1000),
                    max_amount: dec!(10000000),
                    available_liquidity: dec!(5000000),
                    valid_until,
                    execution_time_ms: 300 + (partner.priority as u64 * 100),
                };

                cache.entry(pair.clone())
                    .or_insert_with(Vec::new)
                    .push(quote);
            }
        }

        cache
    }

    /// Get sample FX rate (in production, fetch real-time)
    fn get_sample_rate(pair: &str) -> (Decimal, Decimal, i32) {
        match pair {
            "USD/AED" => (dec!(3.6720), dec!(3.6730), 3),
            "EUR/AED" => (dec!(3.9650), dec!(3.9680), 8),
            "GBP/AED" => (dec!(4.6200), dec!(4.6250), 11),
            "INR/AED" => (dec!(0.0440), dec!(0.0442), 15),
            "USD/INR" => (dec!(83.30), dec!(83.35), 6),
            "EUR/INR" => (dec!(89.90), dec!(89.98), 9),
            "GBP/INR" => (dec!(105.00), dec!(105.12), 11),
            "AED/INR" => (dec!(22.68), dec!(22.72), 18),
            "EUR/USD" => (dec!(1.0795), dec!(1.0800), 5),
            "GBP/USD" => (dec!(1.2600), dec!(1.2608), 6),
            "USD/JPY" => (dec!(154.50), dec!(154.55), 3),
            "USD/CHF" => (dec!(0.8950), dec!(0.8955), 6),
            "EUR/GBP" => (dec!(0.8565), dec!(0.8572), 8),
            "EUR/CHF" => (dec!(0.9660), dec!(0.9668), 8),
            "USD/ILS" => (dec!(3.7500), dec!(3.7520), 5),
            "EUR/ILS" => (dec!(4.0470), dec!(4.0510), 10),
            _ => (dec!(1.0), dec!(1.001), 10),
        }
    }

    /// Find best FX route for given request
    pub async fn find_best_route(&self, request: &FxOptimizationRequest) -> FxOptimizationResult {
        info!(
            "🔍 Finding best FX route: {} → {} ({} {})",
            request.from_currency, request.to_currency,
            request.amount, request.from_currency
        );

        let mut routes = Vec::new();

        // 1. Try direct route
        if let Some(direct) = self.find_direct_route(request) {
            info!("  📍 Direct route found: {} bps", direct.total_cost_bps);
            routes.push(direct);
        }

        // 2. Try routes through USD (common bridge currency)
        if request.from_currency != "USD" && request.to_currency != "USD" {
            if let Some(via_usd) = self.find_route_via_bridge(request, "USD") {
                info!("  📍 Route via USD found: {} bps", via_usd.total_cost_bps);
                routes.push(via_usd);
            }
        }

        // 3. Try routes through EUR (for European corridors)
        if request.from_currency != "EUR" && request.to_currency != "EUR" {
            if let Some(via_eur) = self.find_route_via_bridge(request, "EUR") {
                info!("  📍 Route via EUR found: {} bps", via_eur.total_cost_bps);
                routes.push(via_eur);
            }
        }

        // Sort by total cost (lowest first)
        routes.sort_by_key(|r| r.total_cost_bps);

        let best_route = routes.first().cloned().unwrap_or_else(|| self.fallback_route(request));
        let alternative_routes = routes.into_iter().skip(1).collect();

        // Calculate savings vs direct market rate
        let direct_cost = self.estimate_direct_market_cost(&request.from_currency, &request.to_currency);
        let savings = direct_cost - best_route.total_cost_bps;

        info!(
            "✅ Best route selected: {} hops, {} bps (savings: {} bps)",
            best_route.hops.len(),
            best_route.total_cost_bps,
            savings
        );

        FxOptimizationResult {
            request_id: request.request_id,
            payment_id: request.payment_id,
            best_route,
            alternative_routes,
            savings_vs_direct_bps: savings,
            optimization_notes: "Optimized across multiple partners".to_string(),
            executed: false,
            execution_id: None,
            calculated_at: Utc::now(),
        }
    }

    /// Find direct route (single hop)
    fn find_direct_route(&self, request: &FxOptimizationRequest) -> Option<FxRoute> {
        let pair = format!("{}/{}", request.from_currency, request.to_currency);
        let reverse_pair = format!("{}/{}", request.to_currency, request.from_currency);

        // Try direct pair
        if let Some(quotes) = self.quotes_cache.get(&pair) {
            let best_quote = quotes.iter()
                .filter(|q| q.available_liquidity >= request.amount)
                .min_by_key(|q| q.spread_bps)?;

            let amount_out = request.amount * best_quote.ask_rate;

            return Some(FxRoute {
                route_id: Uuid::new_v4(),
                from_currency: request.from_currency.clone(),
                to_currency: request.to_currency.clone(),
                hops: vec![FxHop {
                    hop_number: 1,
                    from_currency: request.from_currency.clone(),
                    to_currency: request.to_currency.clone(),
                    quote: best_quote.clone(),
                    amount_in: request.amount,
                    amount_out,
                }],
                total_rate: best_quote.ask_rate,
                total_cost_bps: best_quote.spread_bps,
                total_execution_time_ms: best_quote.execution_time_ms,
                confidence: 0.95,
            });
        }

        // Try reverse pair (we're selling)
        if let Some(quotes) = self.quotes_cache.get(&reverse_pair) {
            let best_quote = quotes.iter()
                .filter(|q| q.available_liquidity >= request.amount)
                .min_by_key(|q| q.spread_bps)?;

            let effective_rate = dec!(1) / best_quote.bid_rate;
            let amount_out = request.amount * effective_rate;

            return Some(FxRoute {
                route_id: Uuid::new_v4(),
                from_currency: request.from_currency.clone(),
                to_currency: request.to_currency.clone(),
                hops: vec![FxHop {
                    hop_number: 1,
                    from_currency: request.from_currency.clone(),
                    to_currency: request.to_currency.clone(),
                    quote: best_quote.clone(),
                    amount_in: request.amount,
                    amount_out,
                }],
                total_rate: effective_rate,
                total_cost_bps: best_quote.spread_bps,
                total_execution_time_ms: best_quote.execution_time_ms,
                confidence: 0.93,
            });
        }

        None
    }

    /// Find route via bridge currency (2 hops)
    fn find_route_via_bridge(&self, request: &FxOptimizationRequest, bridge: &str) -> Option<FxRoute> {
        // Hop 1: from_currency → bridge
        let hop1_pair = format!("{}/{}", request.from_currency, bridge);
        let hop1_reverse = format!("{}/{}", bridge, request.from_currency);

        let hop1_quote = self.quotes_cache.get(&hop1_pair)
            .and_then(|q| q.iter().min_by_key(|q| q.spread_bps).cloned())
            .or_else(|| self.quotes_cache.get(&hop1_reverse)
                .and_then(|q| q.iter().min_by_key(|q| q.spread_bps).cloned()))?;

        // Hop 2: bridge → to_currency
        let hop2_pair = format!("{}/{}", bridge, request.to_currency);
        let hop2_reverse = format!("{}/{}", request.to_currency, bridge);

        let hop2_quote = self.quotes_cache.get(&hop2_pair)
            .and_then(|q| q.iter().min_by_key(|q| q.spread_bps).cloned())
            .or_else(|| self.quotes_cache.get(&hop2_reverse)
                .and_then(|q| q.iter().min_by_key(|q| q.spread_bps).cloned()))?;

        // Calculate amounts
        let hop1_rate = if hop1_quote.currency_pair == hop1_pair {
            hop1_quote.ask_rate
        } else {
            dec!(1) / hop1_quote.bid_rate
        };

        let hop2_rate = if hop2_quote.currency_pair == hop2_pair {
            hop2_quote.ask_rate
        } else {
            dec!(1) / hop2_quote.bid_rate
        };

        let bridge_amount = request.amount * hop1_rate;
        let final_amount = bridge_amount * hop2_rate;
        let total_rate = hop1_rate * hop2_rate;
        let total_cost = hop1_quote.spread_bps + hop2_quote.spread_bps;
        let total_time = hop1_quote.execution_time_ms + hop2_quote.execution_time_ms;

        Some(FxRoute {
            route_id: Uuid::new_v4(),
            from_currency: request.from_currency.clone(),
            to_currency: request.to_currency.clone(),
            hops: vec![
                FxHop {
                    hop_number: 1,
                    from_currency: request.from_currency.clone(),
                    to_currency: bridge.to_string(),
                    quote: hop1_quote,
                    amount_in: request.amount,
                    amount_out: bridge_amount,
                },
                FxHop {
                    hop_number: 2,
                    from_currency: bridge.to_string(),
                    to_currency: request.to_currency.clone(),
                    quote: hop2_quote,
                    amount_in: bridge_amount,
                    amount_out: final_amount,
                },
            ],
            total_rate,
            total_cost_bps: total_cost,
            total_execution_time_ms: total_time,
            confidence: 0.88,
        })
    }

    /// Fallback route when no quotes available
    fn fallback_route(&self, request: &FxOptimizationRequest) -> FxRoute {
        warn!(
            "⚠️ No FX route found for {} → {}, using fallback",
            request.from_currency, request.to_currency
        );

        FxRoute {
            route_id: Uuid::new_v4(),
            from_currency: request.from_currency.clone(),
            to_currency: request.to_currency.clone(),
            hops: vec![],
            total_rate: dec!(1),
            total_cost_bps: 50, // High cost for fallback
            total_execution_time_ms: 5000,
            confidence: 0.5,
        }
    }

    /// Estimate direct market cost (without optimization)
    fn estimate_direct_market_cost(&self, from: &str, to: &str) -> i32 {
        // Default retail spreads without optimization
        match (from, to) {
            ("USD", "AED") | ("AED", "USD") => 10,
            ("USD", "INR") | ("INR", "USD") => 25,
            ("EUR", "USD") | ("USD", "EUR") => 15,
            ("INR", "AED") | ("AED", "INR") => 35,
            _ => 40,
        }
    }

    /// Execute the FX trade (in production, call partner APIs)
    pub async fn execute_route(&self, route: &FxRoute, payment_id: Uuid) -> Option<Uuid> {
        info!(
            "💱 Executing FX route for payment {}: {} → {} ({} hops)",
            payment_id,
            route.from_currency,
            route.to_currency,
            route.hops.len()
        );

        // In production, execute each hop with the partner
        for hop in &route.hops {
            info!(
                "  📍 Hop {}: {} → {} via {} @ {}",
                hop.hop_number,
                hop.from_currency,
                hop.to_currency,
                hop.quote.partner_code,
                hop.quote.ask_rate
            );
        }

        Some(Uuid::new_v4()) // Return execution ID
    }
}

impl Default for FxOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
