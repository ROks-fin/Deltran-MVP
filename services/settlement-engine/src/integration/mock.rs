use super::{BankClient, TransferRequest, TransferResult, TransferStatus};
use crate::error::{Result, SettlementError};
use async_trait::async_trait;
use chrono::Utc;
use rand::Rng;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

pub struct MockBankClient {
    latency_ms: u64,
    success_rate: f64,
    transfers: Arc<RwLock<HashMap<String, MockTransfer>>>,
}

#[derive(Debug, Clone)]
struct MockTransfer {
    reference: String,
    status: TransferStatus,
    initiated_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
}

impl MockBankClient {
    pub fn new(latency_ms: u64, success_rate: f64) -> Self {
        Self {
            latency_ms,
            success_rate,
            transfers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn should_succeed(&self) -> bool {
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() <= self.success_rate
    }

    async fn simulate_processing(&self, reference: &str) {
        let transfers = self.transfers.clone();
        let reference = reference.to_string();

        tokio::spawn(async move {
            // Simulate processing time (5-10 seconds)
            tokio::time::sleep(Duration::from_secs(5)).await;

            let mut transfers = transfers.write().await;
            if let Some(transfer) = transfers.get_mut(&reference) {
                transfer.status = TransferStatus::Completed;
                transfer.completed_at = Some(Utc::now());
                info!("Mock transfer {} completed", reference);
            }
        });
    }
}

#[async_trait]
impl BankClient for MockBankClient {
    async fn initiate_transfer(&self, request: &TransferRequest) -> Result<TransferResult> {
        info!(
            "Mock bank: Initiating transfer {} {} from {} to {}",
            request.amount, request.currency, request.from_bank, request.to_bank
        );

        // Simulate network latency
        tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;

        // Simulate random failures
        if !self.should_succeed() {
            warn!("Mock bank: Simulated transfer failure");
            return Err(SettlementError::BankTransferFailed(
                "Simulated bank failure".to_string(),
            ));
        }

        // Generate mock external reference
        let external_reference = format!("MOCK-{}", Uuid::new_v4());
        let now = Utc::now();

        let transfer = MockTransfer {
            reference: external_reference.clone(),
            status: TransferStatus::Processing,
            initiated_at: now,
            completed_at: None,
        };

        // Store transfer
        self.transfers
            .write()
            .await
            .insert(external_reference.clone(), transfer);

        // Start async processing
        self.simulate_processing(&external_reference).await;

        Ok(TransferResult {
            external_reference,
            status: TransferStatus::Processing,
            initiated_at: now,
        })
    }

    async fn get_transfer_status(&self, external_reference: &str) -> Result<TransferStatus> {
        // Simulate network latency
        tokio::time::sleep(Duration::from_millis(self.latency_ms / 2)).await;

        let transfers = self.transfers.read().await;
        match transfers.get(external_reference) {
            Some(transfer) => Ok(transfer.status.clone()),
            None => Err(SettlementError::Internal(format!(
                "Transfer not found: {}",
                external_reference
            ))),
        }
    }

    async fn cancel_transfer(&self, external_reference: &str) -> Result<()> {
        info!("Mock bank: Cancelling transfer {}", external_reference);

        // Simulate network latency
        tokio::time::sleep(Duration::from_millis(self.latency_ms / 3)).await;

        let mut transfers = self.transfers.write().await;
        if let Some(transfer) = transfers.get_mut(external_reference) {
            if transfer.status == TransferStatus::Processing {
                transfer.status = TransferStatus::Cancelled;
                info!("Mock transfer {} cancelled", external_reference);
                Ok(())
            } else {
                Err(SettlementError::Internal(format!(
                    "Cannot cancel transfer in status {:?}",
                    transfer.status
                )))
            }
        } else {
            Err(SettlementError::Internal(format!(
                "Transfer not found: {}",
                external_reference
            )))
        }
    }

    async fn get_account_balance(&self, account: &str, currency: &str) -> Result<Decimal> {
        info!(
            "Mock bank: Getting balance for account {} currency {}",
            account, currency
        );

        // Simulate network latency
        tokio::time::sleep(Duration::from_millis(self.latency_ms / 4)).await;

        // Return mock balance (very high to never block settlements)
        Ok(Decimal::from(10_000_000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_transfer_success() {
        let client = MockBankClient::new(10, 1.0); // 100% success rate
        let request = TransferRequest {
            settlement_id: Uuid::new_v4(),
            from_bank: "BANK001".to_string(),
            to_bank: "BANK002".to_string(),
            amount: Decimal::from(1000),
            currency: "USD".to_string(),
            reference: "TEST-REF".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = client.initiate_transfer(&request).await;
        assert!(result.is_ok());

        let transfer = result.unwrap();
        assert_eq!(transfer.status, TransferStatus::Processing);
        assert!(transfer.external_reference.starts_with("MOCK-"));
    }

    #[tokio::test]
    async fn test_mock_balance() {
        let client = MockBankClient::new(10, 1.0);
        let balance = client.get_account_balance("TEST_ACCOUNT", "USD").await;
        assert!(balance.is_ok());
        assert!(balance.unwrap() > Decimal::ZERO);
    }
}
