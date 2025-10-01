//! SWIFT connector adapter

use crate::{connector::BankConnector, iso20022::Pacs008Generator, types::*, Error, Result};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{info, warn};

/// SWIFT adapter configuration
#[derive(Debug, Clone)]
pub struct SwiftConfig {
    /// SWIFT API endpoint
    pub api_endpoint: String,
    /// API key
    pub api_key: String,
    /// Timeout
    pub timeout_seconds: u64,
}

/// SWIFT connector
pub struct SwiftConnector {
    config: SwiftConfig,
    client: Client,
}

impl SwiftConnector {
    /// Create new SWIFT connector
    pub fn new(config: SwiftConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| Error::Connection(e.to_string()))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl BankConnector for SwiftConnector {
    fn adapter_type(&self) -> AdapterType {
        AdapterType::Swift
    }

    async fn send_transfer(&self, request: &TransferRequest) -> Result<TransferResponse> {
        info!(
            "Sending SWIFT transfer {} for corridor {}",
            request.transfer_id, request.corridor_id
        );

        // Generate ISO 20022 pacs.008
        let pacs008 = Pacs008Generator::from_instruction(
            &request.instruction,
            "DELTRANAEXX", // TODO: Config
            "DELTRANUAEX",
        )?;

        let xml = Pacs008Generator::to_xml(&pacs008)?;

        // Send to SWIFT network (mock for MVP)
        // TODO: Real SWIFT Alliance API integration
        let response = self
            .client
            .post(&self.config.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/xml")
            .body(xml)
            .send()
            .await
            .map_err(|e| Error::Connection(e.to_string()))?;

        if response.status().is_success() {
            Ok(TransferResponse {
                transfer_id: request.transfer_id,
                status: TransferStatus::Accepted,
                external_reference: Some(format!("SWIFT-{}", request.transfer_id)),
                message: Some("Transfer accepted by SWIFT".to_string()),
                completed_at: Utc::now(),
            })
        } else {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(Error::BankApi {
                status_code: status,
                message: body,
            })
        }
    }

    async fn check_status(&self, transfer_id: &str) -> Result<TransferStatus> {
        // TODO: Real SWIFT status query
        Ok(TransferStatus::Pending)
    }

    async fn health_check(&self) -> Result<()> {
        // TODO: Real health check
        Ok(())
    }

    fn name(&self) -> &str {
        "SWIFT"
    }
}