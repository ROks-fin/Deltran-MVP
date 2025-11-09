use crate::error::{Result, SettlementError};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

pub struct RollbackManager {
    db_pool: Arc<PgPool>,
}

impl RollbackManager {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    /// Rollback a settlement and all its associated operations
    pub async fn rollback_settlement(&self, settlement_id: Uuid, reason: &str) -> Result<()> {
        info!("Rolling back settlement {}: {}", settlement_id, reason);

        let mut tx = self.db_pool.begin().await?;

        // 1. Release all fund locks for this settlement
        let locks = sqlx::query!(
            r#"
            SELECT id, nostro_account_id, amount
            FROM fund_locks
            WHERE settlement_id = $1 AND status = 'active'
            "#,
            settlement_id
        )
        .fetch_all(&mut *tx)
        .await?;

        for lock in locks {
            // Update lock status
            sqlx::query!(
                r#"
                UPDATE fund_locks
                SET status = 'released',
                    released_at = $1,
                    released_by = 'rollback'
                WHERE id = $2
                "#,
                Utc::now(),
                lock.id
            )
            .execute(&mut *tx)
            .await?;

            // Restore available balance
            sqlx::query!(
                r#"
                UPDATE nostro_accounts
                SET available_balance = available_balance + $1,
                    locked_balance = locked_balance - $1
                WHERE id = $2
                "#,
                lock.amount,
                lock.nostro_account_id
            )
            .execute(&mut *tx)
            .await?;

            info!("Released fund lock {}", lock.id);
        }

        // 2. Update settlement status
        sqlx::query!(
            r#"
            UPDATE settlement_transactions
            SET status = $1,
                rolled_back_at = $2,
                error_message = $3
            WHERE id = $4
            "#,
            SettlementStatus::RolledBack.to_string(),
            Utc::now(),
            reason,
            settlement_id
        )
        .execute(&mut *tx)
        .await?;

        // 3. Mark atomic operation as rolled back
        sqlx::query!(
            r#"
            UPDATE settlement_atomic_operations
            SET state = 'RolledBack',
                rolled_back_at = $1,
                rollback_reason = $2
            WHERE settlement_id = $3
            "#,
            Utc::now(),
            reason,
            settlement_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        info!("Successfully rolled back settlement {}", settlement_id);

        Ok(())
    }

    /// Clean up expired fund locks
    pub async fn cleanup_expired_locks(&self) -> Result<usize> {
        info!("Cleaning up expired fund locks");

        let mut tx = self.db_pool.begin().await?;

        // Find expired locks
        let expired_locks = sqlx::query!(
            r#"
            SELECT id, nostro_account_id, amount, settlement_id
            FROM fund_locks
            WHERE status = 'active' AND expires_at < NOW()
            "#
        )
        .fetch_all(&mut *tx)
        .await?;

        let count = expired_locks.len();

        for lock in expired_locks {
            warn!(
                "Releasing expired lock {} for settlement {}",
                lock.id, lock.settlement_id
            );

            // Update lock status
            sqlx::query!(
                r#"
                UPDATE fund_locks
                SET status = 'expired',
                    released_at = $1,
                    released_by = 'auto_cleanup'
                WHERE id = $2
                "#,
                Utc::now(),
                lock.id
            )
            .execute(&mut *tx)
            .await?;

            // Restore available balance
            sqlx::query!(
                r#"
                UPDATE nostro_accounts
                SET available_balance = available_balance + $1,
                    locked_balance = locked_balance - $1
                WHERE id = $2
                "#,
                lock.amount,
                lock.nostro_account_id
            )
            .execute(&mut *tx)
            .await?;

            // Mark settlement as failed if it hasn't completed
            sqlx::query!(
                r#"
                UPDATE settlement_transactions
                SET status = 'FAILED',
                    failed_at = $1,
                    error_message = 'Fund lock expired'
                WHERE id = $2 AND status NOT IN ('COMPLETED', 'ROLLED_BACK')
                "#,
                Utc::now(),
                lock.settlement_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        if count > 0 {
            info!("Cleaned up {} expired fund locks", count);
        }

        Ok(count)
    }

    /// Retry a failed settlement
    pub async fn retry_settlement(&self, settlement_id: Uuid) -> Result<()> {
        info!("Retrying settlement {}", settlement_id);

        let settlement = sqlx::query!(
            r#"
            SELECT retry_count
            FROM settlement_transactions
            WHERE id = $1
            "#,
            settlement_id
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::Internal(format!("Settlement {} not found", settlement_id))
        })?;

        // Update retry count and reset status
        sqlx::query!(
            r#"
            UPDATE settlement_transactions
            SET status = 'PENDING',
                retry_count = $1,
                error_message = NULL,
                failed_at = NULL
            WHERE id = $2
            "#,
            settlement.retry_count + 1,
            settlement_id
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(())
    }

    /// Get rollback statistics
    pub async fn get_rollback_stats(&self) -> Result<RollbackStats> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'ROLLED_BACK') as rolled_back_count,
                COUNT(*) FILTER (WHERE status = 'FAILED') as failed_count,
                COUNT(*) FILTER (WHERE retry_count > 0) as retried_count
            FROM settlement_transactions
            WHERE created_at > NOW() - INTERVAL '24 hours'
            "#
        )
        .fetch_one(&*self.db_pool)
        .await?;

        Ok(RollbackStats {
            rolled_back_count: stats.rolled_back_count.unwrap_or(0),
            failed_count: stats.failed_count.unwrap_or(0),
            retried_count: stats.retried_count.unwrap_or(0),
        })
    }
}

#[derive(Debug)]
pub struct RollbackStats {
    pub rolled_back_count: i64,
    pub failed_count: i64,
    pub retried_count: i64,
}
