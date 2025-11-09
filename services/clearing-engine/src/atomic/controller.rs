use crate::atomic::operation::AtomicOperationHandler;
use crate::database::DbPool;
use crate::errors::{ClearingError, Result};
use crate::models::{AtomicOperation, AtomicOperationType, AtomicState};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// AtomicController orchestrates atomic operations across the clearing process
pub struct AtomicController {
    pool: DbPool,
}

impl AtomicController {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new atomic operation
    pub async fn create_operation(
        &self,
        window_id: i64,
        operation_type: AtomicOperationType,
    ) -> Result<AtomicOperationHandler> {
        AtomicOperationHandler::new(self.pool.clone(), window_id, operation_type).await
    }

    /// Get an existing operation
    pub async fn get_operation(&self, operation_id: Uuid) -> Result<AtomicOperation> {
        let operation = sqlx::query_as!(
            AtomicOperation,
            r#"
            SELECT operation_id, window_id, operation_type, state,
                   parent_operation_id, checkpoints, started_at, completed_at,
                   rolled_back_at, error_message, error_code, rollback_data, rollback_reason
            FROM clearing_atomic_operations
            WHERE operation_id = $1
            "#,
            operation_id,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            ClearingError::Internal(format!("Operation not found: {}", operation_id))
        })?;

        Ok(operation)
    }

    /// Get all operations for a window
    pub async fn get_window_operations(&self, window_id: i64) -> Result<Vec<AtomicOperation>> {
        let operations = sqlx::query_as!(
            AtomicOperation,
            r#"
            SELECT operation_id, window_id, operation_type, state,
                   parent_operation_id, checkpoints, started_at, completed_at,
                   rolled_back_at, error_message, error_code, rollback_data, rollback_reason
            FROM clearing_atomic_operations
            WHERE window_id = $1
            ORDER BY started_at ASC
            "#,
            window_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(operations)
    }

    /// Check if there are any failed operations for a window
    pub async fn has_failed_operations(&self, window_id: i64) -> Result<bool> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM clearing_atomic_operations
            WHERE window_id = $1 AND state = 'Failed'
            "#,
            window_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Rollback all operations for a window in reverse order
    pub async fn rollback_window_operations(&self, window_id: i64, reason: String) -> Result<()> {
        info!("Rolling back all operations for window {}", window_id);

        let operations = sqlx::query_as!(
            AtomicOperation,
            r#"
            SELECT operation_id, window_id, operation_type, state,
                   parent_operation_id, checkpoints, started_at, completed_at,
                   rolled_back_at, error_message, error_code, rollback_data, rollback_reason
            FROM clearing_atomic_operations
            WHERE window_id = $1 AND state IN ('Committed', 'InProgress')
            ORDER BY started_at DESC
            "#,
            window_id,
        )
        .fetch_all(&self.pool)
        .await?;

        info!("Found {} operations to rollback", operations.len());

        for operation in operations {
            // Skip operations already rolled back
            if operation.state == AtomicState::RolledBack.as_str() {
                continue;
            }

            let handler = AtomicOperationHandler::new(
                self.pool.clone(),
                window_id,
                match operation.operation_type.as_str() {
                    "WindowClose" => AtomicOperationType::WindowClose,
                    "ObligationCollection" => AtomicOperationType::ObligationCollection,
                    "NettingCalculation" => AtomicOperationType::NettingCalculation,
                    "InstructionGeneration" => AtomicOperationType::InstructionGeneration,
                    "SettlementInitiation" => AtomicOperationType::SettlementInitiation,
                    "WindowOpen" => AtomicOperationType::WindowOpen,
                    _ => continue,
                },
            )
            .await?;

            match handler.rollback(reason.clone()).await {
                Ok(_) => {
                    info!(
                        "Successfully rolled back operation {} ({})",
                        operation.operation_id, operation.operation_type
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to rollback operation {} ({}): {}",
                        operation.operation_id, operation.operation_type, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Get operation statistics for a window
    pub async fn get_window_stats(&self, window_id: i64) -> Result<OperationStats> {
        let stats = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE state = 'Pending') as pending,
                COUNT(*) FILTER (WHERE state = 'InProgress') as in_progress,
                COUNT(*) FILTER (WHERE state = 'Committed') as committed,
                COUNT(*) FILTER (WHERE state = 'RolledBack') as rolled_back,
                COUNT(*) FILTER (WHERE state = 'Failed') as failed
            FROM clearing_atomic_operations
            WHERE window_id = $1
            "#,
            window_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(OperationStats {
            total: stats.total.unwrap_or(0) as u32,
            pending: stats.pending.unwrap_or(0) as u32,
            in_progress: stats.in_progress.unwrap_or(0) as u32,
            committed: stats.committed.unwrap_or(0) as u32,
            rolled_back: stats.rolled_back.unwrap_or(0) as u32,
            failed: stats.failed.unwrap_or(0) as u32,
        })
    }

    /// Clean up old completed operations (older than retention days)
    pub async fn cleanup_old_operations(&self, retention_days: i32) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM clearing_atomic_operations
            WHERE state IN ('Committed', 'RolledBack')
            AND started_at < NOW() - INTERVAL '1 day' * $1
            "#,
            retention_days,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[derive(Debug, Clone)]
pub struct OperationStats {
    pub total: u32,
    pub pending: u32,
    pub in_progress: u32,
    pub committed: u32,
    pub rolled_back: u32,
    pub failed: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_controller_create_operation() {
        // Requires database
    }

    #[tokio::test]
    #[ignore]
    async fn test_controller_window_rollback() {
        // Requires database
    }
}
