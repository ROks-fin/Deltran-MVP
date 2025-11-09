use super::{BankClient, TransferRequest, TransferResult, TransferStatus};
use crate::error::{Result, SettlementError};
use async_trait::async_trait;
use rust_decimal::Decimal;
use tracing::warn;

/// SWIFT integration client (stub for future implementation)
pub struct SwiftClient {
    // Future: API credentials, endpoint configuration, etc.
}

impl SwiftClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl BankClient for SwiftClient {
    async fn initiate_transfer(&self, _request: &TransferRequest) -> Result<TransferResult> {
        warn!("SWIFT integration not implemented - use Mock client for MVP");
        Err(SettlementError::Internal(
            "SWIFT integration not implemented".to_string(),
        ))
    }

    async fn get_transfer_status(&self, _external_reference: &str) -> Result<TransferStatus> {
        Err(SettlementError::Internal(
            "SWIFT integration not implemented".to_string(),
        ))
    }

    async fn cancel_transfer(&self, _external_reference: &str) -> Result<()> {
        Err(SettlementError::Internal(
            "SWIFT integration not implemented".to_string(),
        ))
    }

    async fn get_account_balance(&self, _account: &str, _currency: &str) -> Result<Decimal> {
        Err(SettlementError::Internal(
            "SWIFT integration not implemented".to_string(),
        ))
    }
}
