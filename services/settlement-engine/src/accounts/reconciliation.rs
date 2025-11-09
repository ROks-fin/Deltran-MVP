use crate::config::Config;
use crate::error::Result;
use crate::integration::BankClientManager;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
    pub discrepancy_accounts: i32,
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
        let nostro_accounts = sqlx::query!(
            r#"
            SELECT id, bank, currency, ledger_balance
            FROM nostro_accounts
            WHERE is_active = true
            "#
        )
        .fetch_all(&*self.db_pool)
        .await?;

        for account in nostro_accounts {
            total_accounts += 1;

            match self
                .reconcile_nostro_account(account.id, &account.bank, &account.currency)
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
                        account.bank, account.currency, e
                    );
                    discrepancy_accounts += 1;
                    discrepancies.push(AccountDiscrepancy {
                        account_id: account.id,
                        account_type: AccountType::Nostro,
                        bank: account.bank.clone(),
                        currency: account.currency.clone(),
                        internal_balance: account.ledger_balance,
                        external_balance: Decimal::ZERO,
                        discrepancy: account.ledger_balance,
                        status: ReconciliationStatus::InvestigationRequired,
                        unmatched_transactions: vec![],
                    });
                }
            }
        }

        // Reconcile all vostro accounts (simpler - no external balance check for MVP)
        let vostro_accounts = sqlx::query!(
            r#"
            SELECT id, bank, currency, ledger_balance
            FROM vostro_accounts
            WHERE is_active = true
            "#
        )
        .fetch_all(&*self.db_pool)
        .await?;

        for account in vostro_accounts {
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
            discrepancy_accounts,
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
        let internal_balance = sqlx::query!(
            r#"
            SELECT ledger_balance
            FROM nostro_accounts
            WHERE id = $1
            "#,
            account_id
        )
        .fetch_one(&*self.db_pool)
        .await?
        .ledger_balance;

        // For MVP with mock banks, simulate external balance
        // In production, this would query the actual bank API
        let external_balance = self.get_external_balance(bank, currency).await?;

        // Calculate discrepancy
        let discrepancy = internal_balance - external_balance;
        let tolerance = Decimal::from_str(&self.config.reconciliation.tolerance_amount)?;

        if discrepancy.abs() <= tolerance {
            // Update reconciliation timestamp
            sqlx::query!(
                r#"
                UPDATE nostro_accounts
                SET last_reconciled = $1
                WHERE id = $2
                "#,
                Utc::now(),
                account_id
            )
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
        let internal = sqlx::query!(
            r#"
            SELECT ledger_balance
            FROM nostro_accounts
            WHERE bank = $1 AND currency = $2
            "#,
            bank,
            currency
        )
        .fetch_optional(&*self.db_pool)
        .await?;

        match internal {
            Some(record) => {
                // For MVP, return exact balance (no discrepancy)
                Ok(record.ledger_balance)
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
        let transactions = sqlx::query!(
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
            "#,
            account_id
        )
        .fetch_all(&*self.db_pool)
        .await?;

        let mut unmatched = Vec::new();

        for txn in transactions {
            // Look for transactions that might explain the discrepancy
            if txn.status == "PENDING" || txn.status == "EXECUTING" {
                unmatched.push(UnmatchedTransaction {
                    transaction_id: txn.id.to_string(),
                    amount: txn.amount,
                    timestamp: txn.created_at,
                    source: TransactionSource::Internal,
                });
            }
        }

        Ok(unmatched)
    }

    async fn store_report(&self, report: &ReconciliationReport) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO reconciliation_reports (
                id, report_date, total_accounts, balanced_accounts,
                discrepancy_accounts, total_discrepancy, details
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            report.id,
            report.report_date.date_naive(),
            report.total_accounts,
            report.balanced_accounts,
            report.discrepancy_accounts,
            report.total_discrepancy,
            serde_json::to_value(&report.discrepancies)?
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Stored reconciliation report {}", report.id);

        Ok(())
    }

    pub async fn get_latest_report(&self) -> Result<Option<ReconciliationReport>> {
        let record = sqlx::query!(
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
                let discrepancies: Vec<AccountDiscrepancy> =
                    serde_json::from_value(r.details.unwrap_or(serde_json::json!([])))?;

                Ok(Some(ReconciliationReport {
                    id: r.id,
                    report_date: r.report_date.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                    total_accounts: r.total_accounts,
                    balanced_accounts: r.balanced_accounts,
                    discrepancy_accounts: r.discrepancy_accounts,
                    total_discrepancy: r.total_discrepancy.unwrap_or(Decimal::ZERO),
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
                    report.discrepancy_accounts
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
