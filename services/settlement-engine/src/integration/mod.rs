pub mod mock;
pub mod swift;
pub mod sepa;
pub mod local;

use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentRail {
    SWIFT,
    SEPA,
    LocalACH,
    Mock,
}

impl fmt::Display for PaymentRail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PaymentRail::SWIFT => write!(f, "SWIFT"),
            PaymentRail::SEPA => write!(f, "SEPA"),
            PaymentRail::LocalACH => write!(f, "LocalACH"),
            PaymentRail::Mock => write!(f, "Mock"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub settlement_id: Uuid,
    pub from_bank: String,
    pub to_bank: String,
    pub amount: Decimal,
    pub currency: String,
    pub reference: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub external_reference: String,
    pub status: TransferStatus,
    pub initiated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferConfirmation {
    pub external_reference: String,
    pub confirmation_code: String,
    pub completed_at: DateTime<Utc>,
}

#[async_trait]
pub trait BankClient: Send + Sync {
    async fn initiate_transfer(&self, request: &TransferRequest) -> Result<TransferResult>;
    async fn get_transfer_status(&self, external_reference: &str) -> Result<TransferStatus>;
    async fn cancel_transfer(&self, external_reference: &str) -> Result<()>;
    async fn get_account_balance(&self, account: &str, currency: &str) -> Result<Decimal>;
}

pub struct BankClientManager {
    mock_client: mock::MockBankClient,
    swift_client: swift::SwiftClient,
    sepa_client: sepa::SepaClient,
    local_client: local::LocalClient,
}

impl BankClientManager {
    pub fn new(mock_latency_ms: u64, mock_success_rate: f64) -> Self {
        Self {
            mock_client: mock::MockBankClient::new(mock_latency_ms, mock_success_rate),
            swift_client: swift::SwiftClient::new(),
            sepa_client: sepa::SepaClient::new(),
            local_client: local::LocalClient::new(),
        }
    }

    pub fn get_client(&self, rail: &PaymentRail) -> &dyn BankClient {
        match rail {
            PaymentRail::SWIFT => &self.swift_client as &dyn BankClient,
            PaymentRail::SEPA => &self.sepa_client as &dyn BankClient,
            PaymentRail::LocalACH => &self.local_client as &dyn BankClient,
            PaymentRail::Mock => &self.mock_client as &dyn BankClient,
        }
    }
}
