use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Token status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "token_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenStatus {
    Active,
    Locked,
    Burned,
    Converting,
}

/// Currency type (xINR, xAED, xUSD, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub struct Currency(pub String);

impl Currency {
    pub fn new(code: &str) -> Self {
        // Tokenized currencies have 'x' prefix
        Currency(format!("x{}", code.to_uppercase()))
    }

    pub fn fiat_code(&self) -> String {
        // Remove 'x' prefix to get fiat code
        self.0.trim_start_matches('x').to_string()
    }
}

/// Main Token structure
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Token {
    pub id: Uuid,
    pub currency: String,
    pub amount: Decimal,
    pub bank_id: Uuid,
    pub status: String,
    pub clearing_window: i64,
    pub created_at: DateTime<Utc>,
    pub burned_at: Option<DateTime<Utc>>,
}

/// Token creation request
#[derive(Debug, Deserialize, Serialize, validator::Validate)]
pub struct MintTokenRequest {
    #[validate(length(min = 3, max = 4))]
    pub currency: String,
    pub amount: Decimal,
    pub bank_id: Uuid,
    pub reference: String,
    pub metadata: Option<serde_json::Value>,
}

/// Token burn request
#[derive(Debug, Deserialize, Serialize, validator::Validate)]
pub struct BurnTokenRequest {
    pub token_id: Uuid,
    pub amount: Decimal,
    pub reference: String,
    pub destination_account: String,
}

/// Token transfer request
#[derive(Debug, Deserialize, Serialize, validator::Validate)]
pub struct TransferTokenRequest {
    pub from_bank_id: Uuid,
    pub to_bank_id: Uuid,
    #[validate(length(min = 3, max = 4))]
    pub currency: String,
    pub amount: Decimal,
    pub reference: String,
}

/// Token conversion request (xINR -> xAED)
#[derive(Debug, Deserialize, Serialize, validator::Validate)]
pub struct ConvertTokenRequest {
    pub bank_id: Uuid,
    #[validate(length(min = 3, max = 4))]
    pub from_currency: String,
    #[validate(length(min = 3, max = 4))]
    pub to_currency: String,
    pub amount: Decimal,
    pub reference: String,
}

/// Token response
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub id: Uuid,
    pub currency: String,
    pub amount: Decimal,
    pub bank_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub transaction_reference: String,
}

/// Token balance
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TokenBalance {
    pub bank_id: Uuid,
    pub currency: String,
    pub available_balance: Decimal,
    pub locked_balance: Decimal,
    pub total_balance: Decimal,
}

/// Token event for Kafka
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenEvent {
    pub event_type: TokenEventType,
    pub token_id: Uuid,
    pub bank_id: Uuid,
    pub currency: String,
    pub amount: Decimal,
    pub reference: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TokenEventType {
    Minted,
    Burned,
    Transferred,
    Converted,
    Locked,
    Unlocked,
}