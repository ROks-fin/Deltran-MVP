// camt.053.001.08 - Bank to Customer Statement
// End-of-day (EOD) statement for account reconciliation

use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc, NaiveDate};
use uuid::Uuid;
use anyhow::{Result, anyhow};

/// camt.053 Document root
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Document {
    #[serde(rename = "BkToCstmrStmt")]
    pub bank_to_customer_statement: BankToCustomerStatement,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BankToCustomerStatement {
    pub grp_hdr: GroupHeader,
    pub stmt: Vec<Statement>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GroupHeader {
    pub msg_id: String,
    pub cre_dt_tm: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Statement {
    pub id: String,
    pub cre_dt_tm: String,
    pub fr_to_dt: Option<FromToDate>,
    pub acct: Account,
    pub bal: Vec<Balance>,
    pub ntry: Option<Vec<Entry>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FromToDate {
    #[serde(rename = "FrDtTm")]
    pub from_date_time: String,

    #[serde(rename = "ToDtTm")]
    pub to_date_time: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Account {
    pub id: AccountId,
    pub tp: Option<AccountType>,
    pub ccy: Option<String>,
    pub nm: Option<String>,
    pub svcr: Option<Servicer>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct AccountId {
    #[serde(rename = "IBAN")]
    pub iban: Option<String>,

    #[serde(rename = "Othr")]
    pub other: Option<OtherAccountId>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OtherAccountId {
    pub id: String,
    pub schme_nm: Option<SchemeName>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SchemeName {
    pub cd: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AccountType {
    pub cd: String, // CACC, SVGS, etc.
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Servicer {
    pub fin_instn_id: FinancialInstitutionId,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FinancialInstitutionId {
    #[serde(rename = "BICFI")]
    pub bic: Option<String>,
    pub nm: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Balance {
    pub tp: BalanceType,
    pub amt: Amount,
    pub cdt_dbt_ind: String, // CRDT or DBIT
    pub dt: BalanceDate,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BalanceType {
    #[serde(rename = "CdOrPrtry")]
    pub code_or_proprietary: CodeOrProprietary,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CodeOrProprietary {
    pub cd: Option<String>, // OPBD (Opening booked), CLBD (Closing booked), etc.
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Amount {
    #[serde(rename = "@Ccy")]
    pub currency: String,

    #[serde(rename = "$text")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BalanceDate {
    pub dt: Option<String>,
    pub dt_tm: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Entry {
    pub amt: Amount,
    pub cdt_dbt_ind: String, // CRDT or DBIT
    pub sts: Status,
    pub booking_dt: Option<BookingDate>,
    pub val_dt: Option<ValueDate>,
    pub acct_svcr_ref: Option<String>,
    pub ntry_dtls: Option<Vec<EntryDetails>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Status {
    pub cd: String, // BOOK, PDNG, INFO
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BookingDate {
    pub dt: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ValueDate {
    pub dt: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EntryDetails {
    pub tx_dtls: Option<Vec<TransactionDetails>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TransactionDetails {
    pub refs: References,
    pub amt_dtls: Option<AmountDetails>,
    pub rltd_pties: Option<RelatedParties>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct References {
    pub end_to_end_id: Option<String>,
    pub tx_id: Option<String>,
    pub instr_id: Option<String>,
    #[serde(rename = "UETR")]
    pub uetr: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AmountDetails {
    pub tx_amt: Option<Amount>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RelatedParties {
    pub dbtr: Option<Party>,
    pub cdtr: Option<Party>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Party {
    pub nm: Option<String>,
}

/// Statement summary for DelTran
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementSummary {
    pub statement_id: String,
    pub account_id: String,
    pub account_iban: Option<String>,
    pub currency: String,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub opening_balance: Option<Decimal>,
    pub closing_balance: Option<Decimal>,
    pub total_entries: usize,
    pub total_credits: Decimal,
    pub total_debits: Decimal,
    pub entries: Vec<StatementEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementEntry {
    pub entry_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub credit_debit_indicator: String,
    pub status: String,
    pub booking_date: Option<NaiveDate>,
    pub value_date: Option<NaiveDate>,
    pub end_to_end_id: Option<String>,
    pub transaction_id: Option<String>,
    pub uetr: Option<Uuid>,
    pub debtor_name: Option<String>,
    pub creditor_name: Option<String>,
}

/// Parse camt.053 XML message
pub fn parse_camt053(xml: &str) -> Result<Document> {
    quick_xml::de::from_str(xml)
        .map_err(|e| anyhow!("Failed to parse camt.053 XML: {}", e))
}

/// Convert camt.053 to DelTran statement summaries
pub fn to_statement_summaries(document: &Document) -> Result<Vec<StatementSummary>> {
    let mut summaries = Vec::new();

    for stmt in &document.bank_to_customer_statement.stmt {
        let account_id = if let Some(ref iban) = stmt.acct.id.iban {
            iban.clone()
        } else if let Some(ref other) = stmt.acct.id.other {
            other.id.clone()
        } else {
            "UNKNOWN".to_string()
        };

        let currency = stmt.acct.ccy.clone()
            .unwrap_or_else(|| "XXX".to_string());

        // Extract opening and closing balances
        let opening_balance = stmt.bal.iter()
            .find(|b| b.tp.code_or_proprietary.cd.as_deref() == Some("OPBD"))
            .and_then(|b| b.amt.value.parse::<Decimal>().ok());

        let closing_balance = stmt.bal.iter()
            .find(|b| b.tp.code_or_proprietary.cd.as_deref() == Some("CLBD"))
            .and_then(|b| b.amt.value.parse::<Decimal>().ok());

        // Parse date range
        let (from_date, to_date) = if let Some(ref fr_to) = stmt.fr_to_dt {
            (
                DateTime::parse_from_rfc3339(&fr_to.from_date_time).ok()
                    .map(|dt| dt.with_timezone(&Utc)),
                DateTime::parse_from_rfc3339(&fr_to.to_date_time).ok()
                    .map(|dt| dt.with_timezone(&Utc)),
            )
        } else {
            (None, None)
        };

        // Process entries
        let mut entries = Vec::new();
        let mut total_credits = Decimal::ZERO;
        let mut total_debits = Decimal::ZERO;

        if let Some(ref entry_list) = stmt.ntry {
            for entry in entry_list {
                let amount: Decimal = entry.amt.value.parse()
                    .unwrap_or(Decimal::ZERO);

                if entry.cdt_dbt_ind == "CRDT" {
                    total_credits += amount;
                } else {
                    total_debits += amount;
                }

                let booking_date = entry.booking_dt.as_ref()
                    .and_then(|bd| NaiveDate::parse_from_str(&bd.dt, "%Y-%m-%d").ok());

                let value_date = entry.val_dt.as_ref()
                    .and_then(|vd| NaiveDate::parse_from_str(&vd.dt, "%Y-%m-%d").ok());

                // Extract transaction details if available
                let (end_to_end_id, tx_id, uetr, debtor, creditor) = if let Some(ref details) = entry.ntry_dtls {
                    if let Some(tx_details) = details.first().and_then(|d| d.tx_dtls.as_ref()).and_then(|t| t.first()) {
                        (
                            tx_details.refs.end_to_end_id.clone(),
                            tx_details.refs.tx_id.clone(),
                            tx_details.refs.uetr.as_ref().and_then(|u| Uuid::parse_str(u).ok()),
                            tx_details.rltd_pties.as_ref()
                                .and_then(|rp| rp.dbtr.as_ref())
                                .and_then(|d| d.nm.clone()),
                            tx_details.rltd_pties.as_ref()
                                .and_then(|rp| rp.cdtr.as_ref())
                                .and_then(|c| c.nm.clone()),
                        )
                    } else {
                        (None, None, None, None, None)
                    }
                } else {
                    (None, None, None, None, None)
                };

                entries.push(StatementEntry {
                    entry_id: Uuid::new_v4(),
                    amount,
                    currency: entry.amt.currency.clone(),
                    credit_debit_indicator: entry.cdt_dbt_ind.clone(),
                    status: entry.sts.cd.clone(),
                    booking_date,
                    value_date,
                    end_to_end_id,
                    transaction_id: tx_id,
                    uetr,
                    debtor_name: debtor,
                    creditor_name: creditor,
                });
            }
        }

        summaries.push(StatementSummary {
            statement_id: stmt.id.clone(),
            account_id,
            account_iban: stmt.acct.id.iban.clone(),
            currency,
            from_date,
            to_date,
            opening_balance,
            closing_balance,
            total_entries: entries.len(),
            total_credits,
            total_debits,
            entries,
        });
    }

    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_camt053() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.08">
  <BkToCstmrStmt>
    <GrpHdr>
      <MsgId>STMT001</MsgId>
      <CreDtTm>2025-01-19T23:59:59Z</CreDtTm>
    </GrpHdr>
    <Stmt>
      <Id>DAILY_STMT_20250119</Id>
      <CreDtTm>2025-01-19T23:59:59Z</CreDtTm>
      <Acct>
        <Id>
          <IBAN>AE070331234567890123456</IBAN>
        </Id>
        <Ccy>AED</Ccy>
      </Acct>
      <Bal>
        <Tp>
          <CdOrPrtry>
            <Cd>OPBD</Cd>
          </CdOrPrtry>
        </Tp>
        <Amt Ccy="AED">1000000.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt>
          <Dt>2025-01-19</Dt>
        </Dt>
      </Bal>
      <Bal>
        <Tp>
          <CdOrPrtry>
            <Cd>CLBD</Cd>
          </CdOrPrtry>
        </Tp>
        <Amt Ccy="AED">1050000.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt>
          <Dt>2025-01-19</Dt>
        </Dt>
      </Bal>
    </Stmt>
  </BkToCstmrStmt>
</Document>"#;

        let document = parse_camt053(xml).unwrap();
        let summaries = to_statement_summaries(&document).unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].account_iban, Some("AE070331234567890123456".to_string()));
        assert_eq!(summaries[0].currency, "AED");
        assert_eq!(summaries[0].opening_balance, Some(Decimal::new(100000000, 2)));
        assert_eq!(summaries[0].closing_balance, Some(Decimal::new(105000000, 2)));
    }
}
