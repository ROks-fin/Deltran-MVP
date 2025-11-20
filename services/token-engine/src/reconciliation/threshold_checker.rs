// Threshold Checker - Determines severity of reconciliation discrepancies

use rust_decimal::Decimal;
use std::str::FromStr;
use rust_decimal::prelude::ToPrimitive;

#[derive(Debug, Clone, PartialEq)]
pub enum ThresholdLevel {
    Ok,           // No discrepancy or negligible
    Minor,        // 0.01% - 0.05%
    Significant,  // 0.05% - 0.5%
    Critical,     // > 0.5% or ledger > bank
}

#[derive(Debug, Clone)]
pub struct ThresholdResult {
    pub level: ThresholdLevel,
    pub absolute_difference: Decimal,
    pub percentage_difference: Decimal,
    pub action_required: String,
}

pub struct ThresholdChecker;

impl ThresholdChecker {
    /// Check reconciliation threshold according to DelTran spec
    pub fn check(
        ledger_balance: Decimal,
        bank_reported_balance: Decimal,
    ) -> ThresholdResult {
        let absolute_diff = (ledger_balance - bank_reported_balance).abs();

        // Calculate percentage difference
        let percentage = if bank_reported_balance > Decimal::ZERO {
            (absolute_diff / bank_reported_balance) * Decimal::from(100)
        } else if ledger_balance > Decimal::ZERO {
            // If bank reports zero but we have ledger balance, this is critical
            Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let percentage_f64 = percentage.to_f64().unwrap_or(0.0);

        // Determine threshold level and action
        let (level, action) = if absolute_diff <= Decimal::from_str("0.01").unwrap()
            && percentage_f64 <= 0.01 {
            (ThresholdLevel::Ok, "No action required".to_string())
        } else if percentage_f64 <= 0.05 {
            (
                ThresholdLevel::Minor,
                "Create low-priority reconciliation task, continue operations".to_string()
            )
        } else if percentage_f64 <= 0.5 {
            (
                ThresholdLevel::Significant,
                "Suspend new payouts, create high-priority task for Risk & Finance".to_string()
            )
        } else if ledger_balance > bank_reported_balance {
            (
                ThresholdLevel::Critical,
                "CRITICAL: Halt all payouts, activate circuit breaker, immediate replenishment or emergency burn required".to_string()
            )
        } else {
            (
                ThresholdLevel::Critical,
                "CRITICAL: Bank balance exceeds ledger, investigate immediately".to_string()
            )
        };

        ThresholdResult {
            level,
            absolute_difference: absolute_diff,
            percentage_difference: percentage,
            action_required: action,
        }
    }

    /// Check if account should be suspended based on threshold
    pub fn should_suspend_payouts(threshold_result: &ThresholdResult) -> bool {
        matches!(
            threshold_result.level,
            ThresholdLevel::Significant | ThresholdLevel::Critical
        )
    }

    /// Check if circuit breaker should be activated
    pub fn should_activate_circuit_breaker(threshold_result: &ThresholdResult) -> bool {
        threshold_result.level == ThresholdLevel::Critical
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_threshold_ok() {
        let result = ThresholdChecker::check(dec!(1000.00), dec!(1000.00));
        assert_eq!(result.level, ThresholdLevel::Ok);
        assert!(!ThresholdChecker::should_suspend_payouts(&result));
    }

    #[test]
    fn test_threshold_minor() {
        // 0.02% difference
        let result = ThresholdChecker::check(dec!(100000.00), dec!(100020.00));
        assert_eq!(result.level, ThresholdLevel::Minor);
        assert!(!ThresholdChecker::should_suspend_payouts(&result));
    }

    #[test]
    fn test_threshold_significant() {
        // 0.1% difference
        let result = ThresholdChecker::check(dec!(100000.00), dec!(100100.00));
        assert_eq!(result.level, ThresholdLevel::Significant);
        assert!(ThresholdChecker::should_suspend_payouts(&result));
    }

    #[test]
    fn test_threshold_critical_ledger_exceeds_bank() {
        // Ledger > Bank - most dangerous
        let result = ThresholdChecker::check(dec!(100000.00), dec!(99000.00));
        assert_eq!(result.level, ThresholdLevel::Critical);
        assert!(ThresholdChecker::should_activate_circuit_breaker(&result));
    }
}
