use crate::error::{Result, SettlementError};
use crate::integration::{BankClient, TransferRequest};
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

pub struct CompensationManager {
    db_pool: Arc<PgPool>,
}

impl CompensationManager {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    /// Create a compensation transaction to reverse a settlement
    pub async fn create_compensation(
        &self,
        original_settlement_id: Uuid,
        reason: &str,
    ) -> Result<Uuid> {
        info!(
            "Creating compensation for settlement {}: {}",
            original_settlement_id, reason
        );

        // Get original settlement details
        let original = sqlx::query(
            r#"
            SELECT from_bank, to_bank, amount, currency, obligation_id
            FROM settlement_transactions
            WHERE id = $1
            "#
        )
        .bind(original_settlement_id)
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::Internal(format!(
                "Original settlement {} not found",
                original_settlement_id
            ))
        })?;

        let from_bank: String = original.try_get("from_bank")?;
        let to_bank: String = original.try_get("to_bank")?;
        let amount: Decimal = original.try_get("amount")?;
        let currency: String = original.try_get("currency")?;
        let obligation_id: Uuid = original.try_get("obligation_id")?;

        // Create reversal transaction (swap from/to)
        let compensation_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO settlement_transactions (
                id, obligation_id, from_bank, to_bank,
                amount, currency, status, priority,
                metadata, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, 'PENDING', 'urgent', $7, $8)
            "#
        )
        .bind(compensation_id)
        .bind(obligation_id)
        .bind(&to_bank) // Reversed
        .bind(&from_bank) // Reversed
        .bind(amount)
        .bind(&currency)
        .bind(serde_json::json!({
            "type": "compensation",
            "original_settlement_id": original_settlement_id,
            "reason": reason
        }))
        .bind(Utc::now())
        .execute(&*self.db_pool)
        .await?;

        // Link compensation to original
        sqlx::query(
            r#"
            INSERT INTO compensation_transactions (
                id, original_settlement_id, compensation_settlement_id,
                reason, created_at
            ) VALUES ($1, $2, $3, $4, $5)
            "#
        )
        .bind(Uuid::new_v4())
        .bind(original_settlement_id)
        .bind(compensation_id)
        .bind(reason)
        .bind(Utc::now())
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Created compensation settlement {} for {}",
            compensation_id, original_settlement_id
        );

        Ok(compensation_id)
    }

    pub async fn execute_compensation(
        &self,
        compensation_id: Uuid,
        bank_client: &dyn BankClient,
    ) -> Result<()> {
        info!("Executing compensation {}", compensation_id);

        // Get compensation details
        let compensation = sqlx::query(
            r#"
            SELECT from_bank, to_bank, amount, currency
            FROM settlement_transactions
            WHERE id = $1
            "#
        )
        .bind(compensation_id)
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::Internal(format!("Compensation {} not found", compensation_id))
        })?;

        let from_bank: String = compensation.try_get("from_bank")?;
        let to_bank: String = compensation.try_get("to_bank")?;
        let amount: Decimal = compensation.try_get("amount")?;
        let currency: String = compensation.try_get("currency")?;

        // Execute reversal transfer
        let transfer_request = TransferRequest {
            settlement_id: compensation_id,
            from_bank,
            to_bank,
            amount,
            currency,
            reference: format!("COMPENSATION-{}", compensation_id),
            metadata: serde_json::json!({"type": "compensation"}),
        };

        let result = bank_client.initiate_transfer(&transfer_request).await?;

        // Update compensation status
        sqlx::query(
            r#"
            UPDATE settlement_transactions
            SET status = 'EXECUTING',
                external_reference = $1,
                executed_at = $2
            WHERE id = $3
            "#
        )
        .bind(&result.external_reference)
        .bind(Utc::now())
        .bind(compensation_id)
        .execute(&*self.db_pool)
        .await?;

        info!("Compensation {} executed successfully", compensation_id);

        Ok(())
    }

    pub async fn get_pending_compensations(&self) -> Result<Vec<Uuid>> {
        let records = sqlx::query(
            r#"
            SELECT id
            FROM settlement_transactions
            WHERE metadata->>'type' = 'compensation'
                AND status = 'PENDING'
            ORDER BY created_at
            "#
        )
        .fetch_all(&*self.db_pool)
        .await?;

        Ok(records.into_iter().map(|r| r.try_get("id").unwrap()).collect())
    }
}
