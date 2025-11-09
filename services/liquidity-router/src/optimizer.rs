use crate::models::OptimizationPath;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

/// Currency conversion optimizer
pub struct ConversionOptimizer {
    fx_rates: HashMap<(String, String), Decimal>,
    spreads: HashMap<(String, String), i32>, // in basis points
}

impl ConversionOptimizer {
    pub fn new() -> Self {
        let mut fx_rates = HashMap::new();
        let mut spreads = HashMap::new();

        // Initialize common FX rates
        Self::init_fx_rates(&mut fx_rates, &mut spreads);

        ConversionOptimizer { fx_rates, spreads }
    }

    fn init_fx_rates(
        rates: &mut HashMap<(String, String), Decimal>,
        spreads: &mut HashMap<(String, String), i32>,
    ) {
        // Major pairs
        rates.insert(
            ("INR".to_string(), "AED".to_string()),
            Decimal::from_str("0.044").unwrap(),
        );
        rates.insert(
            ("AED".to_string(), "INR".to_string()),
            Decimal::from_str("22.73").unwrap(),
        );
        rates.insert(
            ("USD".to_string(), "AED".to_string()),
            Decimal::from_str("3.67").unwrap(),
        );
        rates.insert(
            ("AED".to_string(), "USD".to_string()),
            Decimal::from_str("0.27").unwrap(),
        );
        rates.insert(
            ("INR".to_string(), "USD".to_string()),
            Decimal::from_str("0.012").unwrap(),
        );
        rates.insert(
            ("USD".to_string(), "INR".to_string()),
            Decimal::from_str("83.33").unwrap(),
        );
        rates.insert(
            ("EUR".to_string(), "USD".to_string()),
            Decimal::from_str("1.08").unwrap(),
        );
        rates.insert(
            ("USD".to_string(), "EUR".to_string()),
            Decimal::from_str("0.93").unwrap(),
        );

        // Default spreads (10 bps)
        for key in rates.keys() {
            spreads.insert(key.clone(), 10);
        }
    }

    /// Find optimal conversion path
    pub fn find_optimal_path(
        &self,
        from_currency: &str,
        to_currency: &str,
    ) -> Option<OptimizationPath> {
        // Direct path
        if let Some(rate) = self
            .fx_rates
            .get(&(from_currency.to_string(), to_currency.to_string()))
        {
            let spread = self
                .spreads
                .get(&(from_currency.to_string(), to_currency.to_string()))
                .copied()
                .unwrap_or(10);

            return Some(OptimizationPath {
                from_currency: from_currency.to_string(),
                to_currency: to_currency.to_string(),
                route: vec![from_currency.to_string(), to_currency.to_string()],
                total_cost_bps: spread,
                estimated_time_seconds: 5,
                fx_rates: vec![*rate],
            });
        }

        // Try path through USD
        self.find_path_through_bridge(from_currency, to_currency, "USD")
    }

    /// Find path through a bridge currency
    fn find_path_through_bridge(
        &self,
        from: &str,
        to: &str,
        bridge: &str,
    ) -> Option<OptimizationPath> {
        let rate1 = self
            .fx_rates
            .get(&(from.to_string(), bridge.to_string()))?;
        let rate2 = self
            .fx_rates
            .get(&(bridge.to_string(), to.to_string()))?;

        let spread1 = self
            .spreads
            .get(&(from.to_string(), bridge.to_string()))
            .copied()
            .unwrap_or(10);
        let spread2 = self
            .spreads
            .get(&(bridge.to_string(), to.to_string()))
            .copied()
            .unwrap_or(10);

        Some(OptimizationPath {
            from_currency: from.to_string(),
            to_currency: to.to_string(),
            route: vec![from.to_string(), bridge.to_string(), to.to_string()],
            total_cost_bps: spread1 + spread2,
            estimated_time_seconds: 10,
            fx_rates: vec![*rate1, *rate2],
        })
    }

    /// Update FX rate
    pub fn update_rate(&mut self, from: &str, to: &str, rate: Decimal, spread_bps: i32) {
        self.fx_rates
            .insert((from.to_string(), to.to_string()), rate);
        self.spreads
            .insert((from.to_string(), to.to_string()), spread_bps);
    }
}