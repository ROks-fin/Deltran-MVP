// Data models for Account Monitor service

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a bank account transaction (from camt.054 or REST API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountTransaction {
    pub transaction_id: String,
    pub account_id: String,
    pub amount: Decimal,
    pub currency: String,
    pub credit_debit_indicator: String, // "CRDT" or "DBIT"
    pub end_to_end_id: Option<String>,
    pub debtor_name: Option<String>,
    pub debtor_account: Option<String>,
    pub booking_date: Option<DateTime<Utc>>,
    pub value_date: Option<DateTime<Utc>>,
}

/// Represents a confirmed funding event (matched transaction)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingEvent {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub transaction_id: String,
    pub account_id: String,
    pub amount: Decimal,
    pub currency: String,
    pub end_to_end_id: Option<String>,
    pub debtor_name: Option<String>,
    pub debtor_account: Option<String>,
    pub booking_date: Option<DateTime<Utc>>,
    pub value_date: Option<DateTime<Utc>>,
    pub confirmed_at: DateTime<Utc>,
}

/// Represents a pending payment waiting for funding confirmation
#[derive(Debug, Clone)]
pub struct PendingPayment {
    pub payment_id: Uuid,
    pub end_to_end_id: Option<String>,
    pub expected_amount: Decimal,
    pub expected_currency: String,
    pub created_at: DateTime<Utc>,
}

/// Represents an unmatched transaction (for manual review)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmatchedTransaction {
    pub id: Uuid,
    pub transaction_id: String,
    pub account_id: String,
    pub amount: Decimal,
    pub currency: String,
    pub credit_debit_indicator: String,
    pub end_to_end_id: Option<String>,
    pub debtor_name: Option<String>,
    pub debtor_account: Option<String>,
    pub booking_date: Option<DateTime<Utc>>,
    pub value_date: Option<DateTime<Utc>>,
    pub detected_at: DateTime<Utc>,
    pub review_status: String, // "PENDING", "MATCHED", "IGNORED"
}
