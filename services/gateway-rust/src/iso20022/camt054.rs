// camt.054 - Bank to Customer Debit/Credit Notification
// THIS IS THE MOST CRITICAL MESSAGE FOR DELTRAN
// It confirms real money has been received (FUNDING EVENT)

use serde::{Deserialize, Serialize};
use quick_xml::de::from_str;
use anyhow::{Result, Context};
use uuid::Uuid;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::models::canonical::{Currency, AccountIdentification};

// camt.054.001.10 - BankToCustomerDebitCreditNotification
#[derive(Debug, Deserialize, Serialize)]
pub struct Document {
    #[serde(rename = "BkToCstmrDbtCdtNtfctn")]
    pub bank_to_customer_debit_credit_notification: BankToCustomerDebitCreditNotification,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BankToCustomerDebitCreditNotification {
    #[serde(rename = "GrpHdr")]
    pub group_header: GroupHeader,

    #[serde(rename = "Ntfctn")]
    pub notification: Vec<AccountNotification>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupHeader {
    #[serde(rename = "MsgId")]
    pub message_id: String,

    #[serde(rename = "CreDtTm")]
    pub creation_date_time: String,

    #[serde(rename = "MsgPgntn", default)]
    pub message_pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Pagination {
    #[serde(rename = "PgNb")]
    pub page_number: String,

    #[serde(rename = "LastPgInd")]
    pub last_page_indicator: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountNotification {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "CreDtTm", default)]
    pub creation_date_time: Option<String>,

    #[serde(rename = "Acct")]
    pub account: CashAccount,

    #[serde(rename = "Ntry")]
    pub entry: Vec<ReportEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CashAccount {
    #[serde(rename = "Id")]
    pub identification: AccountIdentificationType,

    #[serde(rename = "Tp", default)]
    pub account_type: Option<CashAccountType>,

    #[serde(rename = "Ccy", default)]
    pub currency: Option<String>,

    #[serde(rename = "Nm", default)]
    pub name: Option<String>,

    #[serde(rename = "Svcr", default)]
    pub servicer: Option<BranchAndFinancialInstitutionIdentification>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccountIdentificationType {
    #[serde(rename = "IBAN", default)]
    pub iban: Option<String>,

    #[serde(rename = "Othr", default)]
    pub other: Option<GenericAccountIdentification>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GenericAccountIdentification {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "SchmeNm", default)]
    pub scheme_name: Option<SchemeName>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SchemeName {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CashAccountType {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BranchAndFinancialInstitutionIdentification {
    #[serde(rename = "FinInstnId")]
    pub financial_institution_identification: FinancialInstitutionIdentification,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FinancialInstitutionIdentification {
    #[serde(rename = "BICFI", default)]
    pub bic: Option<String>,

    #[serde(rename = "Nm", default)]
    pub name: Option<String>,
}

// THIS IS THE CRITICAL PART - Each entry represents money in/out
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportEntry {
    #[serde(rename = "NtryRef", default)]
    pub entry_reference: Option<String>,

    #[serde(rename = "Amt")]
    pub amount: ActiveOrHistoricCurrencyAndAmount,

    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: String,  // CRDT or DBIT

    #[serde(rename = "Sts")]
    pub status: EntryStatus,

    #[serde(rename = "BookgDt", default)]
    pub booking_date: Option<DateAndDateTime>,

    #[serde(rename = "ValDt", default)]
    pub value_date: Option<DateAndDateTime>,

    #[serde(rename = "AcctSvcrRef", default)]
    pub account_servicer_reference: Option<String>,

    #[serde(rename = "BkTxCd")]
    pub bank_transaction_code: BankTransactionCode,

    #[serde(rename = "NtryDtls", default)]
    pub entry_details: Vec<EntryDetails>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ActiveOrHistoricCurrencyAndAmount {
    #[serde(rename = "Ccy")]
    pub currency: String,

    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EntryStatus {
    #[serde(rename = "Cd", default)]
    pub code: Option<String>,  // BOOK, PDNG, INFO

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DateAndDateTime {
    #[serde(rename = "Dt", default)]
    pub date: Option<String>,

    #[serde(rename = "DtTm", default)]
    pub date_time: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BankTransactionCode {
    #[serde(rename = "Domn", default)]
    pub domain: Option<BankTransactionCodeDomain>,

    #[serde(rename = "Prtry", default)]
    pub proprietary: Option<ProprietaryBankTransactionCode>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BankTransactionCodeDomain {
    #[serde(rename = "Cd")]
    pub code: String,

    #[serde(rename = "Fmly")]
    pub family: BankTransactionCodeFamily,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BankTransactionCodeFamily {
    #[serde(rename = "Cd")]
    pub code: String,

    #[serde(rename = "SubFmlyCd")]
    pub sub_family_code: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProprietaryBankTransactionCode {
    #[serde(rename = "Cd")]
    pub code: String,

    #[serde(rename = "Issr", default)]
    pub issuer: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EntryDetails {
    #[serde(rename = "TxDtls", default)]
    pub transaction_details: Vec<TransactionDetails>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionDetails {
    #[serde(rename = "Refs", default)]
    pub references: Option<TransactionReferences>,

    #[serde(rename = "Amt", default)]
    pub amount: Option<ActiveOrHistoricCurrencyAndAmount>,

    #[serde(rename = "CdtDbtInd", default)]
    pub credit_debit_indicator: Option<String>,

    #[serde(rename = "RltdPties", default)]
    pub related_parties: Option<RelatedParties>,

    #[serde(rename = "RmtInf", default)]
    pub remittance_information: Option<RemittanceInformation>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionReferences {
    #[serde(rename = "MsgId", default)]
    pub message_id: Option<String>,

    #[serde(rename = "AcctSvcrRef", default)]
    pub account_servicer_reference: Option<String>,

    #[serde(rename = "PmtInfId", default)]
    pub payment_information_id: Option<String>,

    #[serde(rename = "InstrId", default)]
    pub instruction_id: Option<String>,

    #[serde(rename = "EndToEndId", default)]
    pub end_to_end_id: Option<String>,

    #[serde(rename = "TxId", default)]
    pub transaction_id: Option<String>,

    #[serde(rename = "UETR", default)]
    pub uetr: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RelatedParties {
    #[serde(rename = "Dbtr", default)]
    pub debtor: Option<Party>,

    #[serde(rename = "DbtrAcct", default)]
    pub debtor_account: Option<CashAccount>,

    #[serde(rename = "Cdtr", default)]
    pub creditor: Option<Party>,

    #[serde(rename = "CdtrAcct", default)]
    pub creditor_account: Option<CashAccount>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Party {
    #[serde(rename = "Nm", default)]
    pub name: Option<String>,

    #[serde(rename = "PstlAdr", default)]
    pub postal_address: Option<PostalAddress>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PostalAddress {
    #[serde(rename = "Ctry", default)]
    pub country: Option<String>,

    #[serde(rename = "AdrLine", default)]
    pub address_line: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RemittanceInformation {
    #[serde(rename = "Ustrd", default)]
    pub unstructured: Vec<String>,
}

/// Funding event extracted from camt.054
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingEvent {
    pub account: String,  // IBAN or other account ID
    pub amount: Decimal,
    pub currency: Currency,
    pub credit_debit: CreditDebit,
    pub status: String,  // BOOK, PDNG, etc.
    pub booking_date: Option<String>,
    pub value_date: Option<String>,
    pub end_to_end_id: Option<String>,
    pub instruction_id: Option<String>,
    pub uetr: Option<Uuid>,
    pub debtor_name: Option<String>,
    pub creditor_name: Option<String>,
    pub remittance_info: Option<String>,
    pub entry_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditDebit {
    Credit,  // Money IN (this is what we care about!)
    Debit,   // Money OUT
}

/// Parse camt.054 XML into Document struct
pub fn parse_camt054(xml: &str) -> Result<Document> {
    from_str(xml).context("Failed to parse camt.054 XML")
}

/// Extract funding events from camt.054
/// CRITICAL: This identifies when real money has hit the EMI account
pub fn extract_funding_events(camt054: &Document) -> Result<Vec<FundingEvent>> {
    let mut events = Vec::new();

    for notification in &camt054.bank_to_customer_debit_credit_notification.notification {
        let account_id = notification.account.identification.iban.clone()
            .or_else(|| notification.account.identification.other.as_ref().map(|o| o.id.clone()))
            .unwrap_or_default();

        for entry in &notification.entry {
            // Parse amount
            let amount = Decimal::from_str(&entry.amount.value)
                .context("Invalid amount in camt.054")?;

            let currency = Currency::from_str(&entry.amount.currency)
                .unwrap_or(Currency::Usd);

            let credit_debit = match entry.credit_debit_indicator.as_str() {
                "CRDT" => CreditDebit::Credit,  // Money IN - THIS IS FUNDING!
                "DBIT" => CreditDebit::Debit,   // Money OUT
                _ => continue,  // Skip unknown
            };

            // Extract transaction details if present
            let (end_to_end_id, instruction_id, uetr, debtor_name, creditor_name, remittance_info) =
                if let Some(entry_detail) = entry.entry_details.first() {
                    if let Some(tx_detail) = entry_detail.transaction_details.first() {
                        let e2e = tx_detail.references.as_ref()
                            .and_then(|r| r.end_to_end_id.clone());
                        let instr = tx_detail.references.as_ref()
                            .and_then(|r| r.instruction_id.clone());
                        let uetr_str = tx_detail.references.as_ref()
                            .and_then(|r| r.uetr.clone());
                        let uetr_uuid = uetr_str.and_then(|s| Uuid::parse_str(&s).ok());
                        let debtor = tx_detail.related_parties.as_ref()
                            .and_then(|rp| rp.debtor.as_ref())
                            .and_then(|d| d.name.clone());
                        let creditor = tx_detail.related_parties.as_ref()
                            .and_then(|rp| rp.creditor.as_ref())
                            .and_then(|c| c.name.clone());
                        let remit = tx_detail.remittance_information.as_ref()
                            .and_then(|ri| ri.unstructured.first().cloned());

                        (e2e, instr, uetr_uuid, debtor, creditor, remit)
                    } else {
                        (None, None, None, None, None, None)
                    }
                } else {
                    (None, None, None, None, None, None)
                };

            let event = FundingEvent {
                account: account_id.clone(),
                amount,
                currency,
                credit_debit,
                status: entry.status.code.clone().unwrap_or_else(|| "UNKNOWN".to_string()),
                booking_date: entry.booking_date.as_ref().and_then(|d| d.date.clone()),
                value_date: entry.value_date.as_ref().and_then(|d| d.date.clone()),
                end_to_end_id,
                instruction_id,
                uetr,
                debtor_name,
                creditor_name,
                remittance_info,
                entry_reference: entry.entry_reference.clone(),
            };

            events.push(event);
        }
    }

    Ok(events)
}

/// Check if funding event is a CREDIT (money IN)
pub fn is_credit_event(event: &FundingEvent) -> bool {
    matches!(event.credit_debit, CreditDebit::Credit)
}

/// Check if funding is booked (not pending)
pub fn is_booked(event: &FundingEvent) -> bool {
    event.status == "BOOK"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_camt054() {
        let xml = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.054.001.10">
            <BkToCstmrDbtCdtNtfctn>
                <GrpHdr>
                    <MsgId>CAMT054-20250118-001</MsgId>
                    <CreDtTm>2025-01-18T10:30:00</CreDtTm>
                </GrpHdr>
                <Ntfctn>
                    <Id>N001</Id>
                    <Acct>
                        <Id>
                            <IBAN>AE070331234567890123456</IBAN>
                        </Id>
                    </Acct>
                    <Ntry>
                        <Amt Ccy="AED">10000.00</Amt>
                        <CdtDbtInd>CRDT</CdtDbtInd>
                        <Sts>
                            <Cd>BOOK</Cd>
                        </Sts>
                        <BkTxCd>
                            <Prtry>
                                <Cd>TRANSFER</Cd>
                            </Prtry>
                        </BkTxCd>
                    </Ntry>
                </Ntfctn>
            </BkToCstmrDbtCdtNtfctn>
        </Document>
        "#;

        let result = parse_camt054(xml);
        assert!(result.is_ok());

        let document = result.unwrap();
        assert_eq!(document.bank_to_customer_debit_credit_notification.group_header.message_id, "CAMT054-20250118-001");
    }

    #[test]
    fn test_extract_funding_events() {
        // TODO: Add comprehensive funding event extraction test
    }

    #[test]
    fn test_is_credit_event() {
        let event = FundingEvent {
            account: "AE070331234567890123456".to_string(),
            amount: Decimal::new(10000, 2),
            currency: Currency::AED,
            credit_debit: CreditDebit::Credit,
            status: "BOOK".to_string(),
            booking_date: None,
            value_date: None,
            end_to_end_id: None,
            instruction_id: None,
            uetr: None,
            debtor_name: None,
            creditor_name: None,
            remittance_info: None,
            entry_reference: None,
        };

        assert!(is_credit_event(&event));
        assert!(is_booked(&event));
    }
}
