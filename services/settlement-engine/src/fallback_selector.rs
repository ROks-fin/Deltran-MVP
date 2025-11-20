// Fallback Selector - Selects backup banks when primary fails

use crate::error::{Result, SettlementError};
use crate::integration::PaymentRail;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankRoute {
    pub bank_id: Uuid,
    pub bank_code: String,
    pub bank_name: String,
    pub rail: PaymentRail,
    pub priority: i32,           // Lower = higher priority
    pub is_active: bool,
    pub health_score: f64,       // 0.0 - 1.0
    pub avg_latency_ms: f64,
    pub success_rate: f64,       // 0.0 - 1.0
    pub last_failure_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct FallbackDecision {
    pub use_fallback: bool,
    pub selected_bank: Option<BankRoute>,
    pub reason: String,
}

pub struct FallbackSelector {
    pool: PgPool,
    health_threshold: f64,
    min_success_rate: f64,
}

impl FallbackSelector {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            health_threshold: 0.7,   // 70% health required
            min_success_rate: 0.95,  // 95% success rate required
        }
    }

    /// Select best bank for settlement, with fallback if primary unavailable
    pub async fn select_bank_with_fallback(
        &self,
        currency: &str,
        preferred_rail: PaymentRail,
    ) -> Result<FallbackDecision> {
        // Get all available banks for this corridor
        let available_banks = self.get_available_banks(currency, &preferred_rail).await?;

        if available_banks.is_empty() {
            return Err(SettlementError::Internal(format!(
                "No banks available for currency {} on rail {:?}",
                currency, preferred_rail
            )));
        }

        // Sort by priority, health, and success rate
        let mut ranked_banks = available_banks;
        ranked_banks.sort_by(|a, b| {
            // Primary: priority
            match a.priority.cmp(&b.priority) {
                std::cmp::Ordering::Equal => {
                    // Secondary: health score
                    let score_a = a.health_score * a.success_rate;
                    let score_b = b.health_score * b.success_rate;
                    score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                }
                other => other,
            }
        });

        // Primary bank (priority 1)
        let primary = ranked_banks.first().unwrap();

        // Check if primary is healthy
        if self.is_bank_healthy(primary) {
            info!(
                "Selected primary bank {} for currency {} (health={:.2}, success_rate={:.2})",
                primary.bank_code, currency, primary.health_score, primary.success_rate
            );

            return Ok(FallbackDecision {
                use_fallback: false,
                selected_bank: Some(primary.clone()),
                reason: format!("Primary bank {} is healthy", primary.bank_code),
            });
        }

        // Primary unhealthy - select fallback
        warn!(
            "Primary bank {} is unhealthy (health={:.2}, success_rate={:.2}). Selecting fallback",
            primary.bank_code, primary.health_score, primary.success_rate
        );

        // Find best fallback
        for bank in ranked_banks.iter().skip(1) {
            if self.is_bank_healthy(bank) {
                info!(
                    "Selected fallback bank {} for currency {}",
                    bank.bank_code, currency
                );

                return Ok(FallbackDecision {
                    use_fallback: true,
                    selected_bank: Some(bank.clone()),
                    reason: format!(
                        "Fallback to {} (primary {} unhealthy)",
                        bank.bank_code, primary.bank_code
                    ),
                });
            }
        }

        // No healthy banks available
        Err(SettlementError::Internal(format!(
            "No healthy banks available for currency {} on rail {:?}",
            currency, preferred_rail
        )))
    }

    /// Get available banks for currency and rail
    async fn get_available_banks(
        &self,
        currency: &str,
        rail: &PaymentRail,
    ) -> Result<Vec<BankRoute>> {
        // For MVP, return mock data
        // In production, this would query bank_routes table
        Ok(vec![
            BankRoute {
                bank_id: Uuid::new_v4(),
                bank_code: "ENBD".to_string(),
                bank_name: "Emirates NBD".to_string(),
                rail: rail.clone(),
                priority: 1,
                is_active: true,
                health_score: 0.95,
                avg_latency_ms: 1200.0,
                success_rate: 0.98,
                last_failure_at: None,
            },
            BankRoute {
                bank_id: Uuid::new_v4(),
                bank_code: "FAB".to_string(),
                bank_name: "First Abu Dhabi Bank".to_string(),
                rail: rail.clone(),
                priority: 2,
                is_active: true,
                health_score: 0.92,
                avg_latency_ms: 1500.0,
                success_rate: 0.96,
                last_failure_at: None,
            },
        ])
    }

    /// Check if bank is healthy enough for settlement
    fn is_bank_healthy(&self, bank: &BankRoute) -> bool {
        if !bank.is_active {
            return false;
        }

        // Check health threshold
        if bank.health_score < self.health_threshold {
            warn!(
                "Bank {} below health threshold: {:.2} < {:.2}",
                bank.bank_code, bank.health_score, self.health_threshold
            );
            return false;
        }

        // Check success rate
        if bank.success_rate < self.min_success_rate {
            warn!(
                "Bank {} below success rate threshold: {:.2}% < {:.2}%",
                bank.bank_code,
                bank.success_rate * 100.0,
                self.min_success_rate * 100.0
            );
            return false;
        }

        // Check recent failures (don't use if failed in last 5 minutes)
        if let Some(last_failure) = bank.last_failure_at {
            let minutes_since_failure = (Utc::now() - last_failure).num_minutes();
            if minutes_since_failure < 5 {
                warn!(
                    "Bank {} failed {} minutes ago, too recent",
                    bank.bank_code, minutes_since_failure
                );
                return false;
            }
        }

        true
    }

    /// Record bank failure for health tracking
    pub async fn record_bank_failure(
        &self,
        bank_code: &str,
        error_message: &str,
    ) -> Result<()> {
        info!("Recording failure for bank {}: {}", bank_code, error_message);

        // For MVP, just log
        // In production, update bank_health_metrics table
        // This would be used to calculate health_score and success_rate

        Ok(())
    }

    /// Record successful settlement for health tracking
    pub async fn record_bank_success(
        &self,
        bank_code: &str,
        latency_ms: f64,
    ) -> Result<()> {
        info!(
            "Recording success for bank {} (latency: {:.0}ms)",
            bank_code, latency_ms
        );

        // For MVP, just log
        // In production, update bank_health_metrics table

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bank_health_check() {
        let selector = FallbackSelector {
            pool: sqlx::PgPool::connect_lazy("").unwrap(), // Mock
            health_threshold: 0.7,
            min_success_rate: 0.95,
        };

        // Healthy bank
        let healthy = BankRoute {
            bank_id: Uuid::new_v4(),
            bank_code: "TEST".to_string(),
            bank_name: "Test Bank".to_string(),
            rail: PaymentRail::Mock,
            priority: 1,
            is_active: true,
            health_score: 0.95,
            avg_latency_ms: 1000.0,
            success_rate: 0.98,
            last_failure_at: None,
        };

        assert!(selector.is_bank_healthy(&healthy));

        // Low health
        let unhealthy = BankRoute {
            health_score: 0.5, // Below threshold
            ..healthy.clone()
        };

        assert!(!selector.is_bank_healthy(&unhealthy));

        // Low success rate
        let low_success = BankRoute {
            success_rate: 0.90, // Below threshold
            ..healthy.clone()
        };

        assert!(!selector.is_bank_healthy(&low_success));

        // Recent failure
        let recent_failure = BankRoute {
            last_failure_at: Some(Utc::now()),
            ..healthy
        };

        assert!(!selector.is_bank_healthy(&recent_failure));
    }
}
