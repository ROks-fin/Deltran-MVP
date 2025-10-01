//! Bank connector interface

use crate::{types::*, Error, Result};
use async_trait::async_trait;

/// Bank connector trait
#[async_trait]
pub trait BankConnector: Send + Sync {
    /// Get adapter type
    fn adapter_type(&self) -> AdapterType;

    /// Send transfer to bank
    async fn send_transfer(&self, request: &TransferRequest) -> Result<TransferResponse>;

    /// Check transfer status
    async fn check_status(&self, transfer_id: &str) -> Result<TransferStatus>;

    /// Health check
    async fn health_check(&self) -> Result<()>;

    /// Get connector name
    fn name(&self) -> &str;
}