//! Transaction limit checking

use crate::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitConfig {
    /// Single transaction limit
    pub single_transaction: Decimal,

    /// Daily limit per entity
    pub daily_limit: Decimal,

    /// Monthly limit per entity
    pub monthly_limit: Decimal,
}

impl Default for LimitConfig {
    fn default() -> Self {
        Self {
            single_transaction: Decimal::from(1_000_000), // $1M
            daily_limit: Decimal::from(10_000_000),       // $10M
            monthly_limit: Decimal::from(100_000_000),    // $100M
        }
    }
}

/// Limit checker
pub struct LimitChecker {
    config: LimitConfig,
}

impl LimitChecker {
    /// Create new limit checker
    pub fn new(config: LimitConfig) -> Self {
        Self { config }
    }

    /// Check single transaction limit
    pub fn check_single_transaction(&self, amount: Decimal) -> Result<()> {
        if amount > self.config.single_transaction {
            return Err(Error::LimitExceeded(format!(
                "Transaction amount {} exceeds single transaction limit {}",
                amount, self.config.single_transaction
            )));
        }
        Ok(())
    }

    /// Check daily limit
    pub fn check_daily_limit(&self, amount: Decimal, daily_total: Decimal) -> Result<()> {
        if daily_total + amount > self.config.daily_limit {
            return Err(Error::LimitExceeded(format!(
                "Daily limit {} would be exceeded",
                self.config.daily_limit
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_transaction_limit() {
        let checker = LimitChecker::new(LimitConfig::default());

        assert!(checker.check_single_transaction(Decimal::from(500_000)).is_ok());
        assert!(checker.check_single_transaction(Decimal::from(2_000_000)).is_err());
    }
}
