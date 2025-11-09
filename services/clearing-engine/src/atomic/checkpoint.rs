use crate::database::DbPool;
use crate::errors::{ClearingError, Result};
use crate::models::OperationCheckpoint;
use serde_json::Value;
use tracing::{debug, info};
use uuid::Uuid;

/// CheckpointManager handles checkpoint creation and retrieval for atomic operations
pub struct CheckpointManager {
    pool: DbPool,
}

impl CheckpointManager {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new checkpoint for an operation
    pub async fn create_checkpoint(
        &self,
        operation_id: Uuid,
        checkpoint_name: String,
        checkpoint_order: i32,
        checkpoint_data: Value,
    ) -> Result<Uuid> {
        debug!(
            "Creating checkpoint '{}' for operation {}",
            checkpoint_name, operation_id
        );

        let checkpoint_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO clearing_operation_checkpoints
            (id, operation_id, checkpoint_name, checkpoint_order, checkpoint_data, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
            checkpoint_id,
            operation_id,
            checkpoint_name,
            checkpoint_order,
            checkpoint_data,
        )
        .execute(&self.pool)
        .await?;

        info!(
            "Checkpoint '{}' created successfully for operation {}",
            checkpoint_name, operation_id
        );

        Ok(checkpoint_id)
    }

    /// Retrieve all checkpoints for an operation in order
    pub async fn get_checkpoints(&self, operation_id: Uuid) -> Result<Vec<OperationCheckpoint>> {
        let checkpoints = sqlx::query_as!(
            OperationCheckpoint,
            r#"
            SELECT id, operation_id, checkpoint_name, checkpoint_order, checkpoint_data, created_at
            FROM clearing_operation_checkpoints
            WHERE operation_id = $1
            ORDER BY checkpoint_order ASC
            "#,
            operation_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(checkpoints)
    }

    /// Get checkpoints in reverse order for rollback
    pub async fn get_checkpoints_reverse(
        &self,
        operation_id: Uuid,
    ) -> Result<Vec<OperationCheckpoint>> {
        let checkpoints = sqlx::query_as!(
            OperationCheckpoint,
            r#"
            SELECT id, operation_id, checkpoint_name, checkpoint_order, checkpoint_data, created_at
            FROM clearing_operation_checkpoints
            WHERE operation_id = $1
            ORDER BY checkpoint_order DESC
            "#,
            operation_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(checkpoints)
    }

    /// Get a specific checkpoint by name
    pub async fn get_checkpoint_by_name(
        &self,
        operation_id: Uuid,
        checkpoint_name: &str,
    ) -> Result<OperationCheckpoint> {
        let checkpoint = sqlx::query_as!(
            OperationCheckpoint,
            r#"
            SELECT id, operation_id, checkpoint_name, checkpoint_order, checkpoint_data, created_at
            FROM clearing_operation_checkpoints
            WHERE operation_id = $1 AND checkpoint_name = $2
            "#,
            operation_id,
            checkpoint_name,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ClearingError::CheckpointNotFound {
            checkpoint_name: checkpoint_name.to_string(),
            operation_id,
        })?;

        Ok(checkpoint)
    }

    /// Delete all checkpoints for an operation
    pub async fn delete_checkpoints(&self, operation_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM clearing_operation_checkpoints
            WHERE operation_id = $1
            "#,
            operation_id,
        )
        .execute(&self.pool)
        .await?;

        debug!(
            "All checkpoints deleted for operation {}",
            operation_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Note: These tests require a running database
    #[tokio::test]
    #[ignore]
    async fn test_checkpoint_creation_and_retrieval() {
        // This would require proper test database setup
        // Placeholder for actual implementation
    }
}
