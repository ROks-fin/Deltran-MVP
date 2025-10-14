//! Corridor-based limits for cross-border payments

use crate::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Corridor identifier (e.g., "USD-EUR", "GBP-INR")
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Corridor {
    pub source_currency: String,
    pub target_currency: String,
}

impl Corridor {
    pub fn new(source: &str, target: &str) -> Self {
        Self {
            source_currency: source.to_uppercase(),
            target_currency: target.to_uppercase(),
        }
    }

    pub fn as_string(&self) -> String {
        format!("{}-{}", self.source_currency, self.target_currency)
    }
}

/// Corridor-specific limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorLimits {
    /// Soft limit - generates warning but allows transaction
    pub soft_limit: Decimal,

    /// Hard limit - blocks transaction
    pub hard_limit: Decimal,

    /// Enabled flag
    pub enabled: bool,
}

impl Default for CorridorLimits {
    fn default() -> Self {
        Self {
            soft_limit: Decimal::from(250_000),  // $250k
            hard_limit: Decimal::from(1_000_000), // $1M
            enabled: true,
        }
    }
}

/// Corridor limit checker
pub struct CorridorLimitChecker {
    // Map: corridor -> limits
    corridor_limits: HashMap<String, CorridorLimits>,
    default_limits: CorridorLimits,
}

impl CorridorLimitChecker {
    /// Create new corridor limit checker with default limits
    pub fn new(default_limits: CorridorLimits) -> Self {
        Self {
            corridor_limits: HashMap::new(),
            default_limits,
        }
    }

    /// Set limits for specific corridor
    pub fn set_corridor_limits(&mut self, corridor: Corridor, limits: CorridorLimits) {
        self.corridor_limits.insert(corridor.as_string(), limits);
    }

    /// Check if amount exceeds corridor limits
    pub fn check_corridor_limit(
        &self,
        source_currency: &str,
        target_currency: &str,
        amount: Decimal,
    ) -> Result<CorridorCheckResult> {
        let corridor = Corridor::new(source_currency, target_currency);
        let limits = self.corridor_limits
            .get(&corridor.as_string())
            .unwrap_or(&self.default_limits);

        if !limits.enabled {
            return Ok(CorridorCheckResult {
                corridor: corridor.as_string(),
                status: CorridorStatus::Disabled,
                amount,
                soft_limit: limits.soft_limit,
                hard_limit: limits.hard_limit,
            });
        }

        // Check hard limit first
        if amount > limits.hard_limit {
            return Err(Error::CorridorLimitExceeded(format!(
                "Amount {} exceeds hard limit {} for corridor {}",
                amount, limits.hard_limit, corridor.as_string()
            )));
        }

        // Check soft limit
        let status = if amount > limits.soft_limit {
            CorridorStatus::SoftLimitExceeded
        } else {
            CorridorStatus::Ok
        };

        Ok(CorridorCheckResult {
            corridor: corridor.as_string(),
            status,
            amount,
            soft_limit: limits.soft_limit,
            hard_limit: limits.hard_limit,
        })
    }

    /// Get limits for a specific corridor
    pub fn get_corridor_limits(&self, source_currency: &str, target_currency: &str) -> CorridorLimits {
        let corridor = Corridor::new(source_currency, target_currency);
        self.corridor_limits
            .get(&corridor.as_string())
            .cloned()
            .unwrap_or_else(|| self.default_limits.clone())
    }

    /// Load corridor limits from configuration
    pub fn load_corridors(&mut self, corridors: Vec<(Corridor, CorridorLimits)>) {
        for (corridor, limits) in corridors {
            self.corridor_limits.insert(corridor.as_string(), limits);
        }
    }

    /// Get all configured corridors
    pub fn get_all_corridors(&self) -> Vec<String> {
        self.corridor_limits.keys().cloned().collect()
    }
}

/// Result of corridor limit check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorCheckResult {
    pub corridor: String,
    pub status: CorridorStatus,
    pub amount: Decimal,
    pub soft_limit: Decimal,
    pub hard_limit: Decimal,
}

impl CorridorCheckResult {
    /// Check if transaction can proceed
    pub fn is_allowed(&self) -> bool {
        matches!(self.status, CorridorStatus::Ok | CorridorStatus::SoftLimitExceeded | CorridorStatus::Disabled)
    }

    /// Check if warning should be generated
    pub fn should_warn(&self) -> bool {
        matches!(self.status, CorridorStatus::SoftLimitExceeded)
    }
}

/// Corridor check status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CorridorStatus {
    /// Transaction within limits
    Ok,

    /// Soft limit exceeded (warning)
    SoftLimitExceeded,

    /// Corridor limits disabled
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corridor_within_limits() {
        let checker = CorridorLimitChecker::new(CorridorLimits::default());

        let result = checker
            .check_corridor_limit("USD", "EUR", Decimal::from(100_000))
            .unwrap();

        assert_eq!(result.status, CorridorStatus::Ok);
        assert!(result.is_allowed());
        assert!(!result.should_warn());
    }

    #[test]
    fn test_corridor_soft_limit_exceeded() {
        let checker = CorridorLimitChecker::new(CorridorLimits::default());

        let result = checker
            .check_corridor_limit("USD", "EUR", Decimal::from(500_000))
            .unwrap();

        assert_eq!(result.status, CorridorStatus::SoftLimitExceeded);
        assert!(result.is_allowed());
        assert!(result.should_warn());
    }

    #[test]
    fn test_corridor_hard_limit_exceeded() {
        let checker = CorridorLimitChecker::new(CorridorLimits::default());

        let result = checker.check_corridor_limit("USD", "EUR", Decimal::from(2_000_000));

        assert!(result.is_err());
    }

    #[test]
    fn test_custom_corridor_limits() {
        let mut checker = CorridorLimitChecker::new(CorridorLimits::default());

        // Set custom limits for USD-PKR corridor
        let pkr_limits = CorridorLimits {
            soft_limit: Decimal::from(50_000),
            hard_limit: Decimal::from(100_000),
            enabled: true,
        };
        checker.set_corridor_limits(Corridor::new("USD", "PKR"), pkr_limits);

        // Should use custom limits
        let result = checker
            .check_corridor_limit("USD", "PKR", Decimal::from(75_000))
            .unwrap();
        assert_eq!(result.status, CorridorStatus::SoftLimitExceeded);

        // Should use default limits for other corridors
        let result2 = checker
            .check_corridor_limit("USD", "EUR", Decimal::from(75_000))
            .unwrap();
        assert_eq!(result2.status, CorridorStatus::Ok);
    }

    #[test]
    fn test_disabled_corridor() {
        let mut checker = CorridorLimitChecker::new(CorridorLimits::default());

        let disabled_limits = CorridorLimits {
            soft_limit: Decimal::from(1),
            hard_limit: Decimal::from(1),
            enabled: false,
        };
        checker.set_corridor_limits(Corridor::new("USD", "XXX"), disabled_limits);

        let result = checker
            .check_corridor_limit("USD", "XXX", Decimal::from(999_999_999))
            .unwrap();

        assert_eq!(result.status, CorridorStatus::Disabled);
        assert!(result.is_allowed());
    }
}
