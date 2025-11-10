use crate::config::Config;
use crate::error::{Result, SettlementError};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::info;
use uuid::Uuid;

pub struct RetryManager {
    db_pool: Arc<PgPool>,
    config: Arc<Config>,
}

impl RetryManager {
    pub fn new(db_pool: Arc<PgPool>, config: Arc<Config>) -> Self {
        Self { db_pool, config }
    }

    pub async fn should_retry(&self, settlement_id: Uuid) -> Result<bool> {
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

        Ok(settlement.retry_count.unwrap_or(0) < self.config.settlement.max_retry_attempts as i32)
    }

    pub async fn increment_retry_count(&self, settlement_id: Uuid) -> Result<i32> {
        let result = sqlx::query!(
            r#"
            UPDATE settlement_transactions
            SET retry_count = retry_count + 1,
                last_retry_at = $1
            WHERE id = $2
            RETURNING retry_count
            "#,
            Utc::now(),
            settlement_id
        )
        .fetch_one(&*self.db_pool)
        .await?;

        Ok(result.retry_count.unwrap_or(0))
    }

    pub async fn mark_for_retry(&self, settlement_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE settlement_transactions
            SET status = 'PENDING',
                error_message = NULL
            WHERE id = $1
            "#,
            settlement_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Marked settlement {} for retry", settlement_id);

        Ok(())
    }

    pub async fn get_failed_settlements(&self, limit: i64) -> Result<Vec<Uuid>> {
        let records = sqlx::query!(
            r#"
            SELECT id
            FROM settlement_transactions
            WHERE status = 'FAILED'
                AND retry_count < $1
                AND (last_retry_at IS NULL OR last_retry_at < NOW() - INTERVAL '5 minutes')
            ORDER BY created_at
            LIMIT $2
            "#,
            self.config.settlement.max_retry_attempts as i32,
            limit
        )
        .fetch_all(&*self.db_pool)
        .await?;

        Ok(records.into_iter().map(|r| r.id).collect())
    }

    pub async fn exponential_backoff(&self, retry_count: i32) -> Duration {
        let base_delay = self.config.settlement.retry_delay_seconds;
        let delay_seconds = base_delay * 2_u64.pow(retry_count as u32);
        Duration::from_secs(delay_seconds.min(3600)) // Max 1 hour
    }
}
