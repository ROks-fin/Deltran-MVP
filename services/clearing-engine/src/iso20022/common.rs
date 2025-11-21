// ISO 20022 Common Types and Structures

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Party Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyIdentification {
    #[serde(rename = "Nm", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(rename = "PstlAdr", skip_serializing_if = "Option::is_none")]
    pub postal_address: Option<PostalAddress>,

    #[serde(rename = "Id", skip_serializing_if = "Option::is_none")]
    pub identification: Option<Party>,

    #[serde(rename = "CtryOfRes", skip_serializing_if = "Option::is_none")]
    pub country_of_residence: Option<String>,
}

/// Postal Address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostalAddress {
    #[serde(rename = "Ctry", skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    #[serde(rename = "AdrLine", skip_serializing_if = "Option::is_none")]
    pub address_line: Option<Vec<String>>,
}

/// Party (Organization or Person)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    #[serde(rename = "OrgId", skip_serializing_if = "Option::is_none")]
    pub organization_identification: Option<OrganizationIdentification>,

    #[serde(rename = "PrvtId", skip_serializing_if = "Option::is_none")]
    pub private_identification: Option<PersonIdentification>,
}

/// Organization Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationIdentification {
    #[serde(rename = "BICOrBEI", skip_serializing_if = "Option::is_none")]
    pub bic_or_bei: Option<String>,

    #[serde(rename = "Othr", skip_serializing_if = "Option::is_none")]
    pub other: Option<Vec<GenericIdentification>>,
}

/// Person Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonIdentification {
    #[serde(rename = "DtAndPlcOfBirth", skip_serializing_if = "Option::is_none")]
    pub date_and_place_of_birth: Option<DateAndPlaceOfBirth>,

    #[serde(rename = "Othr", skip_serializing_if = "Option::is_none")]
    pub other: Option<Vec<GenericIdentification>>,
}

/// Date and Place of Birth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateAndPlaceOfBirth {
    #[serde(rename = "BirthDt")]
    pub birth_date: String,

    #[serde(rename = "CityOfBirth")]
    pub city_of_birth: String,

    #[serde(rename = "CtryOfBirth")]
    pub country_of_birth: String,
}

/// Generic Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericIdentification {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "SchmeNm", skip_serializing_if = "Option::is_none")]
    pub scheme_name: Option<SchemeName>,

    #[serde(rename = "Issr", skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
}

/// Scheme Name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeName {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Agent (Financial Institution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    #[serde(rename = "FinInstnId")]
    pub financial_institution_id: FinancialInstitutionIdentification,

    #[serde(rename = "BrnchId", skip_serializing_if = "Option::is_none")]
    pub branch_identification: Option<BranchIdentification>,
}

impl Agent {
    // Alias for backward compatibility
    pub fn financial_institution_identification(&self) -> &FinancialInstitutionIdentification {
        &self.financial_institution_id
    }
}

/// Branch Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchIdentification {
    #[serde(rename = "Id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(rename = "Nm", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Financial Institution Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialInstitutionIdentification {
    #[serde(rename = "BICFI", skip_serializing_if = "Option::is_none")]
    pub bic: Option<String>,

    #[serde(rename = "ClrSysMmbId", skip_serializing_if = "Option::is_none")]
    pub clearing_system_member_id: Option<ClearingSystemMemberIdentification>,

    #[serde(rename = "Nm", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(rename = "PstlAdr", skip_serializing_if = "Option::is_none")]
    pub postal_address: Option<PostalAddress>,

    #[serde(rename = "Othr", skip_serializing_if = "Option::is_none")]
    pub other: Option<GenericIdentification>,
}

/// Clearing System Member Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearingSystemMemberIdentification {
    #[serde(rename = "ClrSysId", skip_serializing_if = "Option::is_none")]
    pub clearing_system_id: Option<ClearingSystemIdentification>,

    #[serde(rename = "MmbId")]
    pub member_id: String,
}

/// Clearing System Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearingSystemIdentification {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Account Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountIdentification {
    #[serde(rename = "Id")]
    pub identification: AccountId,

    #[serde(rename = "Tp", skip_serializing_if = "Option::is_none")]
    pub account_type: Option<AccountType>,

    #[serde(rename = "Ccy", skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,

    #[serde(rename = "Nm", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Account ID
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AccountId {
    IBAN(String),
    Other(GenericAccountIdentification),
}

/// Generic Account Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericAccountIdentification {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "SchmeNm", skip_serializing_if = "Option::is_none")]
    pub scheme_name: Option<SchemeName>,
}

/// Account Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountType {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Active or Historic Currency and Amount
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveOrHistoricCurrencyAndAmount {
    #[serde(rename = "@Ccy")]
    pub currency: String,

    #[serde(rename = "$text")]
    pub amount: String, // Will be converted to Decimal
}

impl ActiveOrHistoricCurrencyAndAmount {
    pub fn to_decimal(&self) -> Result<Decimal, rust_decimal::Error> {
        self.amount.parse::<Decimal>()
    }

    pub fn from_decimal(currency: String, amount: Decimal) -> Self {
        Self {
            currency,
            amount: amount.to_string(),
        }
    }
}

/// Group Header
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroupHeader {
    #[serde(rename = "MsgId")]
    pub message_id: String,

    #[serde(rename = "CreDtTm")]
    pub creation_date_time: String,

    #[serde(rename = "NbOfTxs")]
    pub number_of_transactions: String,

    #[serde(rename = "CtrlSum", skip_serializing_if = "Option::is_none")]
    pub control_sum: Option<String>,

    #[serde(rename = "InitgPty", skip_serializing_if = "Option::is_none")]
    pub initiating_party: Option<PartyIdentification>,

    #[serde(rename = "FwdgAgt", skip_serializing_if = "Option::is_none")]
    pub forwarding_agent: Option<Agent>,
}

/// Payment Identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIdentification {
    #[serde(rename = "InstrId", skip_serializing_if = "Option::is_none")]
    pub instruction_id: Option<String>,

    #[serde(rename = "EndToEndId")]
    pub end_to_end_id: String,

    #[serde(rename = "TxId", skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,

    #[serde(rename = "UETR", skip_serializing_if = "Option::is_none")]
    pub uetr: Option<String>, // Unique End-to-end Transaction Reference
}

/// Purpose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Purpose {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Remittance Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceInformation {
    #[serde(rename = "Ustrd", skip_serializing_if = "Option::is_none")]
    pub unstructured: Option<Vec<String>>,

    #[serde(rename = "Strd", skip_serializing_if = "Option::is_none")]
    pub structured: Option<Vec<StructuredRemittanceInformation>>,
}

/// Structured Remittance Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredRemittanceInformation {
    #[serde(rename = "RfrdDocInf", skip_serializing_if = "Option::is_none")]
    pub referred_document_info: Option<Vec<ReferredDocumentInformation>>,
}

/// Referred Document Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferredDocumentInformation {
    #[serde(rename = "Tp", skip_serializing_if = "Option::is_none")]
    pub document_type: Option<DocumentType>,

    #[serde(rename = "Nb", skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,

    #[serde(rename = "RltdDt", skip_serializing_if = "Option::is_none")]
    pub related_date: Option<String>,
}

/// Document Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentType {
    #[serde(rename = "CdOrPrtry")]
    pub code_or_proprietary: CodeOrProprietary,
}

/// Code or Proprietary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeOrProprietary {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Credit/Debit Code - used across multiple message types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CreditDebitCode {
    CRDT, // Credit (money in)
    DBIT, // Debit (money out)
}

/// Entry Status - used in camt.053 and camt.054
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryStatus {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>, // BOOK, PDNG, INFO

    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Charge Bearer Type - who pays the charges
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChargeBearerType {
    DEBT, // Debtor bears all charges
    CRED, // Creditor bears all charges
    SHAR, // Shared
    SLEV, // Service level
}
