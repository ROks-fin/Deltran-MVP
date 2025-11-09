use super::{BankClient, TransferRequest, TransferResult, TransferStatus};
use crate::error::{Result, SettlementError};
use async_trait::async_trait;
use rust_decimal::Decimal;
use tracing::warn;

/// Local ACH integration client (stub for future implementation)
pub struct LocalClient {
    // Future: API credentials, endpoint configuration, etc.
}

impl LocalClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl BankClient for LocalClient {
    async fn initiate_transfer(&self, _request: &TransferRequest) -> Result<TransferResult> {
        warn!("Local ACH integration not implemented - use Mock client for MVP");
        Err(SettlementError::Internal(
            "Local ACH integration not implemented".to_string(),
        ))
    }

    async fn get_transfer_status(&self, _external_reference: &str) -> Result<TransferStatus> {
        Err(SettlementError::Internal(
            "Local ACH integration not implemented".to_string(),
        ))
    }

    async fn cancel_transfer(&self, _external_reference: &str) -> Result<()> {
        Err(SettlementError::Internal(
            "Local ACH integration not implemented".to_string(),
        ))
    }

    async fn get_account_balance(&self, _account: &str, _currency: &str) -> Result<Decimal> {
        Err(SettlementError::Internal(
            "Local ACH integration not implemented".to_string(),
        ))
    }
}
