// Database layer for Gateway Service
// Persists canonical payments and provides query interface

use sqlx::PgPool;
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, error};

use crate::models::canonical::{CanonicalPayment, PaymentStatus};

/// Insert a new payment into the database
pub async fn insert_payment(pool: &PgPool, payment: &CanonicalPayment) -> Result<()> {
    info!("Inserting payment to DB: {}", payment.deltran_tx_id);

    sqlx::query!(
        r#"
        INSERT INTO payments (
            deltran_tx_id,
            obligation_id,
            uetr,
            end_to_end_id,
            instruction_id,
            instructed_amount,
            settlement_amount,
            currency,
            debtor_name,
            creditor_name,
            debtor_agent_bic,
            creditor_agent_bic,
            status,
            created_at,
            updated_at,
            raw_iso_message
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW(), NOW(), $14
        )
        "#,
        payment.deltran_tx_id,
        payment.obligation_id,
        payment.uetr,
        payment.end_to_end_id,
        payment.instruction_id,
        payment.instructed_amount,
        payment.settlement_amount,
        payment.currency.to_string(),
        payment.debtor.name,
        payment.creditor.name,
        payment.debtor_agent.bic,
        payment.creditor_agent.bic,
        payment.status.to_string(),
        None::<String>, // raw_iso_message - can add later
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get payment by DelTran TX ID
pub async fn get_payment_by_id(pool: &PgPool, tx_id: Uuid) -> Result<Option<CanonicalPayment>> {
    info!("Fetching payment from DB: {}", tx_id);

    let row = sqlx::query!(
        r#"
        SELECT
            deltran_tx_id,
            obligation_id,
            uetr,
            end_to_end_id,
            instruction_id,
            instructed_amount,
            settlement_amount,
            currency,
            debtor_name,
            creditor_name,
            debtor_agent_bic,
            creditor_agent_bic,
            status,
            created_at,
            updated_at
        FROM payments
        WHERE deltran_tx_id = $1
        "#,
        tx_id
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            // Reconstruct CanonicalPayment from DB row
            // TODO: This is simplified - need full reconstruction
            Ok(Some(CanonicalPayment {
                deltran_tx_id: r.deltran_tx_id,
                obligation_id: r.obligation_id,
                uetr: r.uetr,
                end_to_end_id: r.end_to_end_id,
                instruction_id: r.instruction_id,
                instructed_amount: r.instructed_amount,
                settlement_amount: r.settlement_amount,
                currency: r.currency.parse().unwrap_or_default(),
                status: r.status.parse().unwrap_or(PaymentStatus::Received),
                // TODO: Reconstruct Party, FinancialInstitution, etc.
                ..Default::default()
            }))
        }
        None => Ok(None),
    }
}

/// Update payment status
pub async fn update_payment_status(pool: &PgPool, tx_id: Uuid, status: PaymentStatus) -> Result<()> {
    info!("Updating payment status: {} -> {:?}", tx_id, status);

    sqlx::query!(
        r#"
        UPDATE payments
        SET status = $1, updated_at = NOW()
        WHERE deltran_tx_id = $2
        "#,
        status.to_string(),
        tx_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get payments by status
pub async fn get_payments_by_status(pool: &PgPool, status: PaymentStatus, limit: i64) -> Result<Vec<Uuid>> {
    let rows = sqlx::query!(
        r#"
        SELECT deltran_tx_id
        FROM payments
        WHERE status = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
        status.to_string(),
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.deltran_tx_id).collect())
}

/// Get payments pending funding (for monitoring)
pub async fn get_pending_funding(pool: &PgPool) -> Result<Vec<Uuid>> {
    let rows = sqlx::query!(
        r#"
        SELECT deltran_tx_id
        FROM payments
        WHERE status = 'PendingFunding'
        AND created_at > NOW() - INTERVAL '1 hour'
        ORDER BY created_at ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.deltran_tx_id).collect())
}

/// Update payment status by end_to_end_id (for camt.054 funding matching)
pub async fn update_payment_status_by_e2e(
    pool: &PgPool,
    end_to_end_id: &str,
    status: PaymentStatus
) -> Result<()> {
    info!("Updating payment status by E2E: {} -> {:?}", end_to_end_id, status);

    sqlx::query!(
        r#"
        UPDATE payments
        SET status = $1, updated_at = NOW(), funded_at = NOW()
        WHERE end_to_end_id = $2
        "#,
        status.to_string(),
        end_to_end_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Get payment by end_to_end_id (for camt.054 matching)
pub async fn get_payment_by_e2e(pool: &PgPool, end_to_end_id: &str) -> Result<Option<CanonicalPayment>> {
    info!("Fetching payment from DB by E2E: {}", end_to_end_id);

    let row = sqlx::query!(
        r#"
        SELECT
            deltran_tx_id,
            obligation_id,
            uetr,
            end_to_end_id,
            instruction_id,
            instructed_amount,
            settlement_amount,
            currency,
            debtor_name,
            creditor_name,
            debtor_agent_bic,
            creditor_agent_bic,
            status,
            created_at,
            updated_at
        FROM payments
        WHERE end_to_end_id = $1
        "#,
        end_to_end_id
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => {
            Ok(Some(CanonicalPayment {
                deltran_tx_id: r.deltran_tx_id,
                obligation_id: r.obligation_id,
                uetr: r.uetr,
                end_to_end_id: r.end_to_end_id,
                instruction_id: r.instruction_id,
                instructed_amount: r.instructed_amount,
                settlement_amount: r.settlement_amount,
                currency: r.currency.parse().unwrap_or_default(),
                status: r.status.parse().unwrap_or(PaymentStatus::Received),
                ..Default::default()
            }))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_insert_payment() {
        // TODO: Add DB tests
    }
}
