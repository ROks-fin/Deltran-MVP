// ISO 20022 camt.053.001.02 - BankToCustomerStatement
// Used for End-of-Day (EOD) reconciliation
// Critical for three-tier reconciliation system

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use crate::errors::ClearingError;
use super::common::{
    ActiveOrHistoricCurrencyAndAmount, PartyIdentification,
    AccountIdentification, AccountId, Agent, CreditDebitCode, EntryStatus,
};

/// camt.053 Document - BankToCustomerStatement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camt053Document {
    #[serde(rename = "@xmlns")]
    pub xmlns: String,
    #[serde(rename = "BkToCstmrStmt")]
    pub bank_to_customer_statement: BankToCustomerStatement,
}

/// Bank to Customer Statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankToCustomerStatement {
    #[serde(rename = "GrpHdr")]
    pub group_header: GroupHeader,
    #[serde(rename = "Stmt")]
    pub statements: Vec<AccountStatement>,
}

/// Group Header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupHeader {
    #[serde(rename = "MsgId")]
    pub message_id: String,
    #[serde(rename = "CreDtTm")]
    pub creation_date_time: DateTime<Utc>,
    #[serde(rename = "MsgRcpt", skip_serializing_if = "Option::is_none")]
    pub message_recipient: Option<PartyIdentification>,
}

/// Account Statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountStatement {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "ElctrncSeqNb", skip_serializing_if = "Option::is_none")]
    pub electronic_sequence_number: Option<u32>,
    #[serde(rename = "CreDtTm")]
    pub creation_date_time: DateTime<Utc>,
    #[serde(rename = "FrToDt", skip_serializing_if = "Option::is_none")]
    pub from_to_date: Option<DatePeriod>,
    #[serde(rename = "Acct")]
    pub account: CashAccount,
    #[serde(rename = "Bal")]
    pub balances: Vec<CashBalance>,
    #[serde(rename = "Ntry", skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<ReportEntry>>,
}

/// Date Period for statement coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatePeriod {
    #[serde(rename = "FrDtTm")]
    pub from_date_time: DateTime<Utc>,
    #[serde(rename = "ToDtTm")]
    pub to_date_time: DateTime<Utc>,
}

/// Cash Account Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAccount {
    #[serde(rename = "Id")]
    pub id: AccountIdentification,
    #[serde(rename = "Tp", skip_serializing_if = "Option::is_none")]
    pub account_type: Option<CashAccountType>,
    #[serde(rename = "Ccy", skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(rename = "Nm", skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "Ownr", skip_serializing_if = "Option::is_none")]
    pub owner: Option<PartyIdentification>,
    #[serde(rename = "Svcr", skip_serializing_if = "Option::is_none")]
    pub servicer: Option<Agent>,
}

/// Cash Account Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAccountType {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<String>, // CACC (current), SVGS (savings), etc.
    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Cash Balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashBalance {
    #[serde(rename = "Tp")]
    pub balance_type: BalanceType,
    #[serde(rename = "Amt")]
    pub amount: ActiveOrHistoricCurrencyAndAmount,
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: CreditDebitCode,
    #[serde(rename = "Dt")]
    pub date: BalanceDate,
}

/// Balance Type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceType {
    #[serde(rename = "CdOrPrtry")]
    pub code_or_proprietary: CodeOrProprietary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeOrProprietary {
    #[serde(rename = "Cd", skip_serializing_if = "Option::is_none")]
    pub code: Option<BalanceCode>,
    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<String>,
}

/// Balance Code Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BalanceCode {
    OPBD, // Opening Booked
    CLBD, // Closing Booked
    ITBD, // Interim Booked
    PRCD, // Previously Closed Booked
    FWAV, // Forward Available
}

/// Balance Date
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceDate {
    #[serde(rename = "Dt", skip_serializing_if = "Option::is_none")]
    pub date: Option<chrono::NaiveDate>,
    #[serde(rename = "DtTm", skip_serializing_if = "Option::is_none")]
    pub date_time: Option<DateTime<Utc>>,
}

/// Report Entry - Individual transaction in statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEntry {
    #[serde(rename = "Amt")]
    pub amount: ActiveOrHistoricCurrencyAndAmount,
    #[serde(rename = "CdtDbtInd")]
    pub credit_debit_indicator: CreditDebitCode,
    #[serde(rename = "Sts")]
    pub status: EntryStatus,
    #[serde(rename = "BookgDt", skip_serializing_if = "Option::is_none")]
    pub booking_date: Option<BalanceDate>,
    #[serde(rename = "ValDt", skip_serializing_if = "Option::is_none")]
    pub value_date: Option<BalanceDate>,
    #[serde(rename = "AcctSvcrRef", skip_serializing_if = "Option::is_none")]
    pub account_servicer_reference: Option<String>,
    #[serde(rename = "BkTxCd", skip_serializing_if = "Option::is_none")]
    pub bank_transaction_code: Option<BankTransactionCode>,
    #[serde(rename = "NtryDtls", skip_serializing_if = "Option::is_none")]
    pub entry_details: Option<Vec<EntryDetails>>,
}

/// Bank Transaction Code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionCode {
    #[serde(rename = "Domn", skip_serializing_if = "Option::is_none")]
    pub domain: Option<BankTransactionCodeDomain>,
    #[serde(rename = "Prtry", skip_serializing_if = "Option::is_none")]
    pub proprietary: Option<ProprietaryBankTransactionCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionCodeDomain {
    #[serde(rename = "Cd")]
    pub code: String,
    #[serde(rename = "Fmly")]
    pub family: BankTransactionCodeFamily,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionCodeFamily {
    #[serde(rename = "Cd")]
    pub code: String,
    #[serde(rename = "SubFmlyCd")]
    pub sub_family_code: String,
}

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
    #[serde(rename = "TxDtls", skip_serializing_if = "Option::is_none")]
    pub transaction_details: Option<Vec<TransactionDetails>>,
}

/// Transaction Details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDetails {
    #[serde(rename = "Refs", skip_serializing_if = "Option::is_none")]
    pub references: Option<TransactionReferences>,
    #[serde(rename = "Amt", skip_serializing_if = "Option::is_none")]
    pub amount: Option<ActiveOrHistoricCurrencyAndAmount>,
    #[serde(rename = "RltdPties", skip_serializing_if = "Option::is_none")]
    pub related_parties: Option<RelatedParties>,
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
    #[serde(rename = "UETR", skip_serializing_if = "Option::is_none")]
    pub uetr: Option<String>, // UETR support!
}

/// Related Parties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedParties {
    #[serde(rename = "Dbtr", skip_serializing_if = "Option::is_none")]
    pub debtor: Option<PartyIdentification>,
    #[serde(rename = "DbtrAcct", skip_serializing_if = "Option::is_none")]
    pub debtor_account: Option<CashAccount>,
    #[serde(rename = "Cdtr", skip_serializing_if = "Option::is_none")]
    pub creditor: Option<PartyIdentification>,
    #[serde(rename = "CdtrAcct", skip_serializing_if = "Option::is_none")]
    pub creditor_account: Option<CashAccount>,
}

/// Parse camt.053 XML document
pub fn parse_camt053(xml: &str) -> Result<Camt053Document, quick_xml::DeError> {
    quick_xml::de::from_str(xml)
}

/// EOD Reconciliation Information extracted from statement
#[derive(Debug, Clone)]
pub struct EODReconciliationInfo {
    pub account_number: String,
    pub currency: String,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub opening_indicator: CreditDebitCode,
    pub closing_indicator: CreditDebitCode,
    pub statement_date: DateTime<Utc>,
    pub transactions: Vec<TransactionSummary>,
}

#[derive(Debug, Clone)]
pub struct TransactionSummary {
    pub amount: Decimal,
    pub credit_debit: CreditDebitCode,
    pub reference: Option<String>,
    pub uetr: Option<String>,
    pub booking_date: Option<DateTime<Utc>>,
}

/// Extract EOD reconciliation information from camt.053
/// This is critical for the three-tier reconciliation system
pub fn extract_eod_reconciliation(doc: &Camt053Document) -> Result<Vec<EODReconciliationInfo>, ClearingError> {
    let mut reconciliation_info = Vec::new();

    for statement in &doc.bank_to_customer_statement.statements {
        // Extract account number
        let account_number = match &statement.account.id.identification {
            AccountId::IBAN(iban) => iban.clone(),
            AccountId::Other(other) => other.id.clone(),
        };

        let currency = statement.account.currency.clone()
            .unwrap_or_else(|| "USD".to_string());

        // Find opening and closing balances
        let mut opening_balance = Decimal::ZERO;
        let mut closing_balance = Decimal::ZERO;
        let mut opening_indicator = CreditDebitCode::CRDT;
        let mut closing_indicator = CreditDebitCode::CRDT;

        for balance in &statement.balances {
            let amount = balance.amount.to_decimal()
                .map_err(|e| ClearingError::Internal(format!("Failed to parse balance: {}", e)))?;

            match &balance.balance_type.code_or_proprietary.code {
                Some(BalanceCode::OPBD) => {
                    opening_balance = amount;
                    opening_indicator = balance.credit_debit_indicator.clone();
                }
                Some(BalanceCode::CLBD) => {
                    closing_balance = amount;
                    closing_indicator = balance.credit_debit_indicator.clone();
                }
                _ => {}
            }
        }

        // Extract transactions
        let mut transactions = Vec::new();
        if let Some(entries) = &statement.entries {
            for entry in entries {
                let amount = entry.amount.to_decimal()
                    .map_err(|e| ClearingError::Internal(format!("Failed to parse entry amount: {}", e)))?;

                // Extract UETR from entry details if available
                let mut uetr = None;
                let mut reference = entry.account_servicer_reference.clone();

                if let Some(details) = &entry.entry_details {
                    for detail in details {
                        if let Some(txn_details) = &detail.transaction_details {
                            for txn in txn_details {
                                if let Some(refs) = &txn.references {
                                    uetr = refs.uetr.clone();
                                    if reference.is_none() {
                                        reference = refs.end_to_end_id.clone();
                                    }
                                }
                            }
                        }
                    }
                }

                // Extract booking date
                let booking_date = entry.booking_date.as_ref()
                    .and_then(|bd| bd.date_time);

                transactions.push(TransactionSummary {
                    amount,
                    credit_debit: entry.credit_debit_indicator.clone(),
                    reference,
                    uetr,
                    booking_date,
                });
            }
        }

        reconciliation_info.push(EODReconciliationInfo {
            account_number,
            currency,
            opening_balance,
            closing_balance,
            opening_indicator,
            closing_indicator,
            statement_date: statement.creation_date_time,
            transactions,
        });
    }

    Ok(reconciliation_info)
}

/// Calculate expected closing balance from opening balance and transactions
/// Used to verify statement integrity
pub fn calculate_expected_closing(
    opening: Decimal,
    opening_indicator: &CreditDebitCode,
    transactions: &[TransactionSummary],
) -> Result<(Decimal, CreditDebitCode), ClearingError> {
    let mut balance = opening;
    let mut is_credit = matches!(opening_indicator, CreditDebitCode::CRDT);

    for txn in transactions {
        match (&txn.credit_debit, is_credit) {
            (CreditDebitCode::CRDT, true) => {
                // Credit to credit account: add
                balance = balance.checked_add(txn.amount)
                    .ok_or(ClearingError::CalculationOverflow)?;
            }
            (CreditDebitCode::DBIT, true) => {
                // Debit from credit account: subtract
                balance = balance.checked_sub(txn.amount)
                    .ok_or(ClearingError::CalculationUnderflow)?;
                if balance < Decimal::ZERO {
                    balance = balance.abs();
                    is_credit = false;
                }
            }
            (CreditDebitCode::CRDT, false) => {
                // Credit to debit account: subtract from debit
                balance = balance.checked_sub(txn.amount)
                    .ok_or(ClearingError::CalculationUnderflow)?;
                if balance < Decimal::ZERO {
                    balance = balance.abs();
                    is_credit = true;
                }
            }
            (CreditDebitCode::DBIT, false) => {
                // Debit from debit account: add to debit
                balance = balance.checked_add(txn.amount)
                    .ok_or(ClearingError::CalculationOverflow)?;
            }
        }
    }

    let indicator = if is_credit {
        CreditDebitCode::CRDT
    } else {
        CreditDebitCode::DBIT
    };

    Ok((balance, indicator))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_calculation() {
        let opening = Decimal::new(100000, 2); // 1000.00
        let opening_indicator = CreditDebitCode::CRDT;

        let transactions = vec![
            TransactionSummary {
                amount: Decimal::new(50000, 2), // 500.00 credit
                credit_debit: CreditDebitCode::CRDT,
                reference: None,
                uetr: None,
                booking_date: None,
            },
            TransactionSummary {
                amount: Decimal::new(20000, 2), // 200.00 debit
                credit_debit: CreditDebitCode::DBIT,
                reference: None,
                uetr: None,
                booking_date: None,
            },
        ];

        let (closing, indicator) = calculate_expected_closing(
            opening,
            &opening_indicator,
            &transactions,
        ).unwrap();

        // 1000.00 + 500.00 - 200.00 = 1300.00
        assert_eq!(closing, Decimal::new(130000, 2));
        assert_eq!(indicator, CreditDebitCode::CRDT);
    }

    #[test]
    fn test_eod_reconciliation_structure() {
        // Test structure creation
        let info = EODReconciliationInfo {
            account_number: "AE070331234567890123456".to_string(),
            currency: "USD".to_string(),
            opening_balance: Decimal::new(100000, 2),
            closing_balance: Decimal::new(150000, 2),
            opening_indicator: CreditDebitCode::CRDT,
            closing_indicator: CreditDebitCode::CRDT,
            statement_date: Utc::now(),
            transactions: vec![],
        };

        assert_eq!(info.currency, "USD");
        assert_eq!(info.opening_balance, Decimal::new(100000, 2));
    }
}
