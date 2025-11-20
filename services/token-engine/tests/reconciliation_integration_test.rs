// Integration Tests for Reconciliation Service

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::str::FromStr;
    use token_engine::reconciliation::{
        ThresholdChecker,
        threshold_checker::ThresholdLevel,
    };

    #[test]
    fn test_threshold_checker_ok() {
        let result = ThresholdChecker::check(dec!(1000000.00), dec!(1000000.00));
        assert_eq!(result.level, ThresholdLevel::Ok);
        assert_eq!(result.absolute_difference, dec!(0.00));
        assert!(!ThresholdChecker::should_suspend_payouts(&result));
        assert!(!ThresholdChecker::should_activate_circuit_breaker(&result));
    }

    #[test]
    fn test_threshold_checker_minor() {
        // 0.02% difference (200 AED on 1M)
        let result = ThresholdChecker::check(dec!(1000000.00), dec!(1000200.00));
        assert_eq!(result.level, ThresholdLevel::Minor);
        assert_eq!(result.absolute_difference, dec!(200.00));
        assert!(!ThresholdChecker::should_suspend_payouts(&result));
    }

    #[test]
    fn test_threshold_checker_significant() {
        // 0.1% difference (1000 AED on 1M) - should suspend payouts
        let result = ThresholdChecker::check(dec!(1000000.00), dec!(1001000.00));
        assert_eq!(result.level, ThresholdLevel::Significant);
        assert_eq!(result.absolute_difference, dec!(1000.00));
        assert!(ThresholdChecker::should_suspend_payouts(&result));
        assert!(!ThresholdChecker::should_activate_circuit_breaker(&result));
    }

    #[test]
    fn test_threshold_checker_critical_ledger_exceeds_bank() {
        // Ledger > Bank - MOST DANGEROUS (we issued tokens without real money)
        let result = ThresholdChecker::check(dec!(1000000.00), dec!(990000.00));
        assert_eq!(result.level, ThresholdLevel::Critical);
        assert_eq!(result.absolute_difference, dec!(10000.00));
        assert!(ThresholdChecker::should_suspend_payouts(&result));
        assert!(ThresholdChecker::should_activate_circuit_breaker(&result));
    }

    #[test]
    fn test_threshold_checker_critical_bank_exceeds_ledger() {
        // Bank > Ledger - also critical but less dangerous
        let result = ThresholdChecker::check(dec!(1000000.00), dec!(1006000.00));
        assert_eq!(result.level, ThresholdLevel::Critical);
        assert_eq!(result.absolute_difference, dec!(6000.00));
        assert!(ThresholdChecker::should_suspend_payouts(&result));
        // Circuit breaker only activates if ledger > bank
        assert!(!ThresholdChecker::should_activate_circuit_breaker(&result));
    }

    #[test]
    fn test_real_world_scenario_uae_corridor() {
        // UAE corridor: 5M AED on EMI account
        // Bank reports 5.002M AED (200 AED difference = 0.004%)
        let result = ThresholdChecker::check(
            dec!(5000000.00),
            dec!(5000200.00)
        );

        assert_eq!(result.level, ThresholdLevel::Ok);
        assert_eq!(result.absolute_difference, dec!(200.00));

        // Percentage should be ~0.004%
        let percentage = result.percentage_difference.to_string();
        assert!(percentage.starts_with("0.00"));
    }

    #[test]
    fn test_real_world_scenario_india_corridor() {
        // India corridor: 100M INR on EMI account
        // Bank reports 99.95M INR (50k INR difference = 0.05%)
        let result = ThresholdChecker::check(
            dec!(100000000.00),
            dec!(99950000.00)
        );

        assert_eq!(result.level, ThresholdLevel::Significant);
        assert_eq!(result.absolute_difference, dec!(50000.00));
        assert!(ThresholdChecker::should_suspend_payouts(&result));
    }

    #[test]
    fn test_zero_balances() {
        let result = ThresholdChecker::check(dec!(0.00), dec!(0.00));
        assert_eq!(result.level, ThresholdLevel::Ok);
        assert_eq!(result.absolute_difference, dec!(0.00));
    }

    #[test]
    fn test_new_account_first_funding() {
        // New account: ledger=0, bank just received 1M AED
        let result = ThresholdChecker::check(dec!(0.00), dec!(1000000.00));

        // This should be Critical because ledger doesn't match bank
        // We need to mint tokens!
        assert_eq!(result.level, ThresholdLevel::Critical);
        assert_eq!(result.absolute_difference, dec!(1000000.00));
    }

    #[test]
    fn test_precision_handling() {
        // Test 8 decimal places precision
        let result = ThresholdChecker::check(
            Decimal::from_str("1000000.12345678").unwrap(),
            Decimal::from_str("1000000.12345679").unwrap()
        );

        // 0.00000001 difference should be OK
        assert_eq!(result.level, ThresholdLevel::Ok);
    }
}
