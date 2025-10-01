//! Circuit breaker pattern per corridor

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Closed (normal operation)
    Closed,
    /// Open (rejecting requests)
    Open,
    /// Half-open (testing)
    HalfOpen,
}

/// Circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Current state
    state: CircuitState,
    /// Failure count (since last reset)
    failure_count: u32,
    /// Success count (in half-open)
    success_count: u32,
    /// Last failure time
    last_failure_at: Option<DateTime<Utc>>,
    /// Last state change
    last_state_change: DateTime<Utc>,
    /// Config
    config: CircuitBreakerConfig,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold (open after N failures)
    pub failure_threshold: u32,
    /// Timeout (seconds before half-open)
    pub timeout_seconds: u64,
    /// Success threshold (close after N successes in half-open)
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: crate::DEFAULT_CB_FAILURE_THRESHOLD,
            timeout_seconds: crate::DEFAULT_CB_TIMEOUT_SECONDS,
            success_threshold: 2,
        }
    }
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_at: None,
            last_state_change: Utc::now(),
            config,
        }
    }

    /// Check if request is allowed
    pub fn is_request_allowed(&mut self, corridor_id: &str) -> Result<()> {
        match self.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if timeout expired
                if let Some(last_failure) = self.last_failure_at {
                    let elapsed = Utc::now()
                        .signed_duration_since(last_failure)
                        .num_seconds() as u64;

                    if elapsed >= self.config.timeout_seconds {
                        info!("Circuit breaker half-opening for corridor {}", corridor_id);
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        self.last_state_change = Utc::now();
                        Ok(())
                    } else {
                        Err(Error::CircuitBreakerOpen {
                            corridor_id: corridor_id.to_string(),
                            reason: format!(
                                "Circuit open, retry in {}s",
                                self.config.timeout_seconds - elapsed
                            ),
                        })
                    }
                } else {
                    Err(Error::CircuitBreakerOpen {
                        corridor_id: corridor_id.to_string(),
                        reason: "Circuit open".to_string(),
                    })
                }
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }

    /// Record success
    pub fn record_success(&mut self, corridor_id: &str) {
        match self.state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count = 0;
            }
            CircuitState::HalfOpen => {
                self.success_count += 1;

                if self.success_count >= self.config.success_threshold {
                    info!("Circuit breaker closing for corridor {}", corridor_id);
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.last_state_change = Utc::now();
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record failure
    pub fn record_failure(&mut self, corridor_id: &str) {
        self.failure_count += 1;
        self.last_failure_at = Some(Utc::now());

        match self.state {
            CircuitState::Closed => {
                if self.failure_count >= self.config.failure_threshold {
                    warn!(
                        "Circuit breaker opening for corridor {} after {} failures",
                        corridor_id, self.failure_count
                    );
                    self.state = CircuitState::Open;
                    self.last_state_change = Utc::now();
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker re-opening for corridor {}", corridor_id);
                self.state = CircuitState::Open;
                self.success_count = 0;
                self.last_state_change = Utc::now();
            }
            CircuitState::Open => {}
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Reset circuit breaker (manual intervention)
    pub fn reset(&mut self, corridor_id: &str) {
        info!("Manually resetting circuit breaker for corridor {}", corridor_id);
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
        self.last_failure_at = None;
        self.last_state_change = Utc::now();
    }
}

/// Circuit breaker manager (per corridor)
pub struct CircuitBreakerManager {
    /// Circuit breakers by corridor ID
    breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    /// Default config
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerManager {
    /// Create new manager
    pub fn new(default_config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: Arc::new(RwLock::new(HashMap::new())),
            default_config,
        }
    }

    /// Get or create circuit breaker for corridor
    async fn get_or_create(&self, corridor_id: &str) -> CircuitBreaker {
        let breakers = self.breakers.read().await;
        if let Some(breaker) = breakers.get(corridor_id) {
            return breaker.clone();
        }
        drop(breakers);

        // Create new
        let breaker = CircuitBreaker::new(self.default_config.clone());
        let mut breakers = self.breakers.write().await;
        breakers.insert(corridor_id.to_string(), breaker.clone());
        breaker
    }

    /// Check if request is allowed
    pub async fn is_request_allowed(&self, corridor_id: &str) -> Result<()> {
        let mut breakers = self.breakers.write().await;
        let breaker = breakers
            .entry(corridor_id.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.default_config.clone()));

        breaker.is_request_allowed(corridor_id)
    }

    /// Record success
    pub async fn record_success(&self, corridor_id: &str) {
        let mut breakers = self.breakers.write().await;
        if let Some(breaker) = breakers.get_mut(corridor_id) {
            breaker.record_success(corridor_id);
        }
    }

    /// Record failure
    pub async fn record_failure(&self, corridor_id: &str) {
        let mut breakers = self.breakers.write().await;
        if let Some(breaker) = breakers.get_mut(corridor_id) {
            breaker.record_failure(corridor_id);
        }
    }

    /// Get state for corridor
    pub async fn get_state(&self, corridor_id: &str) -> CircuitState {
        let breakers = self.breakers.read().await;
        breakers
            .get(corridor_id)
            .map(|b| b.state())
            .unwrap_or(CircuitState::Closed)
    }

    /// Reset circuit breaker
    pub async fn reset(&self, corridor_id: &str) {
        let mut breakers = self.breakers.write().await;
        if let Some(breaker) = breakers.get_mut(corridor_id) {
            breaker.reset(corridor_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_transitions() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            timeout_seconds: 2,
            success_threshold: 2,
        };

        let mut cb = CircuitBreaker::new(config);

        // Initial state: Closed
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_request_allowed("test").is_ok());

        // Record 3 failures -> Open
        cb.record_failure("test");
        cb.record_failure("test");
        cb.record_failure("test");
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.is_request_allowed("test").is_err());

        // Success in closed state resets failure count
        cb.reset("test");
        cb.record_success("test");
        assert_eq!(cb.failure_count, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_manager() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout_seconds: 60,
            success_threshold: 2,
        };

        let manager = CircuitBreakerManager::new(config);

        // Initial state
        assert!(manager.is_request_allowed("corridor-1").await.is_ok());

        // Record failures
        manager.record_failure("corridor-1").await;
        manager.record_failure("corridor-1").await;

        // Should be open now
        assert_eq!(
            manager.get_state("corridor-1").await,
            CircuitState::Open
        );
        assert!(manager.is_request_allowed("corridor-1").await.is_err());

        // Other corridors unaffected
        assert!(manager.is_request_allowed("corridor-2").await.is_ok());
    }
}