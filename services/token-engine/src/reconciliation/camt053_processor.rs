// CAMT.053 Processor - EOD (End-of-Day) Full Reconciliation
// Processes BankToCustomerStatement for daily reconciliation

use crate::errors::{Result, TokenEngineError};
use crate::reconciliation::{DiscrepancyDetector, ThresholdChecker};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Simplified CAMT.053 statement structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camt053Statement {
    pub message_id: String,
    pub creation_date_time: String,
    pub account_id: String,
    pub currency: String,
    pub statement_date: String,
    pub opening_balance: String,
    pub closing_balance: String,
    pub entries: Vec<Camt053Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camt053Entry {
    pub entry_reference: String,
    pub credit_debit_indicator: String,
    pub amount: String,
    pub booking_date: Option<String>,
    pub value_date: Option<String>,
    pub bank_reference: Option<String>,
    pub end_to_end_id: Option<String>,
}

pub struct Camt053Processor {
    pool: PgPool,
}

impl Camt053Processor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Process EOD statement and create snapshot
    pub async fn process_statement(
        &self,
        statement: Camt053Statement,
    ) -> Result<EodReconciliationResult> {
        info!(
            "Processing CAMT.053 EOD statement {} for account {} on {}",
            statement.message_id, statement.account_id, statement.statement_date
        );

        // Find EMI account
        let emi_account = self.find_emi_account(&statement.account_id).await?;

        // Parse closing balance from bank
        let bank_closing_balance = Decimal::from_str(&statement.closing_balance)
            .map_err(|e| TokenEngineError::Validation(format!("Invalid closing balance: {}", e)))?;

        // Get ledger balance
        let ledger_balance = self.get_ledger_balance(emi_account.id).await?;
        let reserved_balance = self.get_reserved_balance(emi_account.id).await?;
        let available_balance = ledger_balance - reserved_balance;

        info!(
            "EOD Balances for account {}: ledger={}, bank={}, reserved={}, available={}",
            emi_account.id,
            ledger_balance,
            bank_closing_balance,
            reserved_balance,
            available_balance
        );

        // Check threshold
        let threshold_result = ThresholdChecker::check(ledger_balance, bank_closing_balance);

        // Parse statement date
        let statement_date = NaiveDate::parse_from_str(&statement.statement_date, "%Y-%m-%d")
            .map_err(|e| TokenEngineError::Validation(format!("Invalid date format: {}", e)))?;

        // Create EOD snapshot
        let snapshot = self.create_eod_snapshot(
            emi_account.id,
            statement_date,
            ledger_balance,
            bank_closing_balance,
            reserved_balance,
            available_balance,
            &statement.message_id,
            threshold_result.level == crate::reconciliation::threshold_checker::ThresholdLevel::Ok,
        ).await?;

        info!("Created EOD snapshot {} for date {}", snapshot.id, statement_date);

        // Update EMI account with EOD reconciliation
        self.update_bank_reported_balance(emi_account.id, bank_closing_balance).await?;

        let recon_status = if threshold_result.level == crate::reconciliation::threshold_checker::ThresholdLevel::Ok {
            "OK"
        } else {
            "MISMATCH"
        };

        self.update_reconciliation_status(
            emi_account.id,
            recon_status,
            "CAMT_053",
            Some(&statement.message_id),
            threshold_result.absolute_difference,
        ).await?;

        // Create discrepancy if needed
        if threshold_result.level != crate::reconciliation::threshold_checker::ThresholdLevel::Ok {
            warn!(
                "EOD balance mismatch for account {}: ledger={}, bank={}, diff={}",
                emi_account.id,
                ledger_balance,
                bank_closing_balance,
                threshold_result.absolute_difference
            );

            let threshold_exceeded = ThresholdChecker::should_suspend_payouts(&threshold_result);

            DiscrepancyDetector::create_balance_mismatch(
                &self.pool,
                emi_account.id,
                ledger_balance,
                bank_closing_balance,
                "CAMT_053",
                Some(&statement.message_id),
                threshold_exceeded,
            ).await?;

            if ThresholdChecker::should_activate_circuit_breaker(&threshold_result) {
                error!(
                    "CRITICAL EOD mismatch - activating circuit breaker for account {}",
                    emi_account.id
                );
                self.activate_circuit_breaker(emi_account.id).await?;
            }
        }

        // Process individual entries for transaction matching
        let mut matched = 0;
        let mut unmatched = 0;

        for entry in &statement.entries {
            let is_matched = self.match_transaction_entry(
                emi_account.id,
                entry,
            ).await?;

            if is_matched {
                matched += 1;
            } else {
                unmatched += 1;
            }
        }

        info!(
            "EOD statement processing complete: {} entries matched, {} unmatched",
            matched, unmatched
        );

        Ok(EodReconciliationResult {
            snapshot_id: snapshot.id,
            account_id: emi_account.id,
            statement_date,
            ledger_balance,
            bank_balance: bank_closing_balance,
            difference: threshold_result.absolute_difference,
            reconciled: threshold_result.level == crate::reconciliation::threshold_checker::ThresholdLevel::Ok,
            entries_matched: matched,
            entries_unmatched: unmatched,
            threshold_level: threshold_result.level,
        })
    }

    /// Create EOD snapshot
    async fn create_eod_snapshot(
        &self,
        account_id: Uuid,
        snapshot_date: NaiveDate,
        ledger_balance: Decimal,
        bank_balance: Decimal,
        reserved_balance: Decimal,
        available_balance: Decimal,
        statement_reference: &str,
        reconciled: bool,
    ) -> Result<EmiAccountSnapshot> {
        let difference = (ledger_balance - bank_balance).abs();

        let snapshot = sqlx::query_as::<_, EmiAccountSnapshot>(
            r#"
            INSERT INTO emi_account_snapshots (
                id, account_id, snapshot_date, snapshot_time,
                ledger_balance, bank_reported_balance, reserved_balance, available_balance,
                difference, reconciled, statement_reference, snapshot_data, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (account_id, snapshot_date)
            DO UPDATE SET
                snapshot_time = $4,
                ledger_balance = $5,
                bank_reported_balance = $6,
                reserved_balance = $7,
                available_balance = $8,
                difference = $9,
                reconciled = $10,
                statement_reference = $11,
                snapshot_data = $12
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(account_id)
        .bind(snapshot_date)
        .bind(Utc::now())
        .bind(ledger_balance)
        .bind(bank_balance)
        .bind(reserved_balance)
        .bind(available_balance)
        .bind(difference)
        .bind(reconciled)
        .bind(statement_reference)
        .bind(serde_json::json!({
            "processing_timestamp": Utc::now(),
            "reconciliation_type": "EOD_CAMT_053"
        }))
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(snapshot)
    }

    /// Match transaction entry with internal records
    async fn match_transaction_entry(
        &self,
        account_id: Uuid,
        entry: &Camt053Entry,
    ) -> Result<bool> {
        // Try to match by UETR (end-to-end ID)
        if let Some(uetr) = &entry.end_to_end_id {
            let exists: (bool,) = sqlx::query_as(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM emi_transactions
                    WHERE account_id = $1 AND uetr = $2
                )
                "#,
            )
            .bind(account_id)
            .bind(uetr)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| TokenEngineError::Database(e))?;

            if exists.0 {
                return Ok(true);
            }
        }

        // Try to match by bank reference
        if let Some(bank_ref) = &entry.bank_reference {
            let exists: (bool,) = sqlx::query_as(
                r#"
                SELECT EXISTS(
                    SELECT 1 FROM emi_transactions
                    WHERE account_id = $1 AND bank_reference = $2
                )
                "#,
            )
            .bind(account_id)
            .bind(bank_ref)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| TokenEngineError::Database(e))?;

            if exists.0 {
                return Ok(true);
            }
        }

        // Unmatched transaction - could be manual intervention, bank fee, etc.
        warn!(
            "Unmatched transaction in EOD statement: entry_ref={}, amount={}",
            entry.entry_reference, entry.amount
        );

        Ok(false)
    }

    // Helper functions (reused from camt054_processor)
    async fn find_emi_account(&self, account_identifier: &str) -> Result<EmiAccount> {
        let account = sqlx::query_as::<_, EmiAccount>(
            r#"
            SELECT id, account_number, iban, currency
            FROM emi_accounts
            WHERE account_number = $1 OR iban = $1
            LIMIT 1
            "#,
        )
        .bind(account_identifier)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?
        .ok_or_else(|| TokenEngineError::Internal(format!(
            "EMI account not found: {}",
            account_identifier
        )))?;

        Ok(account)
    }

    async fn get_ledger_balance(&self, account_id: Uuid) -> Result<Decimal> {
        let row: (Decimal,) = sqlx::query_as(
            "SELECT ledger_balance FROM emi_accounts WHERE id = $1"
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(row.0)
    }

    async fn get_reserved_balance(&self, account_id: Uuid) -> Result<Decimal> {
        let row: (Decimal,) = sqlx::query_as(
            "SELECT reserved_balance FROM emi_accounts WHERE id = $1"
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(row.0)
    }

    async fn update_bank_reported_balance(&self, account_id: Uuid, balance: Decimal) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE emi_accounts
            SET bank_reported_balance = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(balance)
        .bind(account_id)
        .execute(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(())
    }

    async fn update_reconciliation_status(
        &self,
        account_id: Uuid,
        status: &str,
        source: &str,
        reference: Option<&str>,
        difference: Decimal,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE emi_accounts
            SET reconciliation_status = $1,
                last_reconciliation_at = $2,
                reconciliation_source = $3,
                reconciliation_difference = $4,
                updated_at = NOW()
            WHERE id = $5
            "#,
        )
        .bind(status)
        .bind(Utc::now())
        .bind(source)
        .bind(difference)
        .bind(account_id)
        .execute(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(())
    }

    async fn activate_circuit_breaker(&self, account_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE emi_accounts
            SET metadata = jsonb_set(
                COALESCE(metadata, '{}'::jsonb),
                '{circuit_breaker_active}',
                'true'
            ),
            updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(account_id)
        .execute(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EmiAccount {
    id: Uuid,
    account_number: String,
    iban: Option<String>,
    currency: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EmiAccountSnapshot {
    id: Uuid,
    account_id: Uuid,
    snapshot_date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct EodReconciliationResult {
    pub snapshot_id: Uuid,
    pub account_id: Uuid,
    pub statement_date: NaiveDate,
    pub ledger_balance: Decimal,
    pub bank_balance: Decimal,
    pub difference: Decimal,
    pub reconciled: bool,
    pub entries_matched: i32,
    pub entries_unmatched: i32,
    pub threshold_level: crate::reconciliation::threshold_checker::ThresholdLevel,
}
