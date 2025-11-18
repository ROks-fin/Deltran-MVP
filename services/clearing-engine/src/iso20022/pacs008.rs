// pacs.008.001.10 - FIToFICustomerCreditTransfer
// Core message for inter-bank settlement operations

use super::common::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// pacs.008 Document wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Document")]
pub struct Pacs008Document {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,

    #[serde(rename = "FIToFICstmrCdtTrf")]
    pub fi_to_fi_customer_credit_transfer: FIToFICustomerCreditTransfer,
}

impl Default for Pacs008Document {
    fn default() -> Self {
        Self {
            xmlns: "urn:iso:std:iso:20022:tech:xsd:pacs.008.001.10".to_string(),
            fi_to_fi_customer_credit_transfer: FIToFICustomerCreditTransfer::default(),
        }
    }
}

/// Main pacs.008 message structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FIToFICustomerCreditTransfer {
    #[serde(rename = "GrpHdr")]
    pub group_header: GroupHeader,

    #[serde(rename = "CdtTrfTxInf")]
    pub credit_transfer_transaction_information: Vec<CreditTransferTransaction>,
}

/// Credit Transfer Transaction Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransferTransaction {
    #[serde(rename = "PmtId")]
    pub payment_identification: PaymentIdentification,

    #[serde(rename = "PmtTpInf", skip_serializing_if = "Option::is_none")]
    pub payment_type_information: Option<PaymentTypeInformation>,

    #[serde(rename = "IntrBkSttlmAmt")]
    pub interbank_settlement_amount: ActiveOrHistoricCurrencyAndAmount,

    #[serde(rename = "IntrBkSttlmDt", skip_serializing_if = "Option::is_none")]
    pub interbank_settlement_date: Option<String>, // YYYY-MM-DD

    #[serde(rename = "SttlmTmIndctn", skip_serializing_if = "Option::is_none")]
    pub settlement_time_indication: Option<SettlementTimeIndication>,

    #[serde(rename = "InstdAmt", skip_serializing_if = "Option::is_none")]
    pub instructed_amount: Option<ActiveOrHistoricCurrencyAndAmount>,

    #[serde(rename = "XchgRate", skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<String>, // Decimal as string

    #[serde(rename = "ChrgBr")]
    pub charge_bearer: ChargeBearerType,

    #[serde(rename = "ChrgsInf", skip_serializing_if = "Option::is_none")]
    pub charges_information: Option<Vec<ChargesInformation>>,

    #[serde(rename = "PrvsInstgAgt1", skip_serializing_if = "Option::is_none")]
    pub previous_instructing_agent_1: Option<Agent>,

    #[serde(rename = "InstgAgt", skip_serializing_if = "Option::is_none")]
    pub instructing_agent: Option<Agent>,

    #[serde(rename = "InstdAgt", skip_serializing_if = "Option::is_none")]
    pub instructed_agent: Option<Agent>,

    #[serde(rename = "IntrmyAgt1", skip_serializing_if = "Option::is_none")]
    pub intermediary_agent_1: Option<Agent>,

    #[serde(rename = "IntrmyAgt2", skip_serializing_if = "Option::is_none")]
    pub intermediary_agent_2: Option<Agent>,

    #[serde(rename = "CdtrAgt", skip_serializing_if = "Option::is_none")]
    pub creditor_agent: Option<Agent>,

    #[serde(rename = "Cdtr")]
    pub creditor: PartyIdentification,

    #[serde(rename = "CdtrAcct", skip_serializing_if = "Option::is_none")]
    pub creditor_account: Option<AccountIdentification>,

    #[serde(rename = "UltmtCdtr", skip_serializing_if = "Option::is_none")]
    pub ultimate_creditor: Option<PartyIdentification>,

    #[serde(rename = "DbtrAgt", skip_serializing_if = "Option::is_none")]
    pub debtor_agent: Option<Agent>,

    #[serde(rename = "Dbtr")]
    pub debtor: PartyIdentification,

    #[serde(rename = "DbtrAcct", skip_serializing_if = "Option::is_none")]
    pub debtor_account: Option<AccountIdentification>,

    #[serde(rename = "UltmtDbtr", skip_serializing_if = "Option::is_none")]
    pub ultimate_debtor: Option<PartyIdentification>,

    #[serde(rename = "Purp", skip_serializing_if = "Option::is_none")]
    pub purpose: Option<Purpose>,

    #[serde(rename = "RmtInf", skip_serializing_if = "Option::is_none")]
    pub remittance_information: Option<RemittanceInformation>,

    #[serde(rename = "SplmtryData", skip_serializing_if = "Option::is_none")]
    pub supplementary_data: Option<Vec<SupplementaryData>>,
}

/// Payment Type Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentTypeInformation {
    #[serde(rename = "InstrPrty", skip_serializing_if = "Option::is_none")]
    pub instruction_priority: Option<String>, // HIGH, NORM

    #[serde(rename = "SvcLvl", skip_serializing_if = "Option::is_none")]
    pub service_level: Option<ServiceLevel>,

    #[serde(rename = "LclInstrm", skip_serializing_if = "Option::is_none")]
    pub local_instrument: Option<LocalInstrument>,

    #[serde(rename = "CtgyPurp", skip_serializing_if = "Option::is_none")]
    pub category_purpose: Option<CategoryPurpose>,
}

/// Service Level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLevel {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>, // SEPA, URGP, etc.

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
    pub code: Option<String>, // CASH, TRAD, etc.

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Settlement Time Indication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementTimeIndication {
    #[serde(rename = "DbtDtTm", skip_serializing_if = "Option::is_none")]
    pub debit_date_time: Option<String>, // ISO 8601

    #[serde(rename = "CdtDtTm", skip_serializing_if = "Option::is_none")]
    pub credit_date_time: Option<String>, // ISO 8601
}

/// Charges Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargesInformation {
    #[serde(rename = "Amt")]
    pub amount: ActiveOrHistoricCurrencyAndAmount,

    #[serde(rename = "Agt")]
    pub agent: Agent,

    #[serde(rename = "Tp", skip_serializing_if = "Option::is_none")]
    pub charge_type: Option<ChargeType>,
}

/// Charge Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargeType {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Supplementary Data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplementaryData {
    #[serde(rename = "PlcAndNm", skip_serializing_if = "Option::is_none")]
    pub place_and_name: Option<String>,

    #[serde(rename = "Envlp")]
    pub envelope: SupplementaryDataEnvelope,
}

/// Supplementary Data Envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplementaryDataEnvelope {
    #[serde(rename = "$value")]
    pub content: String, // Can contain any XML
}

/// Builder for pacs.008 messages
pub struct Pacs008Builder {
    message: FIToFICustomerCreditTransfer,
}

impl Pacs008Builder {
    pub fn new() -> Self {
        Self {
            message: FIToFICustomerCreditTransfer::default(),
        }
    }

    /// Set group header
    pub fn with_group_header(
        mut self,
        message_id: String,
        created_at: DateTime<Utc>,
        num_transactions: usize,
    ) -> Self {
        self.message.group_header = GroupHeader {
            message_id,
            creation_date_time: created_at.to_rfc3339(),
            number_of_transactions: num_transactions.to_string(),
            control_sum: None,
            initiating_party: None,
            forwarding_agent: None,
        };
        self
    }

    /// Add credit transfer transaction
    pub fn add_transaction(mut self, transaction: CreditTransferTransaction) -> Self {
        self.message.credit_transfer_transaction_information.push(transaction);
        self
    }

    /// Build the complete document
    pub fn build(self) -> Pacs008Document {
        Pacs008Document {
            xmlns: "urn:iso:std:iso:20022:tech:xsd:pacs.008.001.10".to_string(),
            fi_to_fi_customer_credit_transfer: self.message,
        }
    }
}

impl Default for Pacs008Builder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a basic settlement transaction
pub fn create_settlement_transaction(
    uetr: String,
    amount: Decimal,
    currency: String,
    debtor_bic: String,
    creditor_bic: String,
    debtor_name: String,
    creditor_name: String,
) -> CreditTransferTransaction {
    CreditTransferTransaction {
        payment_identification: PaymentIdentification {
            instruction_id: Some(Uuid::new_v4().to_string()),
            end_to_end_id: Uuid::new_v4().to_string(),
            transaction_id: Some(Uuid::new_v4().to_string()),
            uetr: Some(uetr),
        },
        payment_type_information: Some(PaymentTypeInformation {
            instruction_priority: Some("NORM".to_string()),
            service_level: None,
            local_instrument: None,
            category_purpose: Some(CategoryPurpose {
                code: Some("CASH".to_string()),
                proprietary: None,
            }),
        }),
        interbank_settlement_amount: ActiveOrHistoricCurrencyAndAmount::from_decimal(
            currency.clone(),
            amount,
        ),
        interbank_settlement_date: Some(Utc::now().format("%Y-%m-%d").to_string()),
        settlement_time_indication: None,
        instructed_amount: None,
        exchange_rate: None,
        charge_bearer: ChargeBearerType::SHAR,
        charges_information: None,
        previous_instructing_agent_1: None,
        instructing_agent: None,
        instructed_agent: None,
        intermediary_agent_1: None,
        intermediary_agent_2: None,
        creditor_agent: Some(Agent {
            financial_institution_id: FinancialInstitutionIdentification {
                bic: Some(creditor_bic),
                clearing_system_member_id: None,
                name: None,
                postal_address: None,
            },
        }),
        creditor: PartyIdentification {
            name: Some(creditor_name),
            postal_address: None,
            identification: None,
        },
        creditor_account: None,
        ultimate_creditor: None,
        debtor_agent: Some(Agent {
            financial_institution_id: FinancialInstitutionIdentification {
                bic: Some(debtor_bic),
                clearing_system_member_id: None,
                name: None,
                postal_address: None,
            },
        }),
        debtor: PartyIdentification {
            name: Some(debtor_name),
            postal_address: None,
            identification: None,
        },
        debtor_account: None,
        ultimate_debtor: None,
        purpose: None,
        remittance_information: None,
        supplementary_data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacs008_builder() {
        let builder = Pacs008Builder::new()
            .with_group_header(
                "MSG001".to_string(),
                Utc::now(),
                1,
            );

        let doc = builder.build();
        assert_eq!(doc.fi_to_fi_customer_credit_transfer.group_header.message_id, "MSG001");
    }

    #[test]
    fn test_create_settlement_transaction() {
        let txn = create_settlement_transaction(
            "UETR123".to_string(),
            Decimal::from(1000),
            "USD".to_string(),
            "BANK1XXX".to_string(),
            "BANK2XXX".to_string(),
            "Debtor Bank".to_string(),
            "Creditor Bank".to_string(),
        );

        assert_eq!(txn.payment_identification.uetr, Some("UETR123".to_string()));
        assert_eq!(txn.interbank_settlement_amount.currency, "USD");
    }

    #[test]
    fn test_xml_serialization() {
        let doc = Pacs008Builder::new()
            .with_group_header("TEST001".to_string(), Utc::now(), 0)
            .build();

        let xml = quick_xml::se::to_string(&doc);
        assert!(xml.is_ok());
    }
}
