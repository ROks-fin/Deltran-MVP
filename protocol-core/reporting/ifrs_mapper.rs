//! IFRS Mapping from Subledger
//!
//! Maps DelTran subledger transactions to IFRS financial statements:
//! - IFRS 9: Financial Instruments
//! - IFRS 15: Revenue from Contracts
//! - IAS 1: Presentation of Financial Statements
//! - IAS 7: Statement of Cash Flows

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum IfrsMapperError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Mapping error: {0}")]
    MappingError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// IFRS Account Classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IfrsAccount {
    // Assets (IFRS 9)
    CashAndCashEquivalents,
    SegregatedClientFunds,
    TradeReceivables,
    ContractAssets,
    PrepaidExpenses,
    DeferredTaxAssets,

    // Liabilities (IFRS 9)
    ClientMoneyLiabilities,
    TradePayables,
    AccruedExpenses,
    ContractLiabilities,
    DeferredRevenue,
    DeferredTaxLiabilities,

    // Equity (IAS 1)
    ShareCapital,
    RetainedEarnings,
    OtherComprehensiveIncome,

    // Revenue (IFRS 15)
    FxConversionRevenue,
    TransactionFeeRevenue,
    InterestIncome,
    OtherOperatingIncome,

    // Expenses (IAS 1)
    BankCharges,
    SettlementCosts,
    TechnologyExpenses,
    ComplianceCosts,
    PersonnelExpenses,
    DepreciationAmortization,
    OtherOperatingExpenses,

    // Off-balance sheet
    ContingentLiabilities,
    Commitments,
}

/// IFRS Statement Type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IfrsStatementType {
    /// Statement of Financial Position (Balance Sheet)
    StatementOfFinancialPosition,
    /// Statement of Comprehensive Income (P&L)
    StatementOfComprehensiveIncome,
    /// Statement of Cash Flows
    StatementOfCashFlows,
    /// Statement of Changes in Equity
    StatementOfChangesInEquity,
    /// Notes to Financial Statements
    Notes,
}

/// Subledger transaction type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubledgerTransaction {
    pub transaction_id: Uuid,
    pub transaction_date: DateTime<Utc>,
    pub account_code: String,
    pub debit_amount: Decimal,
    pub credit_amount: Decimal,
    pub currency: String,
    pub description: String,
    pub reference: String,
}

/// IFRS Line Item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfrsLineItem {
    pub ifrs_account: IfrsAccount,
    pub statement_type: IfrsStatementType,
    pub amount: Decimal,
    pub currency: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub source_transactions: Vec<Uuid>,
    pub notes: Option<String>,
}

/// IFRS Report (complete financial statement)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfrsReport {
    pub report_id: Uuid,
    pub entity_name: String,
    pub reporting_period: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub currency: String,
    pub line_items: Vec<IfrsLineItem>,
    pub generated_at: DateTime<Utc>,
    pub prepared_by: String,
    pub approved_by: Option<String>,
}

/// IFRS Mapper Service
pub struct IfrsMapper {
    pool: PgPool,
    account_mapping: HashMap<String, IfrsAccount>,
}

impl IfrsMapper {
    /// Create new IFRS mapper
    pub fn new(pool: PgPool) -> Self {
        let account_mapping = Self::build_account_mapping();

        Self {
            pool,
            account_mapping,
        }
    }

    /// Build mapping from subledger account codes to IFRS accounts
    fn build_account_mapping() -> HashMap<String, IfrsAccount> {
        let mut mapping = HashMap::new();

        // Assets
        mapping.insert("1000".to_string(), IfrsAccount::CashAndCashEquivalents);
        mapping.insert("1010".to_string(), IfrsAccount::SegregatedClientFunds);
        mapping.insert("1100".to_string(), IfrsAccount::TradeReceivables);
        mapping.insert("1110".to_string(), IfrsAccount::ContractAssets);
        mapping.insert("1200".to_string(), IfrsAccount::PrepaidExpenses);
        mapping.insert("1300".to_string(), IfrsAccount::DeferredTaxAssets);

        // Liabilities
        mapping.insert("2000".to_string(), IfrsAccount::ClientMoneyLiabilities);
        mapping.insert("2100".to_string(), IfrsAccount::TradePayables);
        mapping.insert("2110".to_string(), IfrsAccount::AccruedExpenses);
        mapping.insert("2200".to_string(), IfrsAccount::ContractLiabilities);
        mapping.insert("2210".to_string(), IfrsAccount::DeferredRevenue);
        mapping.insert("2300".to_string(), IfrsAccount::DeferredTaxLiabilities);

        // Equity
        mapping.insert("3000".to_string(), IfrsAccount::ShareCapital);
        mapping.insert("3100".to_string(), IfrsAccount::RetainedEarnings);
        mapping.insert("3200".to_string(), IfrsAccount::OtherComprehensiveIncome);

        // Revenue
        mapping.insert("4000".to_string(), IfrsAccount::FxConversionRevenue);
        mapping.insert("4100".to_string(), IfrsAccount::TransactionFeeRevenue);
        mapping.insert("4200".to_string(), IfrsAccount::InterestIncome);
        mapping.insert("4900".to_string(), IfrsAccount::OtherOperatingIncome);

        // Expenses
        mapping.insert("5000".to_string(), IfrsAccount::BankCharges);
        mapping.insert("5100".to_string(), IfrsAccount::SettlementCosts);
        mapping.insert("5200".to_string(), IfrsAccount::TechnologyExpenses);
        mapping.insert("5300".to_string(), IfrsAccount::ComplianceCosts);
        mapping.insert("5400".to_string(), IfrsAccount::PersonnelExpenses);
        mapping.insert("5500".to_string(), IfrsAccount::DepreciationAmortization);
        mapping.insert("5900".to_string(), IfrsAccount::OtherOperatingExpenses);

        // Off-balance sheet
        mapping.insert("9000".to_string(), IfrsAccount::ContingentLiabilities);
        mapping.insert("9100".to_string(), IfrsAccount::Commitments);

        mapping
    }

    /// Generate IFRS report for a period
    pub async fn generate_ifrs_report(
        &self,
        entity_name: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        currency: &str,
        prepared_by: &str,
    ) -> Result<IfrsReport, IfrsMapperError> {
        info!(
            entity = entity_name,
            period_start = %period_start,
            period_end = %period_end,
            "Generating IFRS report"
        );

        // Fetch subledger transactions
        let transactions = self.fetch_subledger_transactions(period_start, period_end).await?;

        // Map to IFRS line items
        let line_items = self.map_to_ifrs_line_items(transactions, period_start, period_end, currency)?;

        // Validate report
        self.validate_report(&line_items)?;

        let report_id = Uuid::new_v4();
        let reporting_period = format!(
            "{} to {}",
            period_start.format("%Y-%m-%d"),
            period_end.format("%Y-%m-%d")
        );

        let report = IfrsReport {
            report_id,
            entity_name: entity_name.to_string(),
            reporting_period,
            period_start,
            period_end,
            currency: currency.to_string(),
            line_items,
            generated_at: Utc::now(),
            prepared_by: prepared_by.to_string(),
            approved_by: None,
        };

        // Store report in database
        self.store_ifrs_report(&report).await?;

        info!(
            report_id = %report_id,
            line_items_count = report.line_items.len(),
            "IFRS report generated successfully"
        );

        Ok(report)
    }

    /// Fetch subledger transactions for period
    async fn fetch_subledger_transactions(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Vec<SubledgerTransaction>, IfrsMapperError> {
        let rows = sqlx::query(
            r#"
            SELECT
                event_id as transaction_id,
                event_timestamp as transaction_date,
                (event_data->>'account_code')::TEXT as account_code,
                COALESCE((event_data->>'debit_amount')::DECIMAL, 0) as debit_amount,
                COALESCE((event_data->>'credit_amount')::DECIMAL, 0) as credit_amount,
                (event_data->>'currency')::TEXT as currency,
                (event_data->>'description')::TEXT as description,
                (event_data->>'reference')::TEXT as reference
            FROM bronze.tx_events
            WHERE event_timestamp >= $1
              AND event_timestamp < $2
              AND event_type = 'subledger_entry'
            ORDER BY event_timestamp
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await?;

        let transactions = rows
            .into_iter()
            .map(|row| SubledgerTransaction {
                transaction_id: row.get("transaction_id"),
                transaction_date: row.get("transaction_date"),
                account_code: row.get("account_code"),
                debit_amount: row.get("debit_amount"),
                credit_amount: row.get("credit_amount"),
                currency: row.get("currency"),
                description: row.get("description"),
                reference: row.get("reference"),
            })
            .collect();

        Ok(transactions)
    }

    /// Map subledger transactions to IFRS line items
    fn map_to_ifrs_line_items(
        &self,
        transactions: Vec<SubledgerTransaction>,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        currency: &str,
    ) -> Result<Vec<IfrsLineItem>, IfrsMapperError> {
        // Group transactions by IFRS account
        let mut account_groups: HashMap<IfrsAccount, Vec<SubledgerTransaction>> = HashMap::new();

        for tx in transactions {
            let ifrs_account = self
                .account_mapping
                .get(&tx.account_code)
                .ok_or_else(|| {
                    IfrsMapperError::MappingError(format!(
                        "No IFRS mapping for account code: {}",
                        tx.account_code
                    ))
                })?;

            account_groups.entry(*ifrs_account).or_insert_with(Vec::new).push(tx);
        }

        // Create line items
        let mut line_items = Vec::new();

        for (ifrs_account, txs) in account_groups {
            let net_amount: Decimal = txs
                .iter()
                .map(|tx| tx.debit_amount - tx.credit_amount)
                .sum();

            let source_transactions: Vec<Uuid> = txs.iter().map(|tx| tx.transaction_id).collect();

            let statement_type = Self::get_statement_type(ifrs_account);

            line_items.push(IfrsLineItem {
                ifrs_account,
                statement_type,
                amount: net_amount,
                currency: currency.to_string(),
                period_start,
                period_end,
                source_transactions,
                notes: None,
            });
        }

        Ok(line_items)
    }

    /// Get IFRS statement type for an account
    fn get_statement_type(account: IfrsAccount) -> IfrsStatementType {
        match account {
            IfrsAccount::CashAndCashEquivalents
            | IfrsAccount::SegregatedClientFunds
            | IfrsAccount::TradeReceivables
            | IfrsAccount::ContractAssets
            | IfrsAccount::PrepaidExpenses
            | IfrsAccount::DeferredTaxAssets
            | IfrsAccount::ClientMoneyLiabilities
            | IfrsAccount::TradePayables
            | IfrsAccount::AccruedExpenses
            | IfrsAccount::ContractLiabilities
            | IfrsAccount::DeferredRevenue
            | IfrsAccount::DeferredTaxLiabilities
            | IfrsAccount::ShareCapital
            | IfrsAccount::RetainedEarnings
            | IfrsAccount::OtherComprehensiveIncome => {
                IfrsStatementType::StatementOfFinancialPosition
            }

            IfrsAccount::FxConversionRevenue
            | IfrsAccount::TransactionFeeRevenue
            | IfrsAccount::InterestIncome
            | IfrsAccount::OtherOperatingIncome
            | IfrsAccount::BankCharges
            | IfrsAccount::SettlementCosts
            | IfrsAccount::TechnologyExpenses
            | IfrsAccount::ComplianceCosts
            | IfrsAccount::PersonnelExpenses
            | IfrsAccount::DepreciationAmortization
            | IfrsAccount::OtherOperatingExpenses => {
                IfrsStatementType::StatementOfComprehensiveIncome
            }

            IfrsAccount::ContingentLiabilities | IfrsAccount::Commitments => {
                IfrsStatementType::Notes
            }
        }
    }

    /// Validate IFRS report
    fn validate_report(&self, line_items: &[IfrsLineItem]) -> Result<(), IfrsMapperError> {
        // Check balance sheet equation: Assets = Liabilities + Equity
        let assets: Decimal = line_items
            .iter()
            .filter(|li| matches!(
                li.ifrs_account,
                IfrsAccount::CashAndCashEquivalents
                    | IfrsAccount::SegregatedClientFunds
                    | IfrsAccount::TradeReceivables
                    | IfrsAccount::ContractAssets
                    | IfrsAccount::PrepaidExpenses
                    | IfrsAccount::DeferredTaxAssets
            ))
            .map(|li| li.amount)
            .sum();

        let liabilities: Decimal = line_items
            .iter()
            .filter(|li| matches!(
                li.ifrs_account,
                IfrsAccount::ClientMoneyLiabilities
                    | IfrsAccount::TradePayables
                    | IfrsAccount::AccruedExpenses
                    | IfrsAccount::ContractLiabilities
                    | IfrsAccount::DeferredRevenue
                    | IfrsAccount::DeferredTaxLiabilities
            ))
            .map(|li| li.amount)
            .sum();

        let equity: Decimal = line_items
            .iter()
            .filter(|li| matches!(
                li.ifrs_account,
                IfrsAccount::ShareCapital
                    | IfrsAccount::RetainedEarnings
                    | IfrsAccount::OtherComprehensiveIncome
            ))
            .map(|li| li.amount)
            .sum();

        let difference = assets - (liabilities + equity);
        let tolerance = Decimal::new(1, 2); // 0.01 tolerance

        if difference.abs() > tolerance {
            return Err(IfrsMapperError::ValidationError(format!(
                "Balance sheet does not balance: Assets={}, Liabilities={}, Equity={}, Difference={}",
                assets, liabilities, equity, difference
            )));
        }

        Ok(())
    }

    /// Store IFRS report in database
    async fn store_ifrs_report(&self, report: &IfrsReport) -> Result<(), IfrsMapperError> {
        sqlx::query(
            r#"
            INSERT INTO compliance.ifrs_reports
            (report_id, entity_name, reporting_period, period_start, period_end,
             currency, line_items, generated_at, prepared_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(report.report_id)
        .bind(&report.entity_name)
        .bind(&report.reporting_period)
        .bind(report.period_start)
        .bind(report.period_end)
        .bind(&report.currency)
        .bind(serde_json::to_value(&report.line_items).unwrap())
        .bind(report.generated_at)
        .bind(&report.prepared_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Export IFRS report to JSON
    pub fn export_to_json(&self, report: &IfrsReport) -> Result<String, IfrsMapperError> {
        serde_json::to_string_pretty(report)
            .map_err(|e| IfrsMapperError::MappingError(format!("JSON export failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_mapping() {
        let mapper = IfrsMapper::new(PgPool::connect_lazy("postgres://localhost").unwrap());
        assert_eq!(
            mapper.account_mapping.get("1000"),
            Some(&IfrsAccount::CashAndCashEquivalents)
        );
        assert_eq!(
            mapper.account_mapping.get("4000"),
            Some(&IfrsAccount::FxConversionRevenue)
        );
    }

    #[test]
    fn test_statement_type_classification() {
        assert_eq!(
            IfrsMapper::get_statement_type(IfrsAccount::CashAndCashEquivalents),
            IfrsStatementType::StatementOfFinancialPosition
        );
        assert_eq!(
            IfrsMapper::get_statement_type(IfrsAccount::FxConversionRevenue),
            IfrsStatementType::StatementOfComprehensiveIncome
        );
    }
}
