// Canonical Payment Model - Internal representation
// This is the single source of truth for payment data within DelTran

use chrono::{DateTime, Utc, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalPayment {
    // DelTran IDs
    pub deltran_tx_id: Uuid,
    pub obligation_id: Option<Uuid>,
    pub clearing_batch_id: Option<Uuid>,
    pub settlement_id: Option<Uuid>,

    // ISO 20022 Identifiers
    pub uetr: Option<Uuid>,                    // Universal Transaction Reference
    pub end_to_end_id: String,                 // EndToEndId from pain.001/pacs.008
    pub instruction_id: String,                // InstrId
    pub message_id: String,                    // MsgId

    // Amount & Currency
    pub instructed_amount: Decimal,            // Original amount requested
    pub settlement_amount: Decimal,            // Final settlement amount (after netting)
    pub currency: Currency,
    pub exchange_rate: Option<Decimal>,

    // Parties
    pub debtor: Party,
    pub creditor: Party,
    pub debtor_agent: FinancialInstitution,    // Originating bank
    pub creditor_agent: FinancialInstitution,  // Beneficiary bank
    pub debtor_account: AccountIdentification,
    pub creditor_account: AccountIdentification,

    // Dates
    pub creation_date: DateTime<Utc>,
    pub requested_execution_date: Option<NaiveDate>,
    pub settlement_date: Option<NaiveDate>,
    pub value_date: Option<NaiveDate>,

    // Status
    pub status: PaymentStatus,
    pub status_reason: Option<StatusReason>,

    // Charges & Fees
    pub charge_bearer: ChargeBearer,
    pub charges: Vec<Charge>,

    // Remittance Information
    pub remittance_info: String,
    pub remittance_structured: Option<StructuredRemittance>,

    // Risk & Compliance
    pub risk_score: Option<f64>,
    pub compliance_status: ComplianceStatus,
    pub sanctions_checked: bool,
    pub aml_score: Option<f64>,

    // Corridor & Routing
    pub corridor: String,                      // e.g., "UAE_TO_INDIA", "UK_TO_UAE"
    pub priority: Priority,
    pub liquidity_pool_id: Option<Uuid>,

    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub name: String,
    pub postal_address: Option<PostalAddress>,
    pub identification: Option<String>,        // Tax ID, passport, etc.
    pub country_code: String,                  // ISO 3166-1 alpha-2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostalAddress {
    pub street_name: Option<String>,
    pub building_number: Option<String>,
    pub post_code: Option<String>,
    pub town_name: Option<String>,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialInstitution {
    pub bic: Option<String>,                   // BIC/SWIFT code
    pub name: String,
    pub country_code: String,
    pub clearing_system_member_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountIdentification {
    pub iban: Option<String>,
    pub bban: Option<String>,
    pub other: Option<String>,
    pub account_type: AccountType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    Checking,
    Savings,
    Emi,        // E-money institution account
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Currency {
    Usd,
    Eur,
    Gbp,
    Aed,
    Inr,
    Sar,
    Qar,
    Omr,
    Kwd,
}

impl Currency {
    pub fn to_string(&self) -> String {
        match self {
            Currency::Usd => "USD".to_string(),
            Currency::Eur => "EUR".to_string(),
            Currency::Gbp => "GBP".to_string(),
            Currency::Aed => "AED".to_string(),
            Currency::Inr => "INR".to_string(),
            Currency::Sar => "SAR".to_string(),
            Currency::Qar => "QAR".to_string(),
            Currency::Omr => "OMR".to_string(),
            Currency::Kwd => "KWD".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "USD" => Some(Currency::Usd),
            "EUR" => Some(Currency::Eur),
            "GBP" => Some(Currency::Gbp),
            "AED" => Some(Currency::Aed),
            "INR" => Some(Currency::Inr),
            "SAR" => Some(Currency::Sar),
            "QAR" => Some(Currency::Qar),
            "OMR" => Some(Currency::Omr),
            "KWD" => Some(Currency::Kwd),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    // Initial states
    Received,               // pain.001 received by Gateway
    Validated,              // Passed XSD and business rules
    Accepted,               // Accepted by bank (pacs.002/pain.002)
    Pending,                // Pending processing
    Rejected,               // Rejected by bank

    // Obligation states
    PendingFunding,         // Waiting for camt.054
    Funded,                 // camt.054 received, tokens minted

    // Clearing states
    ReadyForClearing,       // Ready to enter clearing window
    Clearing,               // In clearing batch
    Netted,                 // Multilateral netting complete

    // Settlement states
    ReadyForSettlement,     // Cleared, ready to settle
    Settling,               // pacs.008 sent to bank
    Executed,               // Bank accepted
    Reconciled,             // camt.054 confirmation received

    // Final states
    Completed,              // Full end-to-end success
    Failed,                 // Failed at any stage
    Cancelled,              // Cancelled by customer/bank
    Returned,               // pacs.004 return received
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReason {
    pub code: String,       // ISO 20022 reason code (e.g., "AC01", "AM04")
    pub description: String,
    pub additional_info: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChargeBearer {
    Shar,   // Shared - default
    Slev,   // Service level
    Debt,   // Debtor pays all
    Cred,   // Creditor pays all
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Charge {
    pub charge_type: ChargeType,
    pub amount: Decimal,
    pub currency: Currency,
    pub party: ChargeParty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChargeType {
    CorridorFee,        // DelTran fee
    FxConversion,       // FX markup
    BankFee,            // Bank processing fee
    RegulatoryFee,      // Government/regulator fee
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChargeParty {
    Debtor,
    Creditor,
    Shared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredRemittance {
    pub invoice_number: Option<String>,
    pub invoice_date: Option<NaiveDate>,
    pub purchase_order: Option<String>,
    pub additional_info: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComplianceStatus {
    Pending,
    Approved,
    RequiresReview,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Priority {
    High,       // Urgent, process ASAP
    Normal,     // Standard processing
    Low,        // Batch, can wait
}

// Helper functions for CanonicalPayment
impl CanonicalPayment {
    pub fn new(
        end_to_end_id: String,
        instruction_id: String,
        message_id: String,
        amount: Decimal,
        currency: Currency,
        debtor: Party,
        creditor: Party,
        debtor_agent: FinancialInstitution,
        creditor_agent: FinancialInstitution,
    ) -> Self {
        let now = Utc::now();

        Self {
            deltran_tx_id: Uuid::new_v4(),
            obligation_id: None,
            clearing_batch_id: None,
            settlement_id: None,
            uetr: Some(Uuid::new_v4()), // Always generate UETR for ISO 20022 compliance
            end_to_end_id,
            instruction_id,
            message_id,
            instructed_amount: amount,
            settlement_amount: amount,
            currency,
            exchange_rate: None,
            debtor,
            creditor,
            debtor_agent,
            creditor_agent,
            debtor_account: AccountIdentification {
                iban: None,
                bban: None,
                other: None,
                account_type: AccountType::Other,
            },
            creditor_account: AccountIdentification {
                iban: None,
                bban: None,
                other: None,
                account_type: AccountType::Other,
            },
            creation_date: now,
            requested_execution_date: None,
            settlement_date: None,
            value_date: None,
            status: PaymentStatus::Received,
            status_reason: None,
            charge_bearer: ChargeBearer::Shar,
            charges: vec![],
            remittance_info: String::new(),
            remittance_structured: None,
            risk_score: None,
            compliance_status: ComplianceStatus::Pending,
            sanctions_checked: false,
            aml_score: None,
            corridor: String::from("UNKNOWN"),
            priority: Priority::Normal,
            liquidity_pool_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_status(&mut self, status: PaymentStatus, reason: Option<StatusReason>) {
        self.status = status;
        self.status_reason = reason;
        self.updated_at = Utc::now();
    }

    pub fn is_final_status(&self) -> bool {
        matches!(
            self.status,
            PaymentStatus::Completed | PaymentStatus::Failed | PaymentStatus::Cancelled | PaymentStatus::Returned
        )
    }

    pub fn can_cancel(&self) -> bool {
        matches!(
            self.status,
            PaymentStatus::Received
                | PaymentStatus::Validated
                | PaymentStatus::PendingFunding
                | PaymentStatus::ReadyForClearing
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_canonical_payment_creation() {
        let payment = CanonicalPayment::new(
            "E2E123".to_string(),
            "INSTR456".to_string(),
            "MSG789".to_string(),
            dec!(1000.00),
            Currency::Usd,
            Party {
                name: "John Doe".to_string(),
                postal_address: None,
                identification: None,
                country_code: "AE".to_string(),
            },
            Party {
                name: "Jane Smith".to_string(),
                postal_address: None,
                identification: None,
                country_code: "IN".to_string(),
            },
            FinancialInstitution {
                bic: Some("BANKAEXXXX".to_string()),
                name: "Bank A".to_string(),
                country_code: "AE".to_string(),
                clearing_system_member_id: None,
            },
            FinancialInstitution {
                bic: Some("BANKINXXXX".to_string()),
                name: "Bank IN".to_string(),
                country_code: "IN".to_string(),
                clearing_system_member_id: None,
            },
        );

        assert_eq!(payment.end_to_end_id, "E2E123");
        assert_eq!(payment.instructed_amount, dec!(1000.00));
        assert_eq!(payment.status, PaymentStatus::Received);
        assert!(!payment.is_final_status());
        assert!(payment.can_cancel());
    }

    #[test]
    fn test_status_transitions() {
        let mut payment = CanonicalPayment::new(
            "E2E123".to_string(),
            "INSTR456".to_string(),
            "MSG789".to_string(),
            dec!(1000.00),
            Currency::Usd,
            Party {
                name: "John".to_string(),
                postal_address: None,
                identification: None,
                country_code: "AE".to_string(),
            },
            Party {
                name: "Jane".to_string(),
                postal_address: None,
                identification: None,
                country_code: "IN".to_string(),
            },
            FinancialInstitution {
                bic: Some("BANKAEXXXX".to_string()),
                name: "Bank A".to_string(),
                country_code: "AE".to_string(),
                clearing_system_member_id: None,
            },
            FinancialInstitution {
                bic: Some("BANKINXXXX".to_string()),
                name: "Bank IN".to_string(),
                country_code: "IN".to_string(),
                clearing_system_member_id: None,
            },
        );

        assert!(payment.can_cancel());

        payment.update_status(PaymentStatus::Funded, None);
        assert_eq!(payment.status, PaymentStatus::Funded);

        payment.update_status(PaymentStatus::Completed, None);
        assert!(payment.is_final_status());
        assert!(!payment.can_cancel());
    }
}
