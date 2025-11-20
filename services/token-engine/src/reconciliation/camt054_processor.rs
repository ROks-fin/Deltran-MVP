// CAMT.054 Processor - Near Real-Time Reconciliation
// Processes BankToCustomerDebitCreditNotification messages

use crate::errors::{Result, TokenEngineError};
use crate::reconciliation::{DiscrepancyDetector, ThresholdChecker};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Simplified CAMT.054 structure for reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camt054Notification {
    pub message_id: String,
    pub creation_date_time: String,
    pub account_id: String,  // IBAN or account number
    pub currency: String,
    pub credit_debit_indicator: String,  // CRDT or DBIT
    pub amount: String,
    pub booking_date: Option<String>,
    pub value_date: Option<String>,
    pub bank_reference: Option<String>,
    pub end_to_end_id: Option<String>,
    pub transaction_id: Option<String>,
}

pub struct Camt054Processor {
    pool: PgPool,
}

impl Camt054Processor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Process incoming CAMT.054 notification and reconcile
    pub async fn process_notification(
        &self,
        notification: Camt054Notification,
    ) -> Result<ReconciliationResult> {
        info!(
            "Processing CAMT.054 notification {} for account {}",
            notification.message_id, notification.account_id
        );

        // Find corresponding EMI account
        let emi_account = self.find_emi_account(&notification.account_id).await?;

        // Parse amount
        let amount = Decimal::from_str(&notification.amount)
            .map_err(|e| TokenEngineError::Validation(format!("Invalid amount: {}", e)))?;

        // Determine if credit or debit
        let is_credit = notification.credit_debit_indicator == "CRDT";

        // Update bank_reported_balance
        let updated_balance = if is_credit {
            self.update_bank_balance(emi_account.id, amount).await?
        } else {
            self.update_bank_balance(emi_account.id, -amount).await?
        };

        info!(
            "Updated bank_reported_balance for account {}: {} {}",
            emi_account.id,
            updated_balance,
            notification.currency
        );

        // Get current ledger balance
        let ledger_balance = self.get_ledger_balance(emi_account.id).await?;

        // Check threshold
        let threshold_result = ThresholdChecker::check(ledger_balance, updated_balance);

        info!(
            "Reconciliation check: ledger={}, bank={}, diff={}, level={:?}",
            ledger_balance,
            updated_balance,
            threshold_result.absolute_difference,
            threshold_result.level
        );

        // Update reconciliation status
        let recon_status = match threshold_result.level {
            crate::reconciliation::threshold_checker::ThresholdLevel::Ok => "OK",
            _ => "MISMATCH",
        };

        self.update_reconciliation_status(
            emi_account.id,
            recon_status,
            "CAMT_054",
            Some(&notification.message_id),
            threshold_result.absolute_difference,
        ).await?;

        // Create discrepancy if threshold exceeded
        if threshold_result.level != crate::reconciliation::threshold_checker::ThresholdLevel::Ok {
            warn!(
                "Balance mismatch detected for account {}: {}",
                emi_account.id,
                threshold_result.action_required
            );

            let threshold_exceeded = ThresholdChecker::should_suspend_payouts(&threshold_result);

            DiscrepancyDetector::create_balance_mismatch(
                &self.pool,
                emi_account.id,
                ledger_balance,
                updated_balance,
                "CAMT_054",
                Some(&notification.message_id),
                threshold_exceeded,
            ).await?;

            // Activate circuit breaker if critical
            if ThresholdChecker::should_activate_circuit_breaker(&threshold_result) {
                error!(
                    "CRITICAL: Activating circuit breaker for account {}",
                    emi_account.id
                );
                self.activate_circuit_breaker(emi_account.id).await?;
            }
        }

        // Record EMI transaction
        self.record_emi_transaction(
            emi_account.id,
            if is_credit { "CREDIT" } else { "DEBIT" },
            amount,
            notification.bank_reference.as_deref(),
            notification.end_to_end_id.as_deref(),
            &notification.message_id,
        ).await?;

        Ok(ReconciliationResult {
            account_id: emi_account.id,
            ledger_balance,
            bank_balance: updated_balance,
            difference: threshold_result.absolute_difference,
            threshold_level: threshold_result.level,
            action_taken: threshold_result.action_required,
        })
    }

    /// Find EMI account by account number or IBAN
    async fn find_emi_account(&self, account_identifier: &str) -> Result<EmiAccount> {
        let account = sqlx::query_as::<_, EmiAccount>(
            r#"
            SELECT id, account_number, iban, currency, ledger_balance, bank_reported_balance
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

    /// Update bank_reported_balance
    async fn update_bank_balance(&self, account_id: Uuid, delta: Decimal) -> Result<Decimal> {
        let row: (Decimal,) = sqlx::query_as(
            r#"
            UPDATE emi_accounts
            SET bank_reported_balance = bank_reported_balance + $1,
                updated_at = NOW()
            WHERE id = $2
            RETURNING bank_reported_balance
            "#,
        )
        .bind(delta)
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(row.0)
    }

    /// Get ledger balance
    async fn get_ledger_balance(&self, account_id: Uuid) -> Result<Decimal> {
        let row: (Decimal,) = sqlx::query_as(
            r#"
            SELECT ledger_balance FROM emi_accounts WHERE id = $1
            "#,
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(row.0)
    }

    /// Update reconciliation status
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

    /// Record EMI transaction
    async fn record_emi_transaction(
        &self,
        account_id: Uuid,
        direction: &str,
        amount: Decimal,
        bank_reference: Option<&str>,
        uetr: Option<&str>,
        iso_message_id: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO emi_transactions (
                id, account_id, transaction_type, direction, amount,
                balance_before, balance_after, bank_reference, uetr,
                iso_message_type, iso_message_id, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, 0, 0, $6, $7, $8, $9, $10, NOW(), NOW())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(account_id)
        .bind("FUNDING")
        .bind(direction)
        .bind(amount)
        .bind(bank_reference)
        .bind(uetr)
        .bind("camt.054")
        .bind(iso_message_id)
        .bind("CONFIRMED")
        .execute(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(())
    }

    /// Activate circuit breaker for critical discrepancies
    async fn activate_circuit_breaker(&self, account_id: Uuid) -> Result<()> {
        // Update account status to suspend payouts
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

        // TODO: Publish circuit breaker event to NATS
        // This should trigger alerts to Risk & Finance teams

        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EmiAccount {
    id: Uuid,
    account_number: String,
    iban: Option<String>,
    currency: String,
    ledger_balance: Decimal,
    bank_reported_balance: Decimal,
}

#[derive(Debug, Clone)]
pub struct ReconciliationResult {
    pub account_id: Uuid,
    pub ledger_balance: Decimal,
    pub bank_balance: Decimal,
    pub difference: Decimal,
    pub threshold_level: crate::reconciliation::threshold_checker::ThresholdLevel,
    pub action_taken: String,
}
