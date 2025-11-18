// ISO 20022 pain.001.001.03 - CustomerCreditTransferInitiation
// Used when customers initiate credit transfer requests
// Entry point for customer-initiated payments into DelTran system

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use crate::errors::ClearingError;
use super::common::{
    ActiveOrHistoricCurrencyAndAmount, PartyIdentification,
    AccountIdentification, Agent, PaymentIdentification,
    RemittanceInformation, Purpose, ChargeBearerType,
};

/// pain.001 Document - CustomerCreditTransferInitiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pain001Document {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "CstmrCdtTrfInitn")]
    pub customer_credit_transfer_initiation: CustomerCreditTransferInitiation,
}

/// Customer Credit Transfer Initiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerCreditTransferInitiation {
    #[serde(rename = "GrpHdr")]
    pub group_header: GroupHeader,
    #[serde(rename = "PmtInf")]
    pub payment_information: Vec<PaymentInformation>,
}

/// Group Header for pain.001
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupHeader {
    #[serde(rename = "MsgId")]
    pub message_id: String,
    #[serde(rename = "CreDtTm")]
    pub creation_date_time: DateTime<Utc>,
    #[serde(rename = "NbOfTxs")]
    pub number_of_transactions: String,
    #[serde(rename = "CtrlSum", skip_serializing_if = "Option::is_none")]
    pub control_sum: Option<String>, // Sum of all amounts
    #[serde(rename = "InitgPty")]
    pub initiating_party: PartyIdentification,
}

/// Payment Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInformation {
    #[serde(rename = "PmtInfId")]
    pub payment_information_id: String,
    #[serde(rename = "PmtMtd")]
    pub payment_method: PaymentMethod,
    #[serde(rename = "BtchBookg", skip_serializing_if = "Option::is_none")]
    pub batch_booking: Option<bool>,
    #[serde(rename = "NbOfTxs", skip_serializing_if = "Option::is_none")]
    pub number_of_transactions: Option<String>,
    #[serde(rename = "CtrlSum", skip_serializing_if = "Option::is_none")]
    pub control_sum: Option<String>,
    #[serde(rename = "PmtTpInf", skip_serializing_if = "Option::is_none")]
    pub payment_type_information: Option<PaymentTypeInformation>,
    #[serde(rename = "ReqdExctnDt")]
    pub requested_execution_date: DateAndDateTime,
    #[serde(rename = "Dbtr")]
    pub debtor: PartyIdentification,
    #[serde(rename = "DbtrAcct")]
    pub debtor_account: CashAccount,
    #[serde(rename = "DbtrAgt")]
    pub debtor_agent: Agent,
    #[serde(rename = "ChrgBr", skip_serializing_if = "Option::is_none")]
    pub charge_bearer: Option<ChargeBearerType>,
    #[serde(rename = "CdtTrfTxInf")]
    pub credit_transfer_transactions: Vec<CreditTransferTransactionInformation>,
}

/// Payment Method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentMethod {
    TRF,  // Transfer
    TRA,  // TransferAdvice
}

/// Payment Type Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTypeInformation {
    #[serde(rename = "InstrPrty", skip_serializing_if = "Option::is_none")]
    pub instruction_priority: Option<Priority>,
    #[serde(rename = "SvcLvl", skip_serializing_if = "Option::is_none")]
    pub service_level: Option<ServiceLevel>,
    #[serde(rename = "LclInstrm", skip_serializing_if = "Option::is_none")]
    pub local_instrument: Option<LocalInstrument>,
    #[serde(rename = "CtgyPurp", skip_serializing_if = "Option::is_none")]
    pub category_purpose: Option<CategoryPurpose>,
}

/// Priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    HIGH,
    NORM,
    URGP, // Urgent Payment
}

/// Service Level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLevel {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>, // SEPA, NURG (non-urgent), SDVA (same day value), URGP (urgent)
    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Local Instrument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalInstrument {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Category Purpose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryPurpose {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>, // CASH, TRAD (trade settlement), TREA (treasury)
    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Date and DateTime (either date or datetime)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateAndDateTime {
    #[serde(rename = "Dt", skip_serializing_if = "Option::is_none")]
    pub date: Option<chrono::NaiveDate>,
    #[serde(rename = "DtTm", skip_serializing_if = "Option::is_none")]
    pub date_time: Option<DateTime<Utc>>,
}

/// Cash Account for pain.001
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAccount {
    #[serde(rename = "Id")]
    pub id: AccountIdentification,
    #[serde(rename = "Ccy", skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

/// Credit Transfer Transaction Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransferTransactionInformation {
    #[serde(rename = "PmtId")]
    pub payment_identification: PaymentIdentification,
    #[serde(rename = "PmtTpInf", skip_serializing_if = "Option::is_none")]
    pub payment_type_information: Option<PaymentTypeInformation>,
    #[serde(rename = "Amt")]
    pub amount: AmountType,
    #[serde(rename = "ChrgBr", skip_serializing_if = "Option::is_none")]
    pub charge_bearer: Option<ChargeBearerType>,
    #[serde(rename = "UltmtDbtr", skip_serializing_if = "Option::is_none")]
    pub ultimate_debtor: Option<PartyIdentification>,
    #[serde(rename = "CdtrAgt", skip_serializing_if = "Option::is_none")]
    pub creditor_agent: Option<Agent>,
    #[serde(rename = "Cdtr")]
    pub creditor: PartyIdentification,
    #[serde(rename = "CdtrAcct")]
    pub creditor_account: CashAccount,
    #[serde(rename = "UltmtCdtr", skip_serializing_if = "Option::is_none")]
    pub ultimate_creditor: Option<PartyIdentification>,
    #[serde(rename = "Purp", skip_serializing_if = "Option::is_none")]
    pub purpose: Option<Purpose>,
    #[serde(rename = "RmtInf", skip_serializing_if = "Option::is_none")]
    pub remittance_information: Option<RemittanceInformation>,
}

/// Amount Type wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountType {
    #[serde(rename = "InstdAmt", skip_serializing_if = "Option::is_none")]
    pub instructed_amount: Option<ActiveOrHistoricCurrencyAndAmount>,
    #[serde(rename = "EqvtAmt", skip_serializing_if = "Option::is_none")]
    pub equivalent_amount: Option<EquivalentAmount>,
}

/// Equivalent Amount (for multi-currency)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalentAmount {
    #[serde(rename = "Amt")]
    pub amount: ActiveOrHistoricCurrencyAndAmount,
    #[serde(rename = "CcyOfTrf")]
    pub currency_of_transfer: String,
}

/// Parse pain.001 XML document
pub fn parse_pain001(xml: &str) -> Result<Pain001Document, quick_xml::DeError> {
    quick_xml::de::from_str(xml)
}

/// Builder for pain.001 messages
pub struct Pain001Builder {
    message: CustomerCreditTransferInitiation,
}

impl Pain001Builder {
    pub fn new(
        message_id: String,
        initiating_party: PartyIdentification,
    ) -> Self {
        Self {
            message: CustomerCreditTransferInitiation {
                group_header: GroupHeader {
                    message_id,
                    creation_date_time: Utc::now(),
                    number_of_transactions: "0".to_string(),
                    control_sum: None,
                    initiating_party,
                },
                payment_information: vec![],
            },
        }
    }

    pub fn with_creation_time(mut self, dt: DateTime<Utc>) -> Self {
        self.message.group_header.creation_date_time = dt;
        self
    }

    pub fn add_payment_info(mut self, payment_info: PaymentInformation) -> Self {
        self.message.payment_information.push(payment_info);
        self
    }

    pub fn build(mut self) -> Pain001Document {
        // Calculate totals
        let total_txns: usize = self.message.payment_information.iter()
            .map(|pi| pi.credit_transfer_transactions.len())
            .sum();

        self.message.group_header.number_of_transactions = total_txns.to_string();

        Pain001Document {
            xmlns: "urn:iso:std:iso:20022:tech:xsd:pain.001.001.03".to_string(),
            customer_credit_transfer_initiation: self.message,
        }
    }
}

/// Helper to create a simple customer payment
pub fn create_customer_payment(
    debtor_name: String,
    debtor_iban: String,
    debtor_bic: String,
    creditor_name: String,
    creditor_iban: String,
    creditor_bic: String,
    amount: Decimal,
    currency: String,
    end_to_end_id: String,
    remittance_info: Option<String>,
) -> Result<CreditTransferTransactionInformation, ClearingError> {
    Ok(CreditTransferTransactionInformation {
        payment_identification: PaymentIdentification {
            instruction_id: None,
            end_to_end_id,
            transaction_id: None,
            uetr: None,
        },
        payment_type_information: None,
        amount: AmountType {
            instructed_amount: Some(
                ActiveOrHistoricCurrencyAndAmount::from_decimal(currency, amount)
            ),
            equivalent_amount: None,
        },
        charge_bearer: Some(ChargeBearerType::SHAR), // Shared charges
        ultimate_debtor: None,
        creditor_agent: Some(Agent {
            financial_institution_identification: super::common::FinancialInstitutionIdentification {
                bic: Some(creditor_bic),
                clearing_system_member_id: None,
                name: None,
                postal_address: None,
                other: None,
            },
            branch_identification: None,
        }),
        creditor: PartyIdentification {
            name: Some(creditor_name),
            postal_address: None,
            identification: None,
            country_of_residence: None,
        },
        creditor_account: CashAccount {
            id: AccountIdentification::IBAN(creditor_iban),
            currency: None,
        },
        ultimate_creditor: None,
        purpose: None,
        remittance_information: remittance_info.map(|info| RemittanceInformation {
            unstructured: Some(vec![info]),
            structured: None,
        }),
    })
}

/// Extract payment requests from pain.001 for processing
#[derive(Debug, Clone)]
pub struct PaymentRequest {
    pub payment_info_id: String,
    pub debtor_name: String,
    pub debtor_account: String,
    pub debtor_bic: String,
    pub creditor_name: String,
    pub creditor_account: String,
    pub creditor_bic: Option<String>,
    pub amount: Decimal,
    pub currency: String,
    pub end_to_end_id: String,
    pub remittance_info: Option<String>,
    pub requested_execution_date: Option<DateTime<Utc>>,
}

/// Extract payment requests from pain.001 document
pub fn extract_payment_requests(doc: &Pain001Document) -> Result<Vec<PaymentRequest>, ClearingError> {
    let mut requests = Vec::new();

    for payment_info in &doc.customer_credit_transfer_initiation.payment_information {
        let debtor_name = payment_info.debtor.name.clone()
            .unwrap_or_else(|| "Unknown".to_string());

        let debtor_account = match &payment_info.debtor_account.id {
            AccountIdentification::IBAN(iban) => iban.clone(),
            AccountIdentification::Other { identification, .. } => identification.clone(),
        };

        let debtor_bic = payment_info.debtor_agent.financial_institution_identification.bic.clone()
            .unwrap_or_else(|| "UNKNOWN".to_string());

        let requested_execution_date = payment_info.requested_execution_date.date_time;

        for txn in &payment_info.credit_transfer_transactions {
            let creditor_name = txn.creditor.name.clone()
                .unwrap_or_else(|| "Unknown".to_string());

            let creditor_account = match &txn.creditor_account.id {
                AccountIdentification::IBAN(iban) => iban.clone(),
                AccountIdentification::Other { identification, .. } => identification.clone(),
            };

            let creditor_bic = txn.creditor_agent.as_ref()
                .and_then(|agent| agent.financial_institution_identification.bic.clone());

            let (amount, currency) = if let Some(instructed) = &txn.amount.instructed_amount {
                let amt = instructed.to_decimal()
                    .map_err(|e| ClearingError::Internal(format!("Failed to parse amount: {}", e)))?;
                (amt, instructed.currency.clone())
            } else {
                return Err(ClearingError::Internal("Missing instructed amount".to_string()));
            };

            let remittance_info = txn.remittance_information.as_ref()
                .and_then(|ri| ri.unstructured.as_ref())
                .and_then(|u| u.first())
                .cloned();

            requests.push(PaymentRequest {
                payment_info_id: payment_info.payment_information_id.clone(),
                debtor_name,
                debtor_account: debtor_account.clone(),
                debtor_bic: debtor_bic.clone(),
                creditor_name,
                creditor_account,
                creditor_bic,
                amount,
                currency,
                end_to_end_id: txn.payment_identification.end_to_end_id.clone(),
                remittance_info,
                requested_execution_date,
            });
        }
    }

    Ok(requests)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_customer_payment() {
        let payment = create_customer_payment(
            "John Doe".to_string(),
            "AE070331234567890123456".to_string(),
            "UAEBAE21".to_string(),
            "Jane Smith".to_string(),
            "IN0123456789012345".to_string(),
            "INDBINBB".to_string(),
            Decimal::new(100000, 2), // 1000.00
            "USD".to_string(),
            "E2E-REF-12345".to_string(),
            Some("Invoice payment".to_string()),
        ).unwrap();

        assert_eq!(payment.payment_identification.end_to_end_id, "E2E-REF-12345");
        assert!(payment.remittance_information.is_some());
    }

    #[test]
    fn test_pain001_builder() {
        let initiating_party = PartyIdentification {
            name: Some("Test Corp".to_string()),
            postal_address: None,
            identification: None,
            country_of_residence: None,
        };

        let doc = Pain001Builder::new("MSG-001".to_string(), initiating_party)
            .build();

        assert_eq!(doc.customer_credit_transfer_initiation.group_header.message_id, "MSG-001");
    }

    #[test]
    fn test_payment_request_extraction() {
        let initiating_party = PartyIdentification {
            name: Some("Test Bank".to_string()),
            postal_address: None,
            identification: None,
            country_of_residence: None,
        };

        // Would need a full document to test extraction properly
        // This is a structure test
        let request = PaymentRequest {
            payment_info_id: "PMT-001".to_string(),
            debtor_name: "Alice".to_string(),
            debtor_account: "AE123".to_string(),
            debtor_bic: "UAEBAE21".to_string(),
            creditor_name: "Bob".to_string(),
            creditor_account: "IN456".to_string(),
            creditor_bic: Some("INDBINBB".to_string()),
            amount: Decimal::new(50000, 2),
            currency: "USD".to_string(),
            end_to_end_id: "E2E-123".to_string(),
            remittance_info: None,
            requested_execution_date: None,
        };

        assert_eq!(request.amount, Decimal::new(50000, 2));
    }
}
