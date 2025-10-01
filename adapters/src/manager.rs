//! Adapter manager (orchestrates all adapters)

use crate::{
    circuit_breaker::*, connector::BankConnector, dlq::DeadLetterQueue, kill_switch::*,
    metrics::*, types::*, Error, Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Adapter manager
pub struct AdapterManager {
    /// Connectors by adapter type
    connectors: Arc<RwLock<HashMap<AdapterType, Arc<dyn BankConnector>>>>,
    /// Circuit breakers
    circuit_breakers: Arc<CircuitBreakerManager>,
    /// Kill switches
    kill_switches: Arc<KillSwitchManager>,
    /// DLQ
    dlq: Arc<DeadLetterQueue>,
}

impl AdapterManager {
    /// Create new manager
    pub fn new(cb_config: CircuitBreakerConfig, dlq_max_size: usize, max_retries: u32) -> Self {
        Self {
            connectors: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(CircuitBreakerManager::new(cb_config)),
            kill_switches: Arc::new(KillSwitchManager::new()),
            dlq: Arc::new(DeadLetterQueue::new(dlq_max_size, max_retries)),
        }
    }

    /// Register connector
    pub async fn register_connector(&self, connector: Arc<dyn BankConnector>) {
        let mut connectors = self.connectors.write().await;
        connectors.insert(connector.adapter_type(), connector);
    }

    /// Send transfer (with circuit breaker + kill switch + DLQ)
    pub async fn send_transfer(&self, request: TransferRequest) -> Result<TransferResponse> {
        let corridor_id = &request.corridor_id;

        // 1. Check kill switch
        self.kill_switches.check_request_allowed(corridor_id).await?;

        // 2. Check circuit breaker
        self.circuit_breakers
            .is_request_allowed(corridor_id)
            .await?;

        // 3. Get connector
        let connectors = self.connectors.read().await;
        let connector = connectors
            .get(&request.adapter_type)
            .ok_or_else(|| Error::UnsupportedAdapter(request.adapter_type.to_string()))?
            .clone();
        drop(connectors);

        // 4. Send transfer
        let start = std::time::Instant::now();
        let result = connector.send_transfer(&request).await;
        let duration = start.elapsed();

        // 5. Record metrics
        ADAPTER_REQUEST_DURATION
            .with_label_values(&[corridor_id, &request.adapter_type.to_string()])
            .observe(duration.as_secs_f64());

        match result {
            Ok(response) => {
                // Success
                self.circuit_breakers.record_success(corridor_id).await;
                ADAPTER_REQUESTS_TOTAL
                    .with_label_values(&[corridor_id, &request.adapter_type.to_string(), "success"])
                    .inc();
                Ok(response)
            }
            Err(e) => {
                // Failure
                error!("Transfer failed for corridor {}: {}", corridor_id, e);
                self.circuit_breakers.record_failure(corridor_id).await;
                ADAPTER_REQUESTS_TOTAL
                    .with_label_values(&[corridor_id, &request.adapter_type.to_string(), "failure"])
                    .inc();

                // Push to DLQ
                if let Err(dlq_err) = self.dlq.push(request, e.to_string()).await {
                    error!("Failed to push to DLQ: {}", dlq_err);
                }

                Err(e)
            }
        }
    }

    /// Get adapter health
    pub async fn get_health(&self, corridor_id: &str) -> AdapterHealth {
        AdapterHealth {
            corridor_id: corridor_id.to_string(),
            adapter_type: AdapterType::Swift, // TODO: per corridor config
            status: HealthStatus::Healthy,
            last_check: chrono::Utc::now(),
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            circuit_breaker_open: self.circuit_breakers.get_state(corridor_id).await
                == crate::circuit_breaker::CircuitState::Open,
            kill_switch_active: self.kill_switches.is_active(corridor_id).await,
            dlq_size: self.dlq.size(corridor_id).await,
        }
    }

    /// Activate kill switch
    pub async fn activate_kill_switch(
        &self,
        corridor_id: &str,
        reason: String,
        activated_by: String,
    ) -> Result<()> {
        self.kill_switches
            .activate(corridor_id, reason, activated_by)
            .await
    }

    /// Deactivate kill switch
    pub async fn deactivate_kill_switch(
        &self,
        corridor_id: &str,
        deactivated_by: String,
    ) -> Result<()> {
        self.kill_switches
            .deactivate(corridor_id, deactivated_by)
            .await
    }
}