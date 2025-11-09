use crate::error::{Result, SettlementError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AtomicState {
    InProgress,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub rollback_data: Option<serde_json::Value>,
}

pub struct AtomicOperation {
    pub id: Uuid,
    pub settlement_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub checkpoints: Arc<RwLock<Vec<Checkpoint>>>,
    pub state: Arc<RwLock<AtomicState>>,
    db_pool: Arc<PgPool>,
}

pub struct AtomicController {
    operations: Arc<RwLock<HashMap<Uuid, Arc<AtomicOperation>>>>,
    db_pool: Arc<PgPool>,
}

impl AtomicController {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
            db_pool,
        }
    }

    pub async fn begin_operation(
        &self,
        settlement_id: Uuid,
    ) -> Result<Arc<AtomicOperation>> {
        let operation_id = Uuid::new_v4();
        let now = Utc::now();

        // Create database record
        sqlx::query!(
            r#"
            INSERT INTO settlement_atomic_operations (
                id, settlement_id, operation_type, state, started_at
            ) VALUES ($1, $2, 'settlement', 'InProgress', $3)
            "#,
            operation_id,
            settlement_id,
            now
        )
        .execute(&*self.db_pool)
        .await?;

        let operation = Arc::new(AtomicOperation {
            id: operation_id,
            settlement_id,
            started_at: now,
            checkpoints: Arc::new(RwLock::new(Vec::new())),
            state: Arc::new(RwLock::new(AtomicState::InProgress)),
            db_pool: self.db_pool.clone(),
        });

        // Register operation
        self.operations
            .write()
            .await
            .insert(operation_id, operation.clone());

        info!(
            "Atomic operation {} started for settlement {}",
            operation_id, settlement_id
        );

        Ok(operation)
    }

    pub async fn get_operation(&self, operation_id: Uuid) -> Option<Arc<AtomicOperation>> {
        self.operations.read().await.get(&operation_id).cloned()
    }

    pub async fn cleanup_completed(&self) {
        let mut ops = self.operations.write().await;
        ops.retain(|_, op| {
            let state = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { op.state.read().await.clone() })
            });
            matches!(state, AtomicState::InProgress)
        });
    }
}

impl AtomicOperation {
    pub async fn checkpoint(
        &self,
        name: &str,
        data: serde_json::Value,
        rollback_data: Option<serde_json::Value>,
    ) -> Result<()> {
        let checkpoint = Checkpoint {
            name: name.to_string(),
            timestamp: Utc::now(),
            data: data.clone(),
            rollback_data: rollback_data.clone(),
        };

        // Add to in-memory checkpoints
        self.checkpoints.write().await.push(checkpoint.clone());

        // Persist checkpoint to database
        let checkpoint_order = self.checkpoints.read().await.len() as i32;

        sqlx::query!(
            r#"
            INSERT INTO settlement_operation_checkpoints (
                operation_id, checkpoint_name, checkpoint_order,
                checkpoint_data, rollback_data, status
            ) VALUES ($1, $2, $3, $4, $5, 'completed')
            "#,
            self.id,
            checkpoint.name,
            checkpoint_order,
            checkpoint.data,
            rollback_data.unwrap_or(serde_json::json!({}))
        )
        .execute(&*self.db_pool)
        .await?;

        // Update current checkpoint in atomic operation
        sqlx::query!(
            r#"
            UPDATE settlement_atomic_operations
            SET current_checkpoint = $1,
                checkpoints = checkpoints || $2::jsonb
            WHERE id = $3
            "#,
            checkpoint.name,
            serde_json::to_value(&checkpoint)?,
            self.id
        )
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Checkpoint '{}' created for atomic operation {}",
            name, self.id
        );

        Ok(())
    }

    pub async fn commit(&self) -> Result<()> {
        let mut state = self.state.write().await;

        if !matches!(*state, AtomicState::InProgress) {
            return Err(SettlementError::InvalidState(format!(
                "Cannot commit operation in state {:?}",
                *state
            )));
        }

        *state = AtomicState::Committed;
        drop(state);

        // Update database
        sqlx::query!(
            r#"
            UPDATE settlement_atomic_operations
            SET state = 'Committed',
                completed_at = $1,
                committed_at = $1
            WHERE id = $2
            "#,
            Utc::now(),
            self.id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Atomic operation {} committed successfully", self.id);

        Ok(())
    }

    pub async fn rollback(&self, reason: &str) -> Result<()> {
        let mut state = self.state.write().await;

        if !matches!(*state, AtomicState::InProgress) {
            warn!(
                "Attempted to rollback operation {} in state {:?}",
                self.id, *state
            );
            return Ok(());
        }

        *state = AtomicState::RolledBack;
        drop(state);

        info!(
            "Rolling back atomic operation {}: {}",
            self.id, reason
        );

        // Execute rollback for each checkpoint in reverse order
        let checkpoints = self.checkpoints.read().await.clone();
        for checkpoint in checkpoints.iter().rev() {
            if let Err(e) = self.rollback_checkpoint(checkpoint).await {
                warn!(
                    "Error rolling back checkpoint '{}': {}",
                    checkpoint.name, e
                );
                // Continue with other checkpoints even if one fails
            }
        }

        // Update database
        sqlx::query!(
            r#"
            UPDATE settlement_atomic_operations
            SET state = 'RolledBack',
                completed_at = $1,
                rolled_back_at = $1,
                rollback_reason = $2
            WHERE id = $3
            "#,
            Utc::now(),
            reason,
            self.id
        )
        .execute(&*self.db_pool)
        .await?;

        // Mark all checkpoints as rolled back
        sqlx::query!(
            r#"
            UPDATE settlement_operation_checkpoints
            SET status = 'rolled_back',
                rolled_back_at = $1
            WHERE operation_id = $2
            "#,
            Utc::now(),
            self.id
        )
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Atomic operation {} rolled back successfully",
            self.id
        );

        Ok(())
    }

    async fn rollback_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        info!(
            "Rolling back checkpoint '{}' for operation {}",
            checkpoint.name, self.id
        );

        match checkpoint.name.as_str() {
            "funds_locked" => {
                // Release fund lock
                if let Some(rollback_data) = &checkpoint.rollback_data {
                    let lock_id: Uuid = serde_json::from_value(
                        rollback_data.get("lock_id").cloned()
                            .ok_or_else(|| SettlementError::Internal(
                                "Missing lock_id in rollback data".to_string()
                            ))?
                    )?;

                    self.release_fund_lock(lock_id).await?;
                }
            }
            "transfer_initiated" => {
                // Attempt to cancel external transfer
                if let Some(rollback_data) = &checkpoint.rollback_data {
                    let reference = rollback_data
                        .get("external_reference")
                        .and_then(|v| v.as_str());

                    if let Some(ref_str) = reference {
                        info!("Attempting to cancel transfer: {}", ref_str);
                        // This would call the bank API to cancel
                        // For MVP, we just log it
                    }
                }
            }
            "settlement_recorded" => {
                // Update settlement status to rolled back
                sqlx::query!(
                    r#"
                    UPDATE settlement_transactions
                    SET status = 'rolled_back',
                        rolled_back_at = $1
                    WHERE id = $2
                    "#,
                    Utc::now(),
                    self.settlement_id
                )
                .execute(&*self.db_pool)
                .await?;
            }
            _ => {
                info!("No specific rollback action for checkpoint '{}'", checkpoint.name);
            }
        }

        Ok(())
    }

    async fn release_fund_lock(&self, lock_id: Uuid) -> Result<()> {
        // Get lock details before releasing
        let lock = sqlx::query!(
            r#"
            SELECT nostro_account_id, amount, currency, bank
            FROM fund_locks
            WHERE id = $1 AND status = 'active'
            "#,
            lock_id
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| SettlementError::LockNotFound(lock_id.to_string()))?;

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
            lock_id
        )
        .execute(&*self.db_pool)
        .await?;

        // The trigger will automatically update nostro account available balance

        info!(
            "Released fund lock {} for {} {} on {}",
            lock_id, lock.amount, lock.currency, lock.bank
        );

        Ok(())
    }

    pub async fn get_state(&self) -> AtomicState {
        self.state.read().await.clone()
    }

    pub async fn get_checkpoints(&self) -> Vec<Checkpoint> {
        self.checkpoints.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a test database
    // They are marked as ignored and can be run with --ignored flag

    #[tokio::test]
    #[ignore]
    async fn test_atomic_operation_lifecycle() {
        // Test would require database setup
    }
}
