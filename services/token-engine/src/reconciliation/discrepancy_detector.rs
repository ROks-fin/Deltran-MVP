// Discrepancy Detector - Creates and manages reconciliation discrepancies

use crate::errors::{Result, TokenEngineError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscrepancyType {
    BalanceMismatch,
    MissingTxn,
    DuplicateTxn,
    AmountMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "VARCHAR", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiscrepancyStatus {
    Open,
    Investigating,
    Resolved,
    Escalated,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiscrepancySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReconciliationDiscrepancy {
    pub id: Uuid,
    pub account_id: Uuid,
    pub discrepancy_type: String,
    pub detected_at: DateTime<Utc>,
    pub expected_value: Option<Decimal>,
    pub actual_value: Option<Decimal>,
    pub difference: Option<Decimal>,
    pub threshold_type: Option<String>,
    pub threshold_value: Option<Decimal>,
    pub threshold_exceeded: bool,
    pub status: String,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub source_system: Option<String>,
    pub source_reference: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

pub struct DiscrepancyDetector;

impl DiscrepancyDetector {
    /// Create a balance mismatch discrepancy
    pub async fn create_balance_mismatch(
        pool: &sqlx::PgPool,
        account_id: Uuid,
        ledger_balance: Decimal,
        bank_balance: Decimal,
        source_system: &str,
        source_reference: Option<&str>,
        threshold_exceeded: bool,
    ) -> Result<ReconciliationDiscrepancy> {
        let difference = (ledger_balance - bank_balance).abs();
        let percentage_diff = if bank_balance > Decimal::ZERO {
            (difference / bank_balance) * Decimal::from(100)
        } else {
            Decimal::from(100)
        };

        let discrepancy = sqlx::query_as::<_, ReconciliationDiscrepancy>(
            r#"
            INSERT INTO reconciliation_discrepancies (
                id, account_id, discrepancy_type, detected_at,
                expected_value, actual_value, difference,
                threshold_type, threshold_value, threshold_exceeded,
                status, source_system, source_reference, metadata, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(account_id)
        .bind("BALANCE_MISMATCH")
        .bind(Utc::now())
        .bind(ledger_balance)
        .bind(bank_balance)
        .bind(difference)
        .bind("PERCENTAGE")
        .bind(percentage_diff)
        .bind(threshold_exceeded)
        .bind("OPEN")
        .bind(source_system)
        .bind(source_reference)
        .bind(serde_json::json!({
            "ledger_balance": ledger_balance,
            "bank_balance": bank_balance,
            "percentage_diff": percentage_diff,
        }))
        .bind(Utc::now())
        .fetch_one(pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(discrepancy)
    }

    /// Create a missing transaction discrepancy
    pub async fn create_missing_transaction(
        pool: &sqlx::PgPool,
        account_id: Uuid,
        expected_amount: Decimal,
        source_system: &str,
        source_reference: &str,
        metadata: serde_json::Value,
    ) -> Result<ReconciliationDiscrepancy> {
        let discrepancy = sqlx::query_as::<_, ReconciliationDiscrepancy>(
            r#"
            INSERT INTO reconciliation_discrepancies (
                id, account_id, discrepancy_type, detected_at,
                expected_value, status, source_system, source_reference,
                metadata, created_at, threshold_exceeded
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(account_id)
        .bind("MISSING_TXN")
        .bind(Utc::now())
        .bind(expected_amount)
        .bind("OPEN")
        .bind(source_system)
        .bind(source_reference)
        .bind(metadata)
        .bind(Utc::now())
        .bind(false)
        .fetch_one(pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(discrepancy)
    }

    /// Resolve a discrepancy
    pub async fn resolve_discrepancy(
        pool: &sqlx::PgPool,
        discrepancy_id: Uuid,
        resolution_notes: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE reconciliation_discrepancies
            SET status = $1, resolved_at = $2, resolution_notes = $3
            WHERE id = $4
            "#,
        )
        .bind("RESOLVED")
        .bind(Utc::now())
        .bind(resolution_notes)
        .bind(discrepancy_id)
        .execute(pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(())
    }

    /// Escalate a discrepancy
    pub async fn escalate_discrepancy(
        pool: &sqlx::PgPool,
        discrepancy_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE reconciliation_discrepancies
            SET status = $1
            WHERE id = $2
            "#,
        )
        .bind("ESCALATED")
        .bind(discrepancy_id)
        .execute(pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(())
    }

    /// Get open discrepancies for an account
    pub async fn get_open_discrepancies(
        pool: &sqlx::PgPool,
        account_id: Uuid,
    ) -> Result<Vec<ReconciliationDiscrepancy>> {
        let discrepancies = sqlx::query_as::<_, ReconciliationDiscrepancy>(
            r#"
            SELECT * FROM reconciliation_discrepancies
            WHERE account_id = $1 AND status IN ('OPEN', 'INVESTIGATING')
            ORDER BY detected_at DESC
            "#,
        )
        .bind(account_id)
        .fetch_all(pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(discrepancies)
    }

    /// Get critical discrepancies (threshold exceeded)
    pub async fn get_critical_discrepancies(
        pool: &sqlx::PgPool,
    ) -> Result<Vec<ReconciliationDiscrepancy>> {
        let discrepancies = sqlx::query_as::<_, ReconciliationDiscrepancy>(
            r#"
            SELECT * FROM reconciliation_discrepancies
            WHERE threshold_exceeded = true AND status IN ('OPEN', 'INVESTIGATING')
            ORDER BY detected_at DESC
            LIMIT 100
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| TokenEngineError::Database(e))?;

        Ok(discrepancies)
    }
}
