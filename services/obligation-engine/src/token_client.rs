use crate::errors::{ObligationEngineError, Result};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct MintTokenRequest {
    currency: String,
    amount: Decimal,
    bank_id: Uuid,
    reference: String,
    metadata: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct BurnTokenRequest {
    token_id: Uuid,
    amount: Decimal,
    reference: String,
    destination_account: String,
}

#[derive(Debug, Serialize)]
struct ConvertTokenRequest {
    bank_id: Uuid,
    from_currency: String,
    to_currency: String,
    amount: Decimal,
    reference: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub id: Uuid,
    pub currency: String,
    pub amount: Decimal,
    pub bank_id: Uuid,
    pub status: String,
    pub transaction_reference: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct TokenBalance {
    bank_id: Uuid,
    currency: String,
    available_balance: Decimal,
    locked_balance: Decimal,
    total_balance: Decimal,
}

pub struct TokenEngineClient {
    base_url: String,
    client: Client,
}

impl TokenEngineClient {
    pub fn new(base_url: String, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap();

        TokenEngineClient { base_url, client }
    }

    /// Mint tokens for a bank
    pub async fn mint_tokens(
        &self,
        currency: &str,
        amount: Decimal,
        bank_id: Uuid,
        reference: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<TokenResponse> {
        let request = MintTokenRequest {
            currency: currency.to_string(),
            amount,
            bank_id,
            reference: reference.to_string(),
            metadata,
        };

        let url = format!("{}/api/v1/tokens/mint", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to mint tokens: {}", e);
                ObligationEngineError::TokenEngineError(format!("Mint request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ObligationEngineError::TokenEngineError(format!(
                "Mint failed with status {}: {}",
                status, error_text
            )));
        }

        let token_response = response.json::<TokenResponse>().await.map_err(|e| {
            ObligationEngineError::TokenEngineError(format!("Failed to parse response: {}", e))
        })?;

        info!(
            "Minted {} {} for bank {}",
            amount, currency, bank_id
        );

        Ok(token_response)
    }

    /// Convert tokens from one currency to another
    pub async fn convert_tokens(
        &self,
        bank_id: Uuid,
        from_currency: &str,
        to_currency: &str,
        amount: Decimal,
        reference: &str,
    ) -> Result<TokenResponse> {
        let request = ConvertTokenRequest {
            bank_id,
            from_currency: from_currency.to_string(),
            to_currency: to_currency.to_string(),
            amount,
            reference: reference.to_string(),
        };

        let url = format!("{}/api/v1/tokens/convert", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to convert tokens: {}", e);
                ObligationEngineError::TokenEngineError(format!("Convert request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ObligationEngineError::TokenEngineError(format!(
                "Conversion failed with status {}: {}",
                status, error_text
            )));
        }

        let token_response = response.json::<TokenResponse>().await.map_err(|e| {
            ObligationEngineError::TokenEngineError(format!("Failed to parse response: {}", e))
        })?;

        info!(
            "Converted {} {} to {} for bank {}",
            amount, from_currency, to_currency, bank_id
        );

        Ok(token_response)
    }

    /// Get token balance for a bank
    pub async fn get_balance(&self, bank_id: Uuid, currency: Option<&str>) -> Result<Vec<TokenBalance>> {
        let mut url = format!("{}/api/v1/tokens/balance/{}", self.base_url, bank_id);
        if let Some(curr) = currency {
            url = format!("{}?currency={}", url, curr);
        }

        let response = self.client.get(&url).send().await.map_err(|e| {
            error!("Failed to get balance: {}", e);
            ObligationEngineError::TokenEngineError(format!("Balance request failed: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ObligationEngineError::TokenEngineError(format!(
                "Balance check failed with status {}: {}",
                status, error_text
            )));
        }

        #[derive(Deserialize)]
        struct BalanceResponse {
            balances: Vec<TokenBalance>,
        }

        let balance_response = response.json::<BalanceResponse>().await.map_err(|e| {
            ObligationEngineError::TokenEngineError(format!("Failed to parse balance response: {}", e))
        })?;

        Ok(balance_response.balances)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/v1/tokens/health", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}