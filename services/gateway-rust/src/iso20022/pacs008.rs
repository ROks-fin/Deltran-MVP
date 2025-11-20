// pacs.008 - FI to FI Customer Credit Transfer
// This is used for interbank settlement instructions

use serde::{Deserialize, Serialize};
use quick_xml::de::from_str;
use anyhow::{Result, Context};
use uuid::Uuid;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::models::canonical::{
    CanonicalPayment, PaymentStatus, Party, FinancialInstitution,
    AccountIdentification, Currency, PostalAddress, AccountType, ChargeBearer,
    ComplianceStatus, Priority,
};

// pacs.008.001.10 - FIToFICustomerCreditTransfer
#[derive(Debug, Deserialize, Serialize)]
pub struct Document {
    #[serde(rename = "FIToFICstmrCdtTrf")]
    pub fi_to_fi_customer_credit_transfer: FIToFICustomerCreditTransfer,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FIToFICustomerCreditTransfer {
    #[serde(rename = "GrpHdr")]
    pub group_header: GroupHeader,

    #[serde(rename = "CdtTrfTxInf")]
    pub credit_transfer_transaction_information: Vec<CreditTransferTransaction>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupHeader {
    #[serde(rename = "MsgId")]
    pub message_id: String,

    #[serde(rename = "CreDtTm")]
    pub creation_date_time: String,

    #[serde(rename = "NbOfTxs")]
    pub number_of_transactions: String,

    #[serde(rename = "TtlIntrBkSttlmAmt", default)]
    pub total_interbank_settlement_amount: Option<ActiveCurrencyAndAmount>,

    #[serde(rename = "IntrBkSttlmDt", default)]
    pub interbank_settlement_date: Option<String>,

    #[serde(rename = "SttlmInf")]
    pub settlement_information: SettlementInformation,

    #[serde(rename = "InstgAgt", default)]
    pub instructing_agent: Option<BranchAndFinancialInstitutionIdentification>,

    #[serde(rename = "InstdAgt", default)]
    pub instructed_agent: Option<BranchAndFinancialInstitutionIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SettlementInformation {
    #[serde(rename = "SttlmMtd")]
    pub settlement_method: String,  // INDA, INGA, COVE, CLRG

    #[serde(rename = "SttlmAcct", default)]
    pub settlement_account: Option<CashAccount>,

    #[serde(rename = "ClrSys", default)]
    pub clearing_system: Option<ClearingSystemIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClearingSystemIdentification {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreditTransferTransaction {
    #[serde(rename = "PmtId")]
    pub payment_identification: PaymentIdentification,

    #[serde(rename = "PmtTpInf", default)]
    pub payment_type_information: Option<PaymentTypeInformation>,

    #[serde(rename = "IntrBkSttlmAmt")]
    pub interbank_settlement_amount: ActiveCurrencyAndAmount,

    #[serde(rename = "IntrBkSttlmDt", default)]
    pub interbank_settlement_date: Option<String>,

    #[serde(rename = "ChrgBr", default)]
    pub charge_bearer: Option<String>,  // DEBT, CRED, SHAR, SLEV

    #[serde(rename = "InstgAgt", default)]
    pub instructing_agent: Option<BranchAndFinancialInstitutionIdentification>,

    #[serde(rename = "InstdAgt", default)]
    pub instructed_agent: Option<BranchAndFinancialInstitutionIdentification>,

    #[serde(rename = "Dbtr")]
    pub debtor: PartyIdentification,

    #[serde(rename = "DbtrAcct", default)]
    pub debtor_account: Option<CashAccount>,

    #[serde(rename = "DbtrAgt")]
    pub debtor_agent: BranchAndFinancialInstitutionIdentification,

    #[serde(rename = "CdtrAgt")]
    pub creditor_agent: BranchAndFinancialInstitutionIdentification,

    #[serde(rename = "Cdtr")]
    pub creditor: PartyIdentification,

    #[serde(rename = "CdtrAcct", default)]
    pub creditor_account: Option<CashAccount>,

    #[serde(rename = "Purp", default)]
    pub purpose: Option<Purpose>,

    #[serde(rename = "RmtInf", default)]
    pub remittance_information: Option<RemittanceInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaymentIdentification {
    #[serde(rename = "InstrId", default)]
    pub instruction_id: Option<String>,

    #[serde(rename = "EndToEndId")]
    pub end_to_end_id: String,

    #[serde(rename = "TxId", default)]
    pub transaction_id: Option<String>,

    #[serde(rename = "UETR", default)]
    pub uetr: Option<String>,  // UUID for tracking
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PaymentTypeInformation {
    #[serde(rename = "InstrPrty", default)]
    pub instruction_priority: Option<String>,

    #[serde(rename = "SvcLvl", default)]
    pub service_level: Option<ServiceLevel>,

    #[serde(rename = "LclInstrm", default)]
    pub local_instrument: Option<LocalInstrument>,

    #[serde(rename = "CtgyPurp", default)]
    pub category_purpose: Option<CategoryPurpose>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServiceLevel {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalInstrument {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CategoryPurpose {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ActiveCurrencyAndAmount {
    #[serde(rename = "Ccy")]
    pub currency: String,

    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PartyIdentification {
    #[serde(rename = "Nm", default)]
    pub name: Option<String>,

    #[serde(rename = "PstlAdr", default)]
    pub postal_address: Option<PostalAddressType>,

    #[serde(rename = "Id", default)]
    pub identification: Option<PartyId>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostalAddressType {
    #[serde(rename = "AdrLine", default)]
    pub address_line: Vec<String>,

    #[serde(rename = "Ctry", default)]
    pub country: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PartyId {
    #[serde(rename = "OrgId", default)]
    pub organisation_id: Option<OrganisationIdentification>,

    #[serde(rename = "PrvtId", default)]
    pub private_id: Option<PersonIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OrganisationIdentification {
    #[serde(rename = "AnyBIC", default)]
    pub any_bic: Option<String>,

    #[serde(rename = "Othr", default)]
    pub other: Vec<GenericIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PersonIdentification {
    #[serde(rename = "DtAndPlcOfBirth", default)]
    pub date_and_place_of_birth: Option<DateAndPlaceOfBirth>,

    #[serde(rename = "Othr", default)]
    pub other: Vec<GenericIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DateAndPlaceOfBirth {
    #[serde(rename = "BirthDt")]
    pub birth_date: String,

    #[serde(rename = "CityOfBirth")]
    pub city_of_birth: String,

    #[serde(rename = "CtryOfBirth")]
    pub country_of_birth: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenericIdentification {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "SchmeNm", default)]
    pub scheme_name: Option<SchemeName>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SchemeName {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BranchAndFinancialInstitutionIdentification {
    #[serde(rename = "FinInstnId")]
    pub financial_institution_identification: FinancialInstitutionIdentification,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FinancialInstitutionIdentification {
    #[serde(rename = "BICFI", default)]
    pub bic: Option<String>,

    #[serde(rename = "Nm", default)]
    pub name: Option<String>,

    #[serde(rename = "PstlAdr", default)]
    pub postal_address: Option<PostalAddressType>,

    #[serde(rename = "Othr", default)]
    pub other: Option<GenericIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CashAccount {
    #[serde(rename = "Id")]
    pub identification: AccountIdentificationType,

    #[serde(rename = "Tp", default)]
    pub account_type: Option<CashAccountType>,

    #[serde(rename = "Ccy", default)]
    pub currency: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountIdentificationType {
    #[serde(rename = "IBAN", default)]
    pub iban: Option<String>,

    #[serde(rename = "Othr", default)]
    pub other: Option<GenericAccountIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenericAccountIdentification {
    #[serde(rename = "Id")]
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CashAccountType {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Purpose {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RemittanceInformation {
    #[serde(rename = "Ustrd", default)]
    pub unstructured: Vec<String>,

    #[serde(rename = "Strd", default)]
    pub structured: Vec<StructuredRemittanceInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StructuredRemittanceInformation {
    // Simplified - full implementation has many more fields
    #[serde(rename = "RfrdDocInf", default)]
    pub referred_document_information: Vec<ReferredDocumentInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReferredDocumentInformation {
    #[serde(rename = "Tp", default)]
    pub document_type: Option<String>,

    #[serde(rename = "Nb", default)]
    pub number: Option<String>,
}

/// Parse pacs.008 XML into Document struct
pub fn parse_pacs008(xml: &str) -> Result<Document> {
    from_str(xml).context("Failed to parse pacs.008 XML")
}

/// Convert pacs.008 to canonical payments
pub fn to_canonical(pacs008: &Document) -> Result<Vec<CanonicalPayment>> {
    let mut payments = Vec::new();

    for tx in &pacs008.fi_to_fi_customer_credit_transfer.credit_transfer_transaction_information {
        let amount = Decimal::from_str(&tx.interbank_settlement_amount.value)
            .context("Invalid amount")?;

        let currency = Currency::from_str(&tx.interbank_settlement_amount.currency)
            .unwrap_or(Currency::Usd);

        // Extract UETR if present
        let uetr = tx.payment_identification.uetr.as_ref()
            .and_then(|s| Uuid::parse_str(s).ok());

        // Extract country codes from BIC
        let debtor_country = tx.debtor_agent.financial_institution_identification.bic.as_ref()
            .and_then(|bic| if bic.len() >= 6 { Some(bic[4..6].to_uppercase()) } else { None })
            .unwrap_or_else(|| "XX".to_string());

        let creditor_country = tx.creditor_agent.financial_institution_identification.bic.as_ref()
            .and_then(|bic| if bic.len() >= 6 { Some(bic[4..6].to_uppercase()) } else { None })
            .unwrap_or_else(|| "XX".to_string());

        // Build canonical payment
        let payment = CanonicalPayment {
            deltran_tx_id: Uuid::new_v4(),
            obligation_id: None,
            clearing_batch_id: None,
            settlement_id: None,
            uetr,
            end_to_end_id: tx.payment_identification.end_to_end_id.clone(),
            instruction_id: tx.payment_identification.instruction_id.clone()
                .unwrap_or_else(|| format!("PACS008-{}", Uuid::new_v4())),
            message_id: pacs008.fi_to_fi_customer_credit_transfer.group_header.message_id.clone(),
            instructed_amount: amount,
            settlement_amount: amount,
            currency,
            exchange_rate: None,
            debtor: Party {
                name: tx.debtor.name.clone().unwrap_or_default(),
                postal_address: tx.debtor.postal_address.as_ref().map(|addr| PostalAddress {
                    street_name: None,
                    building_number: None,
                    post_code: None,
                    town_name: None,
                    country: addr.country.clone().unwrap_or_default(),
                }),
                identification: None,
                country_code: debtor_country.clone(),
            },
            creditor: Party {
                name: tx.creditor.name.clone().unwrap_or_default(),
                postal_address: tx.creditor.postal_address.as_ref().map(|addr| PostalAddress {
                    street_name: None,
                    building_number: None,
                    post_code: None,
                    town_name: None,
                    country: addr.country.clone().unwrap_or_default(),
                }),
                identification: None,
                country_code: creditor_country.clone(),
            },
            debtor_agent: FinancialInstitution {
                bic: tx.debtor_agent.financial_institution_identification.bic.clone(),
                name: tx.debtor_agent.financial_institution_identification.name.clone().unwrap_or_default(),
                country_code: debtor_country,
                clearing_system_member_id: None,
            },
            creditor_agent: FinancialInstitution {
                bic: tx.creditor_agent.financial_institution_identification.bic.clone(),
                name: tx.creditor_agent.financial_institution_identification.name.clone().unwrap_or_default(),
                country_code: creditor_country,
                clearing_system_member_id: None,
            },
            debtor_account: tx.debtor_account.as_ref().map(|acc| AccountIdentification {
                iban: acc.identification.iban.clone(),
                other: acc.identification.other.as_ref().map(|o| o.id.clone()),
                bban: None,
                account_type: AccountType::Other,
            }).unwrap_or(AccountIdentification {
                iban: None,
                bban: None,
                other: None,
                account_type: AccountType::Other,
            }),
            creditor_account: tx.creditor_account.as_ref().map(|acc| AccountIdentification {
                iban: acc.identification.iban.clone(),
                other: acc.identification.other.as_ref().map(|o| o.id.clone()),
                bban: None,
                account_type: AccountType::Other,
            }).unwrap_or(AccountIdentification {
                iban: None,
                bban: None,
                other: None,
                account_type: AccountType::Other,
            }),
            creation_date: chrono::Utc::now(),
            requested_execution_date: None,
            settlement_date: None,
            value_date: None,
            status: PaymentStatus::Received,
            status_reason: None,
            charge_bearer: ChargeBearer::Shar,
            charges: vec![],
            remittance_info: tx.remittance_information.as_ref()
                .and_then(|ri| ri.unstructured.first().cloned())
                .unwrap_or_default(),
            remittance_structured: None,
            risk_score: None,
            compliance_status: ComplianceStatus::Pending,
            sanctions_checked: false,
            aml_score: None,
            corridor: format!("{}_TO_{}", debtor_country, creditor_country),
            priority: Priority::Normal,
            liquidity_pool_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        payments.push(payment);
    }

    Ok(payments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pacs008() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.10">
            <FIToFICstmrCdtTrf>
                <GrpHdr>
                    <MsgId>PACS008-20250118-001</MsgId>
                    <CreDtTm>2025-01-18T10:30:00</CreDtTm>
                    <NbOfTxs>1</NbOfTxs>
                    <SttlmInf>
                        <SttlmMtd>CLRG</SttlmMtd>
                    </SttlmInf>
                </GrpHdr>
                <CdtTrfTxInf>
                    <PmtId>
                        <EndToEndId>E2E-001</EndToEndId>
                    </PmtId>
                    <IntrBkSttlmAmt Ccy="USD">10000.00</IntrBkSttlmAmt>
                    <Dbtr>
                        <Nm>John Doe</Nm>
                    </Dbtr>
                    <DbtrAgt>
                        <FinInstnId>
                            <BICFI>BNPAFRPP</BICFI>
                        </FinInstnId>
                    </DbtrAgt>
                    <CdtrAgt>
                        <FinInstnId>
                            <BICFI>CHASUS33</BICFI>
                        </FinInstnId>
                    </CdtrAgt>
                    <Cdtr>
                        <Nm>Jane Smith</Nm>
                    </Cdtr>
                </CdtTrfTxInf>
            </FIToFICstmrCdtTrf>
        </Document>
        "#;

        let result = parse_pacs008(xml);
        assert!(result.is_ok());

        let document = result.unwrap();
        assert_eq!(document.fi_to_fi_customer_credit_transfer.group_header.message_id, "PACS008-20250118-001");
    }

    #[test]
    fn test_pacs008_to_canonical() {
        // TODO: Add comprehensive conversion test
    }
}
