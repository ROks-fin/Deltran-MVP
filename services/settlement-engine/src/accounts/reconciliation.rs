use crate::config::Config;
use crate::error::Result;
use crate::integration::BankClientManager;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub id: Uuid,
    pub report_date: DateTime<Utc>,
    pub total_accounts: i32,
    pub balanced_accounts: i32,
    pub discrepancy_accounts: serde_json::Value,
    pub total_discrepancy: Decimal,
    pub discrepancies: Vec<AccountDiscrepancy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDiscrepancy {
    pub account_id: Uuid,
    pub account_type: AccountType,
    pub bank: String,
    pub currency: String,
    pub internal_balance: Decimal,
    pub external_balance: Decimal,
    pub discrepancy: Decimal,
    pub status: ReconciliationStatus,
    pub unmatched_transactions: Vec<UnmatchedTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountType {
    Nostro,
    Vostro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReconciliationStatus {
    Balanced,
    Unresolved,
    Identified,
    InvestigationRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmatchedTransaction {
    pub transaction_id: String,
    pub amount: Decimal,
    pub timestamp: DateTime<Utc>,
    pub source: TransactionSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionSource {
    Internal,
    External,
}

pub struct ReconciliationEngine {
    db_pool: Arc<PgPool>,
    bank_clients: Arc<BankClientManager>,
    config: Arc<Config>,
}

impl ReconciliationEngine {
    pub fn new(
        db_pool: Arc<PgPool>,
        bank_clients: Arc<BankClientManager>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            db_pool,
            bank_clients,
            config,
        }
    }

    pub async fn reconcile_all_accounts(&self) -> Result<ReconciliationReport> {
        info!("Starting reconciliation for all accounts");

        let report_id = Uuid::new_v4();
        let report_date = Utc::now();

        let mut total_accounts = 0;
        let mut balanced_accounts = 0;
        let mut discrepancy_accounts = 0;
        let mut total_discrepancy = Decimal::ZERO;
        let mut discrepancies = Vec::new();

        // Reconcile all nostro accounts
        let nostro_accounts = sqlx::query(
            r#"
            SELECT id, bank, currency, ledger_balance
            FROM nostro_accounts
            WHERE is_active = true
            "#
        )
        .fetch_all(&*self.db_pool)
        .await?;

        for account in &nostro_accounts {
            total_accounts += 1;

            let account_id: Uuid = account.try_get("id")?;
            let bank: String = account.try_get("bank")?;
            let currency: String = account.try_get("currency")?;
            let ledger_balance: Decimal = account.try_get("ledger_balance")?;

            match self
                .reconcile_nostro_account(account_id, &bank, &currency)
                .await
            {
                Ok(Some(discrepancy)) => {
                    discrepancy_accounts += 1;
                    total_discrepancy += discrepancy.discrepancy.abs();
                    discrepancies.push(discrepancy);
                }
                Ok(None) => {
                    balanced_accounts += 1;
                }
                Err(e) => {
                    error!(
                        "Error reconciling nostro account {} {}: {}",
                        bank, currency, e
                    );
                    discrepancy_accounts += 1;
                    discrepancies.push(AccountDiscrepancy {
                        account_id,
                        account_type: AccountType::Nostro,
                        bank: bank.clone(),
                        currency: currency.clone(),
                        internal_balance: ledger_balance,
                        external_balance: Decimal::ZERO,
                        discrepancy: ledger_balance,
                        status: ReconciliationStatus::InvestigationRequired,
                        unmatched_transactions: vec![],
                    });
                }
            }
        }

        // Reconcile all vostro accounts (simpler - no external balance check for MVP)
        let vostro_accounts = sqlx::query(
            r#"
            SELECT id, bank, currency, ledger_balance
            FROM vostro_accounts
            WHERE is_active = true
            "#
        )
        .fetch_all(&*self.db_pool)
        .await?;

        for _account in &vostro_accounts {
            total_accounts += 1;
            // For MVP, assume vostro accounts are always balanced internally
            balanced_accounts += 1;
        }

        // Store report
        let report = ReconciliationReport {
            id: report_id,
            report_date,
            total_accounts,
            balanced_accounts,
            discrepancy_accounts: serde_json::json!(discrepancy_accounts),
            total_discrepancy,
            discrepancies: discrepancies.clone(),
        };

        self.store_report(&report).await?;

        // Alert on significant discrepancies
        let alert_threshold = Decimal::from_str(&self.config.reconciliation.alert_threshold)?;
        if total_discrepancy > alert_threshold {
            warn!(
                "ALERT: Total discrepancy {} exceeds threshold {}",
                total_discrepancy, alert_threshold
            );
        }

        info!(
            "Reconciliation complete: {} accounts, {} balanced, {} with discrepancies",
            total_accounts, balanced_accounts, discrepancy_accounts
        );

        Ok(report)
    }

    async fn reconcile_nostro_account(
        &self,
        account_id: Uuid,
        bank: &str,
        currency: &str,
    ) -> Result<Option<AccountDiscrepancy>> {
        // Get internal balance
        let row = sqlx::query(
            r#"
            SELECT ledger_balance
            FROM nostro_accounts
            WHERE id = $1
            "#
        )
        .bind(account_id)
        .fetch_one(&*self.db_pool)
        .await?;

        let internal_balance: Decimal = row.try_get("ledger_balance")?;

        // For MVP with mock banks, simulate external balance
        // In production, this would query the actual bank API
        let external_balance = self.get_external_balance(bank, currency).await?;

        // Calculate discrepancy
        let discrepancy = internal_balance - external_balance;
        let tolerance = Decimal::from_str(&self.config.reconciliation.tolerance_amount)?;

        if discrepancy.abs() <= tolerance {
            // Update reconciliation timestamp
            sqlx::query(
                r#"
                UPDATE nostro_accounts
                SET last_reconciled = $1
                WHERE id = $2
                "#
            )
            .bind(Utc::now())
            .bind(account_id)
            .execute(&*self.db_pool)
            .await?;

            return Ok(None);
        }

        // Find unmatched transactions
        let unmatched = self
            .find_unmatched_transactions(account_id, &internal_balance, &external_balance)
            .await?;

        let status = if unmatched.is_empty() {
            ReconciliationStatus::Unresolved
        } else {
            ReconciliationStatus::Identified
        };

        Ok(Some(AccountDiscrepancy {
            account_id,
            account_type: AccountType::Nostro,
            bank: bank.to_string(),
            currency: currency.to_string(),
            internal_balance,
            external_balance,
            discrepancy,
            status,
            unmatched_transactions: unmatched,
        }))
    }

    async fn get_external_balance(&self, bank: &str, currency: &str) -> Result<Decimal> {
        // For MVP with mock implementation
        // In production, this would call the actual bank API
        info!(
            "Getting external balance for bank {} currency {} (mock)",
            bank, currency
        );

        // Simulate: return internal balance with small random variance
        let internal = sqlx::query(
            r#"
            SELECT ledger_balance
            FROM nostro_accounts
            WHERE bank = $1 AND currency = $2
            "#
        )
        .bind(bank)
        .bind(currency)
        .fetch_optional(&*self.db_pool)
        .await?;

        match internal {
            Some(record) => {
                // For MVP, return exact balance (no discrepancy)
                let ledger_balance: Decimal = record.try_get("ledger_balance")?;
                Ok(ledger_balance)
            }
            None => Ok(Decimal::ZERO),
        }
    }

    async fn find_unmatched_transactions(
        &self,
        account_id: Uuid,
        internal_balance: &Decimal,
        external_balance: &Decimal,
    ) -> Result<Vec<UnmatchedTransaction>> {
        // Find recent transactions for this account
        let transactions = sqlx::query(
            r#"
            SELECT
                st.id,
                st.amount,
                st.created_at,
                st.status
            FROM settlement_transactions st
            JOIN fund_locks fl ON fl.settlement_id = st.id
            WHERE fl.nostro_account_id = $1
                AND st.created_at > NOW() - INTERVAL '7 days'
            ORDER BY st.created_at DESC
            LIMIT 100
            "#
        )
        .bind(account_id)
        .fetch_all(&*self.db_pool)
        .await?;

        let mut unmatched = Vec::new();

        for txn in &transactions {
            let txn_id: Uuid = txn.try_get("id")?;
            let amount: Decimal = txn.try_get("amount")?;
            let created_at: DateTime<Utc> = txn.try_get("created_at")?;
            let status: String = txn.try_get("status")?;

            // Look for transactions that might explain the discrepancy
            if status == "PENDING" || status == "EXECUTING" {
                unmatched.push(UnmatchedTransaction {
                    transaction_id: txn_id.to_string(),
                    amount,
                    timestamp: created_at,
                    source: TransactionSource::Internal,
                });
            }
        }

        Ok(unmatched)
    }

    async fn store_report(&self, report: &ReconciliationReport) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO reconciliation_reports (
                id, report_date, total_accounts, balanced_accounts,
                discrepancy_accounts, total_discrepancy, details
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        )
        .bind(report.id)
        .bind(report.report_date.date_naive())
        .bind(report.total_accounts)
        .bind(report.balanced_accounts)
        .bind(&report.discrepancy_accounts)
        .bind(report.total_discrepancy)
        .bind(serde_json::to_value(&report.discrepancies)?)
        .execute(&*self.db_pool)
        .await?;

        info!("Stored reconciliation report {}", report.id);

        Ok(())
    }

    pub async fn get_latest_report(&self) -> Result<Option<ReconciliationReport>> {
        let record = sqlx::query(
            r#"
            SELECT
                id, report_date, total_accounts, balanced_accounts,
                discrepancy_accounts, total_discrepancy, details
            FROM reconciliation_reports
            ORDER BY report_date DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&*self.db_pool)
        .await?;

        match record {
            Some(r) => {
                let id: Uuid = r.try_get("id")?;
                let report_date: chrono::NaiveDate = r.try_get("report_date")?;
                let total_accounts: i32 = r.try_get("total_accounts")?;
                let balanced_accounts: i32 = r.try_get("balanced_accounts")?;
                let discrepancy_accounts: serde_json::Value = r.try_get("discrepancy_accounts").unwrap_or(serde_json::json!(0));
                let total_discrepancy: Decimal = r.try_get("total_discrepancy")?;
                let details: Option<serde_json::Value> = r.try_get("details").ok();

                let discrepancies: Vec<AccountDiscrepancy> =
                    serde_json::from_value(details.unwrap_or(serde_json::json!([])))?;

                Ok(Some(ReconciliationReport {
                    id,
                    report_date: report_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    total_accounts,
                    balanced_accounts,
                    discrepancy_accounts,
                    total_discrepancy,
                    discrepancies,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn run_scheduled_reconciliation(&self) -> Result<()> {
        info!("Running scheduled reconciliation");

        match self.reconcile_all_accounts().await {
            Ok(report) => {
                info!(
                    "Scheduled reconciliation completed: {} discrepancies found",
                    report.discrepancy_accounts.as_i64().unwrap_or(0)
                );
                Ok(())
            }
            Err(e) => {
                error!("Scheduled reconciliation failed: {}", e);
                Err(e)
            }
        }
    }
}
