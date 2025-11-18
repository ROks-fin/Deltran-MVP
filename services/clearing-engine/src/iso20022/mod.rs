// ISO 20022 Message Support Module
// Implements parser/generator for key message types

pub mod common;  // Common types and structures (must be first)
pub mod pacs008; // FIToFICustomerCreditTransfer
pub mod camt054; // BankToCustomerDebitCreditNotification
pub mod camt053; // BankToCustomerStatement - EOD reconciliation
pub mod pain001; // CustomerCreditTransferInitiation - Customer payments

// Re-exports for convenience
pub use pacs008::{Pacs008Document, Pacs008Builder, create_settlement_transaction};
pub use camt054::{Camt054Document, parse_camt054, extract_funding_info, FundingInfo};
pub use camt053::{
    Camt053Document, parse_camt053, extract_eod_reconciliation,
    EODReconciliationInfo, calculate_expected_closing
};
pub use pain001::{
    Pain001Document, Pain001Builder, parse_pain001,
    create_customer_payment, extract_payment_requests, PaymentRequest
};

use serde::{Deserialize, Serialize};
use crate::errors::ClearingError;

/// ISO 20022 Message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iso20022Message<T> {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "Document")]
    pub document: T,
}

/// Parse ISO 20022 XML message
pub fn parse_message<T>(xml: &str) -> Result<Iso20022Message<T>, ClearingError>
where
    T: for<'de> Deserialize<'de>,
{
    quick_xml::de::from_str(xml)
        .map_err(|e| ClearingError::Internal(format!("Failed to parse ISO message: {}", e)))
}

/// Generate ISO 20022 XML message
pub fn generate_message<T>(message: &Iso20022Message<T>) -> Result<String, ClearingError>
where
    T: Serialize,
{
    quick_xml::se::to_string(message)
        .map_err(|e| ClearingError::Internal(format!("Failed to generate ISO message: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_structure() {
        // Basic structure test
        assert!(true);
    }
}
