// camt.054.001.10 - BankToCustomerDebitCreditNotification
// Critical for triggering mint operations when funding received

use super::common::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// camt.054 Document wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Document")]
pub struct Camt054Document {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,

    #[serde(rename = "BkToCstmrDbtCdtNtfctn")]
    pub bank_to_customer_debit_credit_notification: BankToCustomerDebitCreditNotification,
}

impl Default for Camt054Document {
    fn default() -> Self {
        Self {
            xmlns: "urn:iso:std:iso:20022:tech:xsd:camt.054.001.10".to_string(),
            bank_to_customer_debit_credit_notification: BankToCustomerDebitCreditNotification::default(),
        }
    }
}

/// Main camt.054 message structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BankToCustomerDebitCreditNotification {
    #[serde(rename = "GrpHdr")]
    pub group_header: GroupHeader,

    #[serde(rename = "Ntfctn")]
    pub notification: Vec<AccountNotification>,
}

/// Account Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountNotification {
    #[serde(rename = "Id")]
    pub identification: String,

    #[serde(rename = "CreDtTm")]
    pub creation_date_time: String, // ISO 8601

    #[serde(rename = "Acct")]
    pub account: AccountIdentification,

    #[serde(rename = "Ntry")]
    pub entries: Vec<ReportEntry>,
}

/// Report Entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEntry {
    #[serde(rename = "NtryRef", skip_serializing_if = "Option::is_none")]
    pub entry_reference: Option<String>,

    #[serde(rename = "Amt")]
    pub amount: ActiveOrHistoricCurrencyAndAmount,

    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: CreditDebitCode,

    #[serde(rename = "Sts")]
    pub status: EntryStatus,

    #[serde(rename = "BookgDt", skip_serializing_if = "Option::is_none")]
    pub booking_date: Option<DateAndDateTime>,

    #[serde(rename = "ValDt", skip_serializing_if = "Option::is_none")]
    pub value_date: Option<DateAndDateTime>,

    #[serde(rename = "AcctSvcrRef", skip_serializing_if = "Option::is_none")]
    pub account_servicer_reference: Option<String>,

    #[serde(rename = "BkTxCd", skip_serializing_if = "Option::is_none")]
    pub bank_transaction_code: Option<BankTransactionCode>,

    #[serde(rename = "NtryDtls", skip_serializing_if = "Option::is_none")]
    pub entry_details: Option<Vec<EntryDetails>>,

    #[serde(rename = "AddtlNtryInf", skip_serializing_if = "Option::is_none")]
    pub additional_entry_info: Option<String>,
}

/// Date and DateTime choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateAndDateTime {
    #[serde(rename = "Dt", skip_serializing_if = "Option::is_none")]
    pub date: Option<String>, // YYYY-MM-DD

    #[serde(rename = "DtTm", skip_serializing_if = "Option::is_none")]
    pub date_time: Option<String>, // ISO 8601
}

/// Bank Transaction Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionCode {
    #[serde(rename = "Domn", skip_serializing_if = "Option::is_none")]
    pub domain: Option<BankTransactionCodeDomain>,

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<ProprietaryBankTransactionCode>,
}

/// Bank Transaction Code Domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionCodeDomain {
    #[serde(rename = "Cd")]
    pub code: String, // PMNT, CASH, etc.

    #[serde(rename = "Fmly")]
    pub family: BankTransactionCodeFamily,
}

/// Bank Transaction Code Family
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionCodeFamily {
    #[serde(rename = "Cd")]
    pub code: String, // RCDT, ICDT, etc.

    #[serde(rename = "SubFmlyCd")]
    pub sub_family_code: String, // ESCT, XBCT, etc.
}

/// Proprietary Bank Transaction Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProprietaryBankTransactionCode {
    #[serde(rename = "Cd")]
    pub code: String,

    #[serde(rename = "Issr", skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
}

/// Entry Details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDetails {
    #[serde(rename = "Btch", skip_serializing_if = "Option::is_none")]
    pub batch: Option<BatchInformation>,

    #[serde(rename = "TxDtls", skip_serializing_if = "Option::is_none")]
    pub transaction_details: Option<Vec<TransactionDetails>>,
}

/// Batch Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchInformation {
    #[serde(rename = "MsgId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    #[serde(rename = "PmtInfId", skip_serializing_if = "Option::is_none")]
    pub payment_information_id: Option<String>,

    #[serde(rename = "NbOfTxs", skip_serializing_if = "Option::is_none")]
    pub number_of_transactions: Option<String>,

    #[serde(rename = "TtlAmt", skip_serializing_if = "Option::is_none")]
    pub total_amount: Option<ActiveOrHistoricCurrencyAndAmount>,
}

/// Transaction Details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDetails {
    #[serde(rename = "Refs", skip_serializing_if = "Option::is_none")]
    pub references: Option<TransactionReferences>,

    #[serde(rename = "Amt", skip_serializing_if = "Option::is_none")]
    pub amount: Option<ActiveOrHistoricCurrencyAndAmount>,

    #[serde(rename = "CdtDbtInd", skip_serializing_if = "Option::is_none")]
    pub credit_debit_indicator: Option<CreditDebitCode>,

    #[serde(rename = "RltdPties", skip_serializing_if = "Option::is_none")]
    pub related_parties: Option<RelatedParties>,

    #[serde(rename = "RltdAgts", skip_serializing_if = "Option::is_none")]
    pub related_agents: Option<RelatedAgents>,

    #[serde(rename = "Purp", skip_serializing_if = "Option::is_none")]
    pub purpose: Option<Purpose>,

    #[serde(rename = "RmtInf", skip_serializing_if = "Option::is_none")]
    pub remittance_information: Option<RemittanceInformation>,
}

/// Transaction References
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionReferences {
    #[serde(rename = "MsgId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    #[serde(rename = "AcctSvcrRef", skip_serializing_if = "Option::is_none")]
    pub account_servicer_reference: Option<String>,

    #[serde(rename = "PmtInfId", skip_serializing_if = "Option::is_none")]
    pub payment_information_id: Option<String>,

    #[serde(rename = "InstrId", skip_serializing_if = "Option::is_none")]
    pub instruction_id: Option<String>,

    #[serde(rename = "EndToEndId", skip_serializing_if = "Option::is_none")]
    pub end_to_end_id: Option<String>,

    #[serde(rename = "TxId", skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,

    #[serde(rename = "UETR", skip_serializing_if = "Option::is_none")]
    pub uetr: Option<String>,
}

/// Related Parties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedParties {
    #[serde(rename = "Dbtr", skip_serializing_if = "Option::is_none")]
    pub debtor: Option<PartyIdentification>,

    #[serde(rename = "DbtrAcct", skip_serializing_if = "Option::is_none")]
    pub debtor_account: Option<AccountIdentification>,

    #[serde(rename = "Cdtr", skip_serializing_if = "Option::is_none")]
    pub creditor: Option<PartyIdentification>,

    #[serde(rename = "CdtrAcct", skip_serializing_if = "Option::is_none")]
    pub creditor_account: Option<AccountIdentification>,
}

/// Related Agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedAgents {
    #[serde(rename = "DbtrAgt", skip_serializing_if = "Option::is_none")]
    pub debtor_agent: Option<Agent>,

    #[serde(rename = "CdtrAgt", skip_serializing_if = "Option::is_none")]
    pub creditor_agent: Option<Agent>,
}

/// Parse camt.054 from XML string
pub fn parse_camt054(xml: &str) -> Result<Camt054Document, quick_xml::DeError> {
    quick_xml::de::from_str(xml)
}

/// Extract funding information from camt.054
pub fn extract_funding_info(doc: &Camt054Document) -> Vec<FundingInfo> {
    let mut funding_info = Vec::new();

    for notification in &doc.bank_to_customer_debit_credit_notification.notification {
        for entry in &notification.entries {
            // Only process credits (money in)
            if entry.credit_debit_indicator == CreditDebitCode::CRDT {
                // Only process booked entries
                if let Some(ref status) = entry.status.code {
                    if status == "BOOK" {
                        if let Ok(amount) = entry.amount.to_decimal() {
                            let uetr = entry.entry_details.as_ref()
                                .and_then(|details| details.first())
                                .and_then(|detail| detail.transaction_details.as_ref())
                                .and_then(|txns| txns.first())
                                .and_then(|txn| txn.references.as_ref())
                                .and_then(|refs| refs.uetr.clone());

                            funding_info.push(FundingInfo {
                                amount,
                                currency: entry.amount.currency.clone(),
                                uetr,
                                account_servicer_ref: entry.account_servicer_reference.clone(),
                                booking_date: entry.booking_date.as_ref()
                                    .and_then(|d| d.date_time.clone()),
                            });
                        }
                    }
                }
            }
        }
    }

    funding_info
}

/// Extracted funding information
#[derive(Debug, Clone)]
pub struct FundingInfo {
    pub amount: Decimal,
    pub currency: String,
    pub uetr: Option<String>,
    pub account_servicer_ref: Option<String>,
    pub booking_date: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credit_debit_indicator() {
        assert_eq!(CreditDebitCode::CRDT, CreditDebitCode::CRDT);
        assert_ne!(CreditDebitCode::CRDT, CreditDebitCode::DBIT);
    }

    #[test]
    fn test_camt054_default() {
        let doc = Camt054Document::default();
        assert!(doc.xmlns.contains("camt.054"));
    }
}
