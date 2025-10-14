//! Velocity controls for transaction monitoring

use crate::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Velocity control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityConfig {
    /// Maximum transactions per account per 24h
    pub max_transactions_per_day: u32,

    /// Maximum total amount per account per 24h
    pub max_amount_per_day: Decimal,

    /// Sliding window duration (default: 24 hours)
    pub window_hours: i64,
}

impl Default for VelocityConfig {
    fn default() -> Self {
        Self {
            max_transactions_per_day: 10,
            max_amount_per_day: Decimal::from(2_000_000), // $2M
            window_hours: 24,
        }
    }
}

/// Transaction record for velocity tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionRecord {
    transaction_id: Uuid,
    amount: Decimal,
    timestamp: DateTime<Utc>,
}

/// Account velocity tracker
struct AccountVelocity {
    transactions: Vec<TransactionRecord>,
}

impl AccountVelocity {
    fn new() -> Self {
        Self {
            transactions: Vec::new(),
        }
    }

    /// Clean up transactions outside the window
    fn cleanup(&mut self, window_start: DateTime<Utc>) {
        self.transactions.retain(|tx| tx.timestamp >= window_start);
    }

    /// Add transaction
    fn add_transaction(&mut self, transaction_id: Uuid, amount: Decimal, timestamp: DateTime<Utc>) {
        self.transactions.push(TransactionRecord {
            transaction_id,
            amount,
            timestamp,
        });
    }

    /// Get transaction count in window
    fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Get total amount in window
    fn total_amount(&self) -> Decimal {
        self.transactions.iter().map(|tx| tx.amount).sum()
    }
}

/// Velocity controller monitors transaction patterns per account
pub struct VelocityController {
    config: VelocityConfig,
    // Map: account_id -> AccountVelocity
    accounts: Arc<DashMap<String, AccountVelocity>>,
}

impl VelocityController {
    /// Create new velocity controller
    pub fn new(config: VelocityConfig) -> Self {
        Self {
            config,
            accounts: Arc::new(DashMap::new()),
        }
    }

    /// Check if transaction violates velocity limits
    pub fn check_velocity(
        &self,
        account_id: &str,
        transaction_id: Uuid,
        amount: Decimal,
    ) -> Result<()> {
        let now = Utc::now();
        let window_start = now - Duration::hours(self.config.window_hours);

        // Get or create account velocity
        let mut account_entry = self.accounts.entry(account_id.to_string()).or_insert_with(AccountVelocity::new);
        let account = account_entry.value_mut();

        // Clean up old transactions
        account.cleanup(window_start);

        // Check transaction count limit
        let current_count = account.transaction_count();
        if current_count >= self.config.max_transactions_per_day as usize {
            return Err(Error::VelocityLimitExceeded(format!(
                "Transaction count limit exceeded: {} >= {} in {}h window",
                current_count, self.config.max_transactions_per_day, self.config.window_hours
            )));
        }

        // Check amount limit
        let current_amount = account.total_amount();
        let new_total = current_amount + amount;
        if new_total > self.config.max_amount_per_day {
            return Err(Error::VelocityLimitExceeded(format!(
                "Amount limit exceeded: {} + {} = {} > {} in {}h window",
                current_amount, amount, new_total, self.config.max_amount_per_day, self.config.window_hours
            )));
        }

        // Record transaction
        account.add_transaction(transaction_id, amount, now);

        Ok(())
    }

    /// Get current velocity stats for an account
    pub fn get_velocity_stats(&self, account_id: &str) -> Option<VelocityStats> {
        let now = Utc::now();
        let window_start = now - Duration::hours(self.config.window_hours);

        self.accounts.get_mut(account_id).map(|mut entry| {
            let account = entry.value_mut();
            account.cleanup(window_start);

            VelocityStats {
                account_id: account_id.to_string(),
                transaction_count: account.transaction_count() as u32,
                total_amount: account.total_amount(),
                remaining_transactions: self.config.max_transactions_per_day.saturating_sub(account.transaction_count() as u32),
                remaining_amount: self.config.max_amount_per_day - account.total_amount(),
                window_start,
                window_end: now,
            }
        })
    }

    /// Clear velocity data for an account (e.g., for testing or manual reset)
    pub fn reset_account(&self, account_id: &str) {
        self.accounts.remove(account_id);
    }

    /// Get total number of tracked accounts
    pub fn tracked_accounts(&self) -> usize {
        self.accounts.len()
    }
}

/// Velocity statistics for an account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityStats {
    pub account_id: String,
    pub transaction_count: u32,
    pub total_amount: Decimal,
    pub remaining_transactions: u32,
    pub remaining_amount: Decimal,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_transaction_count_limit() {
        let config = VelocityConfig {
            max_transactions_per_day: 3,
            max_amount_per_day: Decimal::from(10_000_000),
            window_hours: 24,
        };
        let controller = VelocityController::new(config);

        let account_id = "ACC001";
        let amount = Decimal::from(100);

        // First 3 transactions should succeed
        for i in 0..3 {
            let tx_id = Uuid::new_v4();
            assert!(controller.check_velocity(account_id, tx_id, amount).is_ok());
        }

        // 4th transaction should fail
        let tx_id = Uuid::new_v4();
        assert!(controller.check_velocity(account_id, tx_id, amount).is_err());
    }

    #[test]
    fn test_velocity_amount_limit() {
        let config = VelocityConfig {
            max_transactions_per_day: 100,
            max_amount_per_day: Decimal::from(1_000),
            window_hours: 24,
        };
        let controller = VelocityController::new(config);

        let account_id = "ACC002";

        // Transaction that fits within limit
        let tx1 = Uuid::new_v4();
        assert!(controller.check_velocity(account_id, tx1, Decimal::from(600)).is_ok());

        // Transaction that would exceed limit
        let tx2 = Uuid::new_v4();
        assert!(controller.check_velocity(account_id, tx2, Decimal::from(500)).is_err());

        // Transaction that fits in remaining limit
        let tx3 = Uuid::new_v4();
        assert!(controller.check_velocity(account_id, tx3, Decimal::from(300)).is_ok());
    }

    #[test]
    fn test_velocity_stats() {
        let controller = VelocityController::new(VelocityConfig::default());
        let account_id = "ACC003";

        // Add some transactions
        controller.check_velocity(account_id, Uuid::new_v4(), Decimal::from(1000)).unwrap();
        controller.check_velocity(account_id, Uuid::new_v4(), Decimal::from(2000)).unwrap();

        let stats = controller.get_velocity_stats(account_id).unwrap();
        assert_eq!(stats.transaction_count, 2);
        assert_eq!(stats.total_amount, Decimal::from(3000));
        assert_eq!(stats.remaining_transactions, 8); // 10 - 2
    }

    #[test]
    fn test_velocity_reset() {
        let controller = VelocityController::new(VelocityConfig::default());
        let account_id = "ACC004";

        controller.check_velocity(account_id, Uuid::new_v4(), Decimal::from(1000)).unwrap();
        assert_eq!(controller.tracked_accounts(), 1);

        controller.reset_account(account_id);
        assert_eq!(controller.tracked_accounts(), 0);
    }
}
