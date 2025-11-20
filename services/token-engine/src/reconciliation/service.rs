// Reconciliation Service - Main orchestrator for all reconciliation tiers

use crate::errors::{Result, TokenEngineError};
use crate::reconciliation::{
    Camt053Processor, Camt054Processor, DiscrepancyDetector,
    camt053_processor::{Camt053Statement, EodReconciliationResult},
    camt054_processor::{Camt054Notification, ReconciliationResult},
};
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Main Reconciliation Service implementing three-tier reconciliation
pub struct ReconciliationService {
    pool: PgPool,
    camt054_processor: Arc<Camt054Processor>,
    camt053_processor: Arc<Camt053Processor>,
}

impl ReconciliationService {
    pub fn new(pool: PgPool) -> Self {
        let camt054_processor = Arc::new(Camt054Processor::new(pool.clone()));
        let camt053_processor = Arc::new(Camt053Processor::new(pool.clone()));

        Self {
            pool,
            camt054_processor,
            camt053_processor,
        }
    }

    // ========== TIER 1: Near Real-Time (CAMT.054) ==========

    /// Process incoming CAMT.054 notification (triggered by bank webhook/NATS event)
    pub async fn process_camt054_notification(
        &self,
        notification: Camt054Notification,
    ) -> Result<ReconciliationResult> {
        info!(
            "TIER 1 - Near Real-Time: Processing CAMT.054 for account {}",
            notification.account_id
        );

        self.camt054_processor
            .process_notification(notification)
            .await
    }

    // ========== TIER 2: Intradey (15-60 min) ==========

    /// Run intradey reconciliation for a specific account
    pub async fn run_intradey_reconciliation(
        &self,
        account_id: Uuid,
    ) -> Result<IntradeyReconciliationResult> {
        info!("TIER 2 - Intradey: Running reconciliation for account {}", account_id);

        // Get account info
        let account = self.get_emi_account(account_id).await?;

        // Get current balances
        let ledger_balance = self.get_ledger_balance(account_id).await?;
        let bank_balance = self.query_bank_balance_api(account_id).await?;

        info!(
            "Intradey check for account {}: ledger={}, bank={}",
            account_id, ledger_balance, bank_balance
        );

        // Check threshold
        let threshold_result = crate::reconciliation::ThresholdChecker::check(
            ledger_balance,
            bank_balance,
        );

        // Update reconciliation status
        let status = if threshold_result.level == crate::reconciliation::threshold_checker::ThresholdLevel::Ok {
            "OK"
        } else {
            "MISMATCH"
        };

        self.update_reconciliation_status(
            account_id,
            status,
            "API_POLL",
            None,
            threshold_result.absolute_difference,
        ).await?;

        // Create discrepancy if needed
        if threshold_result.level != crate::reconciliation::threshold_checker::ThresholdLevel::Ok {
            warn!(
                "Intradey mismatch detected for account {}: {}",
                account_id, threshold_result.action_required
            );

            let threshold_exceeded = crate::reconciliation::ThresholdChecker::should_suspend_payouts(&threshold_result);

            DiscrepancyDetector::create_balance_mismatch(
                &self.pool,
                account_id,
                ledger_balance,
                bank_balance,
                "API_POLL",
                None,
                threshold_exceeded,
            ).await?;
        }

        Ok(IntradeyReconciliationResult {
            account_id,
            account_number: account.account_number,
            currency: account.currency,
            ledger_balance,
            bank_balance,
            difference: threshold_result.absolute_difference,
            threshold_level: threshold_result.level,
            timestamp: Utc::now(),
        })
    }

    /// Run intradey reconciliation for all active accounts
    pub async fn run_intradey_reconciliation_all(&self) -> Result<Vec<IntradeyReconciliationResult>> {
        info!("TIER 2 - Intradey: Running reconciliation for all accounts");

        let account_ids = self.get_active_account_ids().await?;
        let mut results = Vec::new();

        for account_id in account_ids {
            match self.run_intradey_reconciliation(account_id).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("Failed intradey reconciliation for account {}: {}", account_id, e);
                }
            }
        }

        info!("Intradey reconciliation complete: {} accounts processed", results.len());
        Ok(results)
    }

    /// Start continuous intradey reconciliation loop (15-60 min interval)
    pub async fn start_intradey_loop(self: Arc<Self>, interval_minutes: u64) {
        let mut ticker = interval(Duration::from_secs(interval_minutes * 60));

        info!(
            "Starting intradey reconciliation loop with {} minute interval",
            interval_minutes
        );

        loop {
            ticker.tick().await;

            info!("Intradey reconciliation tick - starting batch reconciliation");
            if let Err(e) = self.run_intradey_reconciliation_all().await {
                error!("Intradey reconciliation batch failed: {}", e);
            }
        }
    }

    // ========== TIER 3: EOD (End-of-Day CAMT.053) ==========

    /// Process EOD statement (CAMT.053)
    pub async fn process_eod_statement(
        &self,
        statement: Camt053Statement,
    ) -> Result<EodReconciliationResult> {
        info!(
            "TIER 3 - EOD: Processing CAMT.053 statement for account {} on {}",
            statement.account_id, statement.statement_date
        );

        self.camt053_processor
            .process_statement(statement)
            .await
    }

    // ========== Helper Methods ==========

    async fn get_emi_account(&self, account_id: Uuid) -> Result<EmiAccountInfo> {
        let account = sqlx::query_as::<_, EmiAccountInfo>(
            r#"
            SELECT id, account_number, currency, ledger_balance, bank_reported_balance
            FROM emi_accounts
            WHERE id = $1
            "#,
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?
        .ok_or_else(|| TokenEngineError::Internal(format!(
            "EMI account not found: {}",
            account_id
        )))?;

        Ok(account)
    }

    async fn get_active_account_ids(&self) -> Result<Vec<Uuid>> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM emi_accounts
            WHERE metadata->>'circuit_breaker_active' IS NULL
               OR metadata->>'circuit_breaker_active' = 'false'
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
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

    /// Query bank balance via API
    /// In production, this would call actual bank API
    async fn query_bank_balance_api(&self, account_id: Uuid) -> Result<Decimal> {
        // TODO: Implement actual bank API call
        // For now, return the stored bank_reported_balance
        let row: (Decimal,) = sqlx::query_as(
            "SELECT bank_reported_balance FROM emi_accounts WHERE id = $1"
        )
        .bind(account_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(row.0)
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

    // ========== Monitoring & Reporting ==========

    /// Get reconciliation status summary
    pub async fn get_reconciliation_summary(&self) -> Result<ReconciliationSummary> {
        let total_accounts: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM emi_accounts"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        let accounts_ok: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM emi_accounts WHERE reconciliation_status = 'OK'"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        let accounts_mismatch: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM emi_accounts WHERE reconciliation_status = 'MISMATCH'"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        let open_discrepancies: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reconciliation_discrepancies WHERE status IN ('OPEN', 'INVESTIGATING')"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        let critical_discrepancies: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reconciliation_discrepancies WHERE threshold_exceeded = true AND status IN ('OPEN', 'INVESTIGATING')"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(ReconciliationSummary {
            total_accounts: total_accounts.0,
            accounts_ok: accounts_ok.0,
            accounts_mismatch: accounts_mismatch.0,
            open_discrepancies: open_discrepancies.0,
            critical_discrepancies: critical_discrepancies.0,
            timestamp: Utc::now(),
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct EmiAccountInfo {
    id: Uuid,
    account_number: String,
    currency: String,
    ledger_balance: Decimal,
    bank_reported_balance: Decimal,
}

#[derive(Debug, Clone)]
pub struct IntradeyReconciliationResult {
    pub account_id: Uuid,
    pub account_number: String,
    pub currency: String,
    pub ledger_balance: Decimal,
    pub bank_balance: Decimal,
    pub difference: Decimal,
    pub threshold_level: crate::reconciliation::threshold_checker::ThresholdLevel,
    pub timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ReconciliationSummary {
    pub total_accounts: i64,
    pub accounts_ok: i64,
    pub accounts_mismatch: i64,
    pub open_discrepancies: i64,
    pub critical_discrepancies: i64,
    pub timestamp: chrono::DateTime<Utc>,
}
