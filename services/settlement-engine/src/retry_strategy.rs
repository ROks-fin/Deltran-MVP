// Retry Strategy - Exponential backoff with jitter for settlement retries

use crate::error::{Result, SettlementError};
use crate::settlement::executor::SettlementRequest;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 2000,      // 2 seconds
            max_delay_ms: 30000,          // 30 seconds
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,           // 10% jitter
        }
    }
}

pub struct RetryStrategy {
    config: RetryConfig,
}

impl RetryStrategy {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Calculate delay for nth retry with exponential backoff + jitter
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.initial_delay_ms as f64
            * self.config.backoff_multiplier.powi(attempt as i32);

        // Cap at max_delay
        let capped_delay = base_delay.min(self.config.max_delay_ms as f64);

        // Add jitter to prevent thundering herd
        let jitter_range = capped_delay * self.config.jitter_factor;
        let jitter = (rand::random::<f64>() - 0.5) * jitter_range * 2.0;
        let final_delay = (capped_delay + jitter).max(0.0);

        Duration::from_millis(final_delay as u64)
    }

    /// Execute operation with retry logic
    pub async fn execute_with_retry<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay = self.calculate_delay(attempt - 1);
                warn!(
                    "Retry attempt {}/{} for {} after {:?}",
                    attempt, self.config.max_retries, operation_name, delay
                );
                tokio::time::sleep(delay).await;
            }

            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!(
                            "Operation {} succeeded on retry attempt {}/{}",
                            operation_name, attempt, self.config.max_retries
                        );
                    }
                    return Ok(result);
                }
                Err(e) => {
                    // Check if error is retryable
                    if !self.is_retryable_error(&e) {
                        warn!(
                            "Non-retryable error for {}: {}",
                            operation_name, e
                        );
                        return Err(e);
                    }

                    warn!(
                        "Attempt {}/{} failed for {}: {}",
                        attempt + 1,
                        self.config.max_retries + 1,
                        operation_name,
                        e
                    );

                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            SettlementError::Internal("Max retries exceeded without error".to_string())
        }))
    }

    /// Determine if an error is retryable
    fn is_retryable_error(&self, error: &SettlementError) -> bool {
        match error {
            // Retryable errors
            SettlementError::BankTransferFailed(_) => true,
            SettlementError::TransferTimeout(_) => true,
            SettlementError::Internal(msg) if msg.contains("timeout") => true,
            SettlementError::Internal(msg) if msg.contains("connection") => true,
            SettlementError::Database(_) => true, // Temporary DB issues

            // Non-retryable errors
            SettlementError::InsufficientFunds { .. } => false,
            SettlementError::AccountNotFound(_) => false,
            SettlementError::LockNotFound(_) => false,
            SettlementError::Validation(_) => false,

            // Conservative: don't retry unknown internal errors
            _ => false,
        }
    }

    /// Check if we should move to next clearing window instead of retrying
    pub fn should_postpone_to_next_window(&self, error: &SettlementError) -> bool {
        matches!(
            error,
            SettlementError::Internal(msg) if msg.contains("maintenance") ||
                                              msg.contains("unavailable") ||
                                              msg.contains("holiday")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0, // No jitter for predictable testing
        };

        let strategy = RetryStrategy::new(config);

        // Attempt 0: 1000ms
        let delay0 = strategy.calculate_delay(0);
        assert_eq!(delay0.as_millis(), 1000);

        // Attempt 1: 2000ms
        let delay1 = strategy.calculate_delay(1);
        assert_eq!(delay1.as_millis(), 2000);

        // Attempt 2: 4000ms
        let delay2 = strategy.calculate_delay(2);
        assert_eq!(delay2.as_millis(), 4000);
    }

    #[test]
    fn test_max_delay_cap() {
        let config = RetryConfig {
            max_retries: 10,
            initial_delay_ms: 1000,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0,
        };

        let strategy = RetryStrategy::new(config);

        // Even with high retry count, delay should cap at max_delay
        let delay = strategy.calculate_delay(10);
        assert!(delay.as_millis() <= 5000);
    }

    #[test]
    fn test_retryable_errors() {
        let strategy = RetryStrategy::with_defaults();

        // Retryable
        assert!(strategy.is_retryable_error(&SettlementError::BankTransferFailed("test".to_string())));
        assert!(strategy.is_retryable_error(&SettlementError::TransferTimeout(30)));

        // Non-retryable
        assert!(!strategy.is_retryable_error(&SettlementError::InsufficientFunds {
            required: rust_decimal::Decimal::new(100, 0),
            available: rust_decimal::Decimal::new(50, 0),
        }));
        assert!(!strategy.is_retryable_error(&SettlementError::AccountNotFound("test".to_string())));
    }
}
