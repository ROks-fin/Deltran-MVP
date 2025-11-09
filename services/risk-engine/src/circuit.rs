use crate::errors::{RiskError, RiskResult};
use crate::models::{CircuitBreakerState, CircuitState};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitBreakerState>>,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self::with_config("risk_engine".to_string(), 5, 3, 60)
    }

    pub fn with_config(
        id: String,
        failure_threshold: u32,
        recovery_threshold: u32,
        timeout_seconds: i64,
    ) -> Self {
        CircuitBreaker {
            state: Arc::new(RwLock::new(CircuitBreakerState {
                id,
                state: CircuitState::Closed,
                failure_count: 0,
                failure_threshold,
                success_count: 0,
                recovery_threshold,
                last_failure_time: None,
                timeout_duration: timeout_seconds,
            })),
        }
    }

    /// Execute a function with circuit breaker protection
    pub async fn call<F, T, Fut>(&self, f: F) -> RiskResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = RiskResult<T>>,
    {
        // Check if we should attempt to recover from open state
        {
            let mut state = self.state.write().await;
            if state.state == CircuitState::Open && self.should_attempt_reset(&state) {
                info!("Circuit breaker {} transitioning to HalfOpen", state.id);
                state.state = CircuitState::HalfOpen;
                state.success_count = 0;
            }
        }

        // Check current state
        {
            let state = self.state.read().await;
            if state.state == CircuitState::Open {
                warn!("Circuit breaker {} is OPEN, rejecting request", state.id);
                return Err(RiskError::CircuitBreakerOpen);
            }
        }

        // Execute the function
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }

    /// Record a successful operation
    pub async fn on_success(&self) {
        let mut state = self.state.write().await;

        match state.state {
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= state.recovery_threshold {
                    info!(
                        "Circuit breaker {} recovered - transitioning to Closed",
                        state.id
                    );
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                if state.failure_count > 0 {
                    state.failure_count = 0;
                }
            }
            _ => {}
        }
    }

    /// Record a failed operation
    pub async fn on_failure(&self) {
        let mut state = self.state.write().await;
        state.last_failure_time = Some(Utc::now());

        match state.state {
            CircuitState::Closed => {
                state.failure_count += 1;
                if state.failure_count >= state.failure_threshold {
                    warn!(
                        "Circuit breaker {} TRIPPED - transitioning to Open (failures: {})",
                        state.id, state.failure_count
                    );
                    state.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                warn!(
                    "Circuit breaker {} failed in HalfOpen - back to Open",
                    state.id
                );
                state.state = CircuitState::Open;
                state.failure_count = 0;
                state.success_count = 0;
            }
            _ => {}
        }
    }

    /// Manually reset the circuit breaker
    pub async fn reset(&self) {
        let mut state = self.state.write().await;
        info!("Circuit breaker {} manually reset to Closed", state.id);
        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure_time = None;
    }

    /// Get current state
    pub async fn get_state(&self) -> CircuitBreakerState {
        self.state.read().await.clone()
    }

    /// Check if enough time has passed to attempt recovery
    fn should_attempt_reset(&self, state: &CircuitBreakerState) -> bool {
        if let Some(last_failure) = state.last_failure_time {
            let elapsed = Utc::now() - last_failure;
            elapsed.num_seconds() >= state.timeout_duration
        } else {
            true
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::with_config("test".to_string(), 3, 2, 1);

        // First 3 failures should open the circuit
        for _ in 0..3 {
            cb.on_failure().await;
        }

        let state = cb.get_state().await;
        assert_eq!(state.state, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovers() {
        let cb = CircuitBreaker::with_config("test".to_string(), 3, 2, 0);

        // Open the circuit
        for _ in 0..3 {
            cb.on_failure().await;
        }

        // Should transition to HalfOpen after timeout (0 seconds for test)
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let result: RiskResult<()> = cb.call(|| async { Ok(()) }).await;
        assert!(result.is_ok());

        let state = cb.get_state().await;
        assert_eq!(state.state, CircuitState::HalfOpen);

        // Another success should close it
        cb.on_success().await;
        let state = cb.get_state().await;
        assert_eq!(state.state, CircuitState::Closed);
    }
}
