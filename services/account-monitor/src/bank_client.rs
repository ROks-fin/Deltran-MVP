// Bank API client for fetching transactions

use crate::config::AccountConfig;
use crate::models::AccountTransaction;
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

pub struct BankClient {
    config: AccountConfig,
    http_client: Client,
}

#[derive(Debug, Deserialize)]
struct BankApiResponse {
    transactions: Vec<BankApiTransaction>,
}

#[derive(Debug, Deserialize)]
struct BankApiTransaction {
    transaction_id: String,
    amount: Decimal,
    currency: String,
    credit_debit_indicator: String,
    end_to_end_id: Option<String>,
    debtor_name: Option<String>,
    debtor_account: Option<String>,
    booking_date: String,
    value_date: String,
}

impl BankClient {
    pub fn new(config: AccountConfig) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
        }
    }

    pub fn get_account_id(&self) -> &str {
        &self.config.account_id
    }

    /// Fetch recent transactions from bank API
    pub async fn fetch_recent_transactions(&self) -> Result<Vec<AccountTransaction>> {
        match self.config.api_type.as_str() {
            "REST" => self.fetch_via_rest_api().await,
            "ISO20022" => self.fetch_via_iso20022().await,
            _ => {
                warn!("Unknown API type: {}, returning empty", self.config.api_type);
                Ok(Vec::new())
            }
        }
    }

    /// Fetch via REST API
    async fn fetch_via_rest_api(&self) -> Result<Vec<AccountTransaction>> {
        info!("🔌 Fetching transactions via REST API for account {}", self.config.account_id);

        // Build API endpoint
        let url = format!(
            "{}/accounts/{}/transactions?since={}",
            self.config.api_endpoint,
            self.config.account_id,
            // Last 5 minutes
            (Utc::now() - chrono::Duration::minutes(5)).to_rfc3339()
        );

        // Make authenticated request
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Bank API returned error: {}",
                response.status()
            ));
        }

        let api_response: BankApiResponse = response.json().await?;

        // Convert to AccountTransaction
        let transactions: Vec<AccountTransaction> = api_response
            .transactions
            .into_iter()
            .map(|t| AccountTransaction {
                transaction_id: t.transaction_id,
                account_id: self.config.account_id.clone(),
                amount: t.amount,
                currency: t.currency,
                credit_debit_indicator: t.credit_debit_indicator,
                end_to_end_id: t.end_to_end_id,
                debtor_name: t.debtor_name,
                debtor_account: t.debtor_account,
                booking_date: t.booking_date.parse().ok(),
                value_date: t.value_date.parse().ok(),
            })
            .collect();

        info!("✅ Fetched {} transactions via REST API", transactions.len());

        Ok(transactions)
    }

    /// Fetch via ISO 20022 camt.052 (Account Report)
    async fn fetch_via_iso20022(&self) -> Result<Vec<AccountTransaction>> {
        info!("📄 Fetching transactions via ISO 20022 camt.052 for account {}", self.config.account_id);

        // Build camt.052 request
        let camt052_request = self.build_camt052_request();

        // Send to bank's ISO endpoint
        let response = self.http_client
            .post(&self.config.api_endpoint)
            .header("Content-Type", "application/xml")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .body(camt052_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Bank ISO 20022 endpoint returned error: {}",
                response.status()
            ));
        }

        let xml = response.text().await?;

        // Parse camt.052 response
        // TODO: Implement proper camt.052 parser
        // For now, return empty
        warn!("⚠️  camt.052 parsing not yet implemented");
        Ok(Vec::new())
    }

    /// Build camt.052 request (Account Report Request)
    fn build_camt052_request(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.052.001.08">
  <BkToCstmrAcctRpt>
    <GrpHdr>
      <MsgId>{}</MsgId>
      <CreDtTm>{}</CreDtTm>
    </GrpHdr>
    <Rpt>
      <Acct>
        <Id>
          <IBAN>{}</IBAN>
        </Id>
      </Acct>
      <Bal>
        <Tp>
          <CdOrPrtry>
            <Cd>OPBD</Cd>
          </CdOrPrtry>
        </Tp>
      </Bal>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#,
            Uuid::new_v4(),
            Utc::now().format("%Y-%m-%dT%H:%M:%S"),
            self.config.account_id
        )
    }
}
