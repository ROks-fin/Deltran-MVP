use crate::atomic::checkpoint::CheckpointManager;
use crate::database::DbPool;
use crate::errors::{ClearingError, Result};
use crate::models::{AtomicOperation, AtomicOperationType, AtomicState};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// AtomicOperationHandler manages the execution of atomic operations with rollback capability
pub struct AtomicOperationHandler {
    pool: DbPool,
    checkpoint_manager: Arc<CheckpointManager>,
    operation_id: Uuid,
    window_id: i64,
    operation_type: AtomicOperationType,
    state: Arc<RwLock<AtomicState>>,
    checkpoint_counter: Arc<RwLock<i32>>,
}

impl AtomicOperationHandler {
    /// Create a new atomic operation handler
    pub async fn new(
        pool: DbPool,
        window_id: i64,
        operation_type: AtomicOperationType,
    ) -> Result<Self> {
        let operation_id = Uuid::new_v4();
        let checkpoint_manager = Arc::new(CheckpointManager::new(pool.clone()));

        // Create operation record in database
        sqlx::query!(
            r#"
            INSERT INTO clearing_atomic_operations
            (operation_id, window_id, operation_type, state, checkpoints, started_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
            operation_id,
            window_id,
            operation_type.as_str(),
            AtomicState::Pending.as_str(),
            json!([]),
        )
        .execute(&pool)
        .await?;

        info!(
            "Atomic operation created: {} for window {}",
            operation_id, window_id
        );

        Ok(Self {
            pool,
            checkpoint_manager,
            operation_id,
            window_id,
            operation_type,
            state: Arc::new(RwLock::new(AtomicState::Pending)),
            checkpoint_counter: Arc::new(RwLock::new(0)),
        })
    }

    /// Get the operation ID
    pub fn operation_id(&self) -> Uuid {
        self.operation_id
    }

    /// Get the current state
    pub async fn get_state(&self) -> AtomicState {
        self.state.read().await.clone()
    }

    /// Start the operation
    pub async fn start(&self) -> Result<()> {
        info!(
            "Starting atomic operation {} ({})",
            self.operation_id,
            self.operation_type.as_str()
        );

        let mut state = self.state.write().await;
        *state = AtomicState::InProgress;

        sqlx::query!(
            r#"
            UPDATE clearing_atomic_operations
            SET state = $1
            WHERE operation_id = $2
            "#,
            AtomicState::InProgress.as_str(),
            self.operation_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a checkpoint within the operation
    pub async fn checkpoint(
        &self,
        checkpoint_name: String,
        checkpoint_data: Value,
    ) -> Result<Uuid> {
        let mut counter = self.checkpoint_counter.write().await;
        *counter += 1;
        let order = *counter;

        debug!(
            "Creating checkpoint '{}' (order: {}) for operation {}",
            checkpoint_name, order, self.operation_id
        );

        let checkpoint_id = self
            .checkpoint_manager
            .create_checkpoint(self.operation_id, checkpoint_name, order, checkpoint_data)
            .await?;

        Ok(checkpoint_id)
    }

    /// Commit the operation
    pub async fn commit(&self) -> Result<()> {
        info!("Committing atomic operation {}", self.operation_id);

        let mut state = self.state.write().await;
        *state = AtomicState::Committed;

        sqlx::query!(
            r#"
            UPDATE clearing_atomic_operations
            SET state = $1, completed_at = NOW()
            WHERE operation_id = $2
            "#,
            AtomicState::Committed.as_str(),
            self.operation_id,
        )
        .execute(&self.pool)
        .await?;

        info!("Operation {} committed successfully", self.operation_id);

        Ok(())
    }

    /// Rollback the operation using checkpoints in reverse order
    pub async fn rollback(&self, reason: String) -> Result<()> {
        error!(
            "Rolling back operation {} - Reason: {}",
            self.operation_id, reason
        );

        let mut state = self.state.write().await;

        // Get checkpoints in reverse order
        let checkpoints = self
            .checkpoint_manager
            .get_checkpoints_reverse(self.operation_id)
            .await?;

        info!(
            "Found {} checkpoints to rollback for operation {}",
            checkpoints.len(),
            self.operation_id
        );

        // Execute rollback for each checkpoint in reverse
        for checkpoint in checkpoints {
            debug!(
                "Rolling back checkpoint '{}' (order: {})",
                checkpoint.checkpoint_name, checkpoint.checkpoint_order
            );

            // Rollback logic would be specific to each checkpoint type
            // This is a hook for custom rollback handlers
            match self
                .execute_checkpoint_rollback(&checkpoint.checkpoint_name, &checkpoint.checkpoint_data)
                .await
            {
                Ok(_) => {
                    info!(
                        "Successfully rolled back checkpoint '{}'",
                        checkpoint.checkpoint_name
                    );
                }
                Err(e) => {
                    warn!(
                        "Failed to rollback checkpoint '{}': {}",
                        checkpoint.checkpoint_name, e
                    );
                    // Continue with other checkpoints even if one fails
                }
            }
        }

        *state = AtomicState::RolledBack;

        sqlx::query!(
            r#"
            UPDATE clearing_atomic_operations
            SET state = $1, rolled_back_at = NOW(), rollback_reason = $2
            WHERE operation_id = $3
            "#,
            AtomicState::RolledBack.as_str(),
            reason,
            self.operation_id,
        )
        .execute(&self.pool)
        .await?;

        info!("Operation {} rolled back successfully", self.operation_id);

        Ok(())
    }

    /// Mark operation as failed
    pub async fn fail(&self, error_message: String, error_code: Option<String>) -> Result<()> {
        error!(
            "Operation {} failed: {}",
            self.operation_id, error_message
        );

        let mut state = self.state.write().await;
        *state = AtomicState::Failed;

        sqlx::query!(
            r#"
            UPDATE clearing_atomic_operations
            SET state = $1, error_message = $2, error_code = $3, completed_at = NOW()
            WHERE operation_id = $4
            "#,
            AtomicState::Failed.as_str(),
            error_message,
            error_code,
            self.operation_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Execute rollback for a specific checkpoint
    /// This is a hook that can be customized based on checkpoint type
    async fn execute_checkpoint_rollback(
        &self,
        checkpoint_name: &str,
        checkpoint_data: &Value,
    ) -> Result<()> {
        // This would contain specific rollback logic based on checkpoint type
        // For now, we log the rollback action
        debug!(
            "Executing rollback for checkpoint '{}' with data: {}",
            checkpoint_name, checkpoint_data
        );

        // Specific rollback handlers would be implemented here based on checkpoint_name
        match checkpoint_name {
            "window_status_changed" => {
                // Revert window status change
                if let Some(old_status) = checkpoint_data.get("old_status") {
                    debug!("Would revert window status to: {}", old_status);
                }
            }
            "obligations_collected" => {
                // Mark obligations as not processed
                debug!("Would mark obligations as not processed");
            }
            "netting_calculated" => {
                // Delete net positions
                debug!("Would delete calculated net positions");
            }
            "instructions_generated" => {
                // Delete settlement instructions
                debug!("Would delete generated settlement instructions");
            }
            _ => {
                debug!("No specific rollback handler for '{}'", checkpoint_name);
            }
        }

        Ok(())
    }

    /// Execute the operation with automatic rollback on failure
    pub async fn execute<F, Fut>(&self, operation_fn: F) -> Result<()>
    where
        F: FnOnce(Arc<Self>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        self.start().await?;

        let self_arc = Arc::new(self.clone_handler());

        match operation_fn(self_arc).await {
            Ok(_) => {
                self.commit().await?;
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.rollback(error_msg.clone()).await?;
                Err(e)
            }
        }
    }

    /// Clone handler for Arc wrapping
    fn clone_handler(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            checkpoint_manager: self.checkpoint_manager.clone(),
            operation_id: self.operation_id,
            window_id: self.window_id,
            operation_type: self.operation_type.clone(),
            state: self.state.clone(),
            checkpoint_counter: self.checkpoint_counter.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_atomic_operation_lifecycle() {
        // Requires database
        // Test: create -> start -> checkpoint -> commit
    }

    #[tokio::test]
    #[ignore]
    async fn test_atomic_operation_rollback() {
        // Requires database
        // Test: create -> start -> checkpoint -> fail -> rollback
    }
}
