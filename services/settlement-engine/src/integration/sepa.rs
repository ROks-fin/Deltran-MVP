use super::{BankClient, TransferRequest, TransferResult, TransferStatus};
use crate::error::{Result, SettlementError};
use async_trait::async_trait;
use rust_decimal::Decimal;
use tracing::warn;

/// SEPA integration client (stub for future implementation)
pub struct SepaClient {
    // Future: API credentials, endpoint configuration, etc.
}

impl SepaClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl BankClient for SepaClient {
    async fn initiate_transfer(&self, _request: &TransferRequest) -> Result<TransferResult> {
        warn!("SEPA integration not implemented - use Mock client for MVP");
        Err(SettlementError::Internal(
            "SEPA integration not implemented".to_string(),
        ))
    }

    async fn get_transfer_status(&self, _external_reference: &str) -> Result<TransferStatus> {
        Err(SettlementError::Internal(
            "SEPA integration not implemented".to_string(),
        ))
    }

    async fn cancel_transfer(&self, _external_reference: &str) -> Result<()> {
        Err(SettlementError::Internal(
            "SEPA integration not implemented".to_string(),
        ))
    }

    async fn get_account_balance(&self, _account: &str, _currency: &str) -> Result<Decimal> {
        Err(SettlementError::Internal(
            "SEPA integration not implemented".to_string(),
        ))
    }
}
