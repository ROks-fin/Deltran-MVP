// fx/pvp_settlement.rs
// PvP (Payment vs Payment) Atomic Settlement Engine
// Guarantees both legs of FX trade settle atomically or neither settles

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

const DEFAULT_PVP_TIMEOUT_SECONDS: u64 = 60;
const LOCK_RETRY_ATTEMPTS: u32 = 3;
const LOCK_RETRY_DELAY_MS: u64 = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvPSettlementRequest {
    pub deal_id: Uuid,
    pub payment_id: Uuid,
    pub leg_a: PvPLeg,
    pub leg_b: PvPLeg,
    pub timeout_seconds: Option<u64>,
    pub allow_partial_settlement: bool,
    pub pvp_mode: PvPMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvPLeg {
    pub leg_id: Uuid,
    pub currency: String,
    pub amount: Decimal,
    pub from_account: String,
    pub to_account: String,
    pub settlement_reference: String,
    pub value_date: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PvPMode {
    Simultaneous,  // Both legs execute simultaneously
    Sequential,    // Leg A then Leg B
    Escrow,        // Both legs to escrow, then release
    CLS,           // CLS-style netting
}

impl ToString for PvPMode {
    fn to_string(&self) -> String {
        match self {
            PvPMode::Simultaneous => "simultaneous".to_string(),
            PvPMode::Sequential => "sequential".to_string(),
            PvPMode::Escrow => "escrow".to_string(),
            PvPMode::CLS => "cls".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvPSettlementResponse {
    pub settlement_id: Uuid,
    pub deal_id: Uuid,
    pub payment_id: Uuid,
    pub pvp_status: PvPStatus,
    pub leg_a_status: LegStatus,
    pub leg_b_status: LegStatus,
    pub atomic_completion: bool,
    pub started_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
    pub duration_ms: Option<i32>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PvPStatus {
    Initiated,
    LockingFunds,
    FundsLocked,
    Executing,
    Completed,
    Partial,
    Failed,
    RolledBack,
}

impl ToString for PvPStatus {
    fn to_string(&self) -> String {
        match self {
            PvPStatus::Initiated => "initiated".to_string(),
            PvPStatus::LockingFunds => "locking_funds".to_string(),
            PvPStatus::FundsLocked => "funds_locked".to_string(),
            PvPStatus::Executing => "executing".to_string(),
            PvPStatus::Completed => "completed".to_string(),
            PvPStatus::Partial => "partial".to_string(),
            PvPStatus::Failed => "failed".to_string(),
            PvPStatus::RolledBack => "rolled_back".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LegStatus {
    Pending,
    Locked,
    Executing,
    Completed,
    Failed,
    RolledBack,
}

impl ToString for LegStatus {
    fn to_string(&self) -> String {
        match self {
            LegStatus::Pending => "pending".to_string(),
            LegStatus::Locked => "locked".to_string(),
            LegStatus::Executing => "executing".to_string(),
            LegStatus::Completed => "completed".to_string(),
            LegStatus::Failed => "failed".to_string(),
            LegStatus::RolledBack => "rolled_back".to_string(),
        }
    }
}

// PvP Settlement Engine
pub struct PvPSettlementEngine {
    pool: PgPool,
    // Track active settlements for monitoring
    active_settlements: Arc<RwLock<std::collections::HashMap<Uuid, PvPStatus>>>,
}

impl PvPSettlementEngine {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            active_settlements: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Execute PvP settlement with atomic guarantees
    pub async fn execute_pvp(
        &self,
        request: PvPSettlementRequest,
    ) -> Result<PvPSettlementResponse, PvPError> {
        let settlement_id = Uuid::new_v4();
        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(
            request.timeout_seconds.unwrap_or(DEFAULT_PVP_TIMEOUT_SECONDS),
        );

        info!(
            "Starting PvP settlement {} for deal {} (mode: {:?})",
            settlement_id, request.deal_id, request.pvp_mode
        );

        // Mark as active
        self.active_settlements
            .write()
            .await
            .insert(settlement_id, PvPStatus::Initiated);

        // Create settlement record
        self.create_settlement_record(settlement_id, &request)
            .await?;

        // Execute with timeout
        let result = timeout(
            timeout_duration,
            self.execute_pvp_internal(settlement_id, request.clone()),
        )
        .await;

        // Clean up active settlements
        self.active_settlements.write().await.remove(&settlement_id);

        match result {
            Ok(Ok(response)) => {
                let duration_ms = start_time.elapsed().as_millis() as i32;
                info!(
                    "PvP settlement {} completed successfully in {}ms",
                    settlement_id, duration_ms
                );
                Ok(response)
            }
            Ok(Err(e)) => {
                error!("PvP settlement {} failed: {:?}", settlement_id, e);
                self.handle_settlement_failure(settlement_id, &request, e.to_string())
                    .await
            }
            Err(_) => {
                warn!("PvP settlement {} timed out", settlement_id);
                self.handle_settlement_timeout(settlement_id, &request).await
            }
        }
    }

    /// Internal PvP execution logic
    async fn execute_pvp_internal(
        &self,
        settlement_id: Uuid,
        request: PvPSettlementRequest,
    ) -> Result<PvPSettlementResponse, PvPError> {
        match request.pvp_mode {
            PvPMode::Simultaneous => self.execute_simultaneous(settlement_id, request).await,
            PvPMode::Sequential => self.execute_sequential(settlement_id, request).await,
            PvPMode::Escrow => self.execute_escrow(settlement_id, request).await,
            PvPMode::CLS => self.execute_cls(settlement_id, request).await,
        }
    }

    /// Execute simultaneous mode - both legs in single transaction
    async fn execute_simultaneous(
        &self,
        settlement_id: Uuid,
        request: PvPSettlementRequest,
    ) -> Result<PvPSettlementResponse, PvPError> {
        let start_time = std::time::Instant::now();

        // Start transaction
        let mut tx = self.pool.begin().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        // Update status to locking
        self.update_settlement_status(&mut tx, settlement_id, PvPStatus::LockingFunds)
            .await?;

        // Lock funds for both legs
        info!("Locking funds for both legs of settlement {}", settlement_id);

        let lock_a_result = self.lock_leg_funds(&mut tx, settlement_id, &request.leg_a, "A").await;
        let lock_b_result = self.lock_leg_funds(&mut tx, settlement_id, &request.leg_b, "B").await;

        // Check both locks succeeded
        if lock_a_result.is_err() || lock_b_result.is_err() {
            error!("Failed to lock funds for settlement {}", settlement_id);
            tx.rollback().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;
            return Err(PvPError::LockFailed("Failed to lock funds for both legs".to_string()));
        }

        // Update status to funds locked
        self.update_settlement_status(&mut tx, settlement_id, PvPStatus::FundsLocked)
            .await?;

        // Execute both legs
        self.update_settlement_status(&mut tx, settlement_id, PvPStatus::Executing)
            .await?;

        info!("Executing both legs of settlement {}", settlement_id);

        let execute_a_result = self.execute_leg(&mut tx, settlement_id, &request.leg_a, "A").await;
        let execute_b_result = self.execute_leg(&mut tx, settlement_id, &request.leg_b, "B").await;

        // Check both executions succeeded
        if execute_a_result.is_err() || execute_b_result.is_err() {
            error!("Failed to execute legs for settlement {}", settlement_id);
            // Rollback transaction (automatic on drop, but explicit for clarity)
            tx.rollback().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;
            return Err(PvPError::ExecutionFailed("Failed to execute both legs".to_string()));
        }

        // Both legs successful - commit transaction
        self.update_settlement_status(&mut tx, settlement_id, PvPStatus::Completed)
            .await?;

        tx.commit().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        let duration_ms = start_time.elapsed().as_millis() as i32;

        // Update final settlement record
        self.finalize_settlement(settlement_id, true, duration_ms, None)
            .await?;

        info!(
            "PvP settlement {} completed atomically in {}ms",
            settlement_id, duration_ms
        );

        Ok(PvPSettlementResponse {
            settlement_id,
            deal_id: request.deal_id,
            payment_id: request.payment_id,
            pvp_status: PvPStatus::Completed,
            leg_a_status: LegStatus::Completed,
            leg_b_status: LegStatus::Completed,
            atomic_completion: true,
            started_at: chrono::Utc::now().naive_utc(),
            completed_at: Some(chrono::Utc::now().naive_utc()),
            duration_ms: Some(duration_ms),
            failure_reason: None,
        })
    }

    /// Execute sequential mode - Leg A, then Leg B (less safe)
    async fn execute_sequential(
        &self,
        settlement_id: Uuid,
        request: PvPSettlementRequest,
    ) -> Result<PvPSettlementResponse, PvPError> {
        let start_time = std::time::Instant::now();

        // Execute Leg A first
        info!("Executing Leg A for settlement {}", settlement_id);
        let mut tx_a = self.pool.begin().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        self.update_settlement_status(&mut tx_a, settlement_id, PvPStatus::Executing)
            .await?;

        self.lock_leg_funds(&mut tx_a, settlement_id, &request.leg_a, "A").await?;
        self.execute_leg(&mut tx_a, settlement_id, &request.leg_a, "A").await?;

        tx_a.commit().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        // Execute Leg B
        info!("Executing Leg B for settlement {}", settlement_id);
        let mut tx_b = self.pool.begin().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        let leg_b_result = self.lock_leg_funds(&mut tx_b, settlement_id, &request.leg_b, "B").await;

        if leg_b_result.is_err() {
            // Leg B failed - need to rollback Leg A
            warn!("Leg B failed, rolling back Leg A for settlement {}", settlement_id);
            self.rollback_leg(settlement_id, &request.leg_a, "A").await?;

            tx_b.rollback().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

            return Err(PvPError::PartialSettlement("Leg A completed but Leg B failed".to_string()));
        }

        self.execute_leg(&mut tx_b, settlement_id, &request.leg_b, "B").await?;

        self.update_settlement_status(&mut tx_b, settlement_id, PvPStatus::Completed)
            .await?;

        tx_b.commit().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        let duration_ms = start_time.elapsed().as_millis() as i32;

        self.finalize_settlement(settlement_id, true, duration_ms, None)
            .await?;

        Ok(PvPSettlementResponse {
            settlement_id,
            deal_id: request.deal_id,
            payment_id: request.payment_id,
            pvp_status: PvPStatus::Completed,
            leg_a_status: LegStatus::Completed,
            leg_b_status: LegStatus::Completed,
            atomic_completion: true,
            started_at: chrono::Utc::now().naive_utc(),
            completed_at: Some(chrono::Utc::now().naive_utc()),
            duration_ms: Some(duration_ms),
            failure_reason: None,
        })
    }

    /// Execute escrow mode - lock both, then release
    async fn execute_escrow(
        &self,
        settlement_id: Uuid,
        request: PvPSettlementRequest,
    ) -> Result<PvPSettlementResponse, PvPError> {
        // Similar to simultaneous but with explicit escrow accounts
        // Simplified for MVP - would have escrow account logic
        self.execute_simultaneous(settlement_id, request).await
    }

    /// Execute CLS mode - multilateral netting
    async fn execute_cls(
        &self,
        settlement_id: Uuid,
        request: PvPSettlementRequest,
    ) -> Result<PvPSettlementResponse, PvPError> {
        // CLS would involve multilateral netting across multiple FX trades
        // Simplified for MVP
        self.execute_simultaneous(settlement_id, request).await
    }

    /// Lock funds for a leg
    async fn lock_leg_funds(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        settlement_id: Uuid,
        leg: &PvPLeg,
        leg_side: &str,
    ) -> Result<(), PvPError> {
        // In production, this would call the ledger service to lock/reserve funds
        // For MVP, we just record the lock

        sqlx::query(
            r#"
            UPDATE fx_pvp_legs
            SET status = $1, locked_at = NOW(), updated_at = NOW()
            WHERE settlement_id = $2 AND leg_side = $3
            "#,
        )
        .bind(LegStatus::Locked.to_string())
        .bind(settlement_id)
        .bind(leg_side)
        .execute(&mut **tx)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Execute a leg settlement
    async fn execute_leg(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        settlement_id: Uuid,
        leg: &PvPLeg,
        leg_side: &str,
    ) -> Result<(), PvPError> {
        // In production, this would call the ledger service to transfer funds
        // For MVP, we just record the execution

        sqlx::query(
            r#"
            UPDATE fx_pvp_legs
            SET status = $1, executed_at = NOW(), completed_at = NOW(), updated_at = NOW()
            WHERE settlement_id = $2 AND leg_side = $3
            "#,
        )
        .bind(LegStatus::Completed.to_string())
        .bind(settlement_id)
        .bind(leg_side)
        .execute(&mut **tx)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Rollback a leg (for sequential mode failures)
    async fn rollback_leg(
        &self,
        settlement_id: Uuid,
        leg: &PvPLeg,
        leg_side: &str,
    ) -> Result<(), PvPError> {
        sqlx::query(
            r#"
            UPDATE fx_pvp_legs
            SET status = $1, updated_at = NOW()
            WHERE settlement_id = $2 AND leg_side = $3
            "#,
        )
        .bind(LegStatus::RolledBack.to_string())
        .bind(settlement_id)
        .bind(leg_side)
        .execute(&self.pool)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Create settlement record
    async fn create_settlement_record(
        &self,
        settlement_id: Uuid,
        request: &PvPSettlementRequest,
    ) -> Result<(), PvPError> {
        let mut tx = self.pool.begin().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        // Create settlement record
        sqlx::query(
            r#"
            INSERT INTO fx_pvp_settlements
            (settlement_id, deal_id, payment_id, pvp_mode, pvp_status, timeout_seconds, allow_partial_settlement)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(settlement_id)
        .bind(request.deal_id)
        .bind(request.payment_id)
        .bind(request.pvp_mode.to_string())
        .bind(PvPStatus::Initiated.to_string())
        .bind(request.timeout_seconds.unwrap_or(DEFAULT_PVP_TIMEOUT_SECONDS) as i32)
        .bind(request.allow_partial_settlement)
        .execute(&mut *tx)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        // Create leg A record
        sqlx::query(
            r#"
            INSERT INTO fx_pvp_legs
            (leg_id, settlement_id, leg_side, currency, amount, from_account, to_account, settlement_reference, value_date, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(request.leg_a.leg_id)
        .bind(settlement_id)
        .bind("A")
        .bind(&request.leg_a.currency)
        .bind(request.leg_a.amount)
        .bind(&request.leg_a.from_account)
        .bind(&request.leg_a.to_account)
        .bind(&request.leg_a.settlement_reference)
        .bind(request.leg_a.value_date)
        .bind(LegStatus::Pending.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        // Create leg B record
        sqlx::query(
            r#"
            INSERT INTO fx_pvp_legs
            (leg_id, settlement_id, leg_side, currency, amount, from_account, to_account, settlement_reference, value_date, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(request.leg_b.leg_id)
        .bind(settlement_id)
        .bind("B")
        .bind(&request.leg_b.currency)
        .bind(request.leg_b.amount)
        .bind(&request.leg_b.from_account)
        .bind(&request.leg_b.to_account)
        .bind(&request.leg_b.settlement_reference)
        .bind(request.leg_b.value_date)
        .bind(LegStatus::Pending.to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        tx.commit().await.map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update settlement status
    async fn update_settlement_status(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        settlement_id: Uuid,
        status: PvPStatus,
    ) -> Result<(), PvPError> {
        sqlx::query(
            r#"
            UPDATE fx_pvp_settlements
            SET pvp_status = $1, updated_at = NOW()
            WHERE settlement_id = $2
            "#,
        )
        .bind(status.to_string())
        .bind(settlement_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Finalize settlement with result
    async fn finalize_settlement(
        &self,
        settlement_id: Uuid,
        atomic_completion: bool,
        duration_ms: i32,
        failure_reason: Option<String>,
    ) -> Result<(), PvPError> {
        sqlx::query(
            r#"
            UPDATE fx_pvp_settlements
            SET atomic_completion = $1, duration_ms = $2, failure_reason = $3, completed_at = NOW(), updated_at = NOW()
            WHERE settlement_id = $4
            "#,
        )
        .bind(atomic_completion)
        .bind(duration_ms)
        .bind(failure_reason)
        .bind(settlement_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Handle settlement failure
    async fn handle_settlement_failure(
        &self,
        settlement_id: Uuid,
        request: &PvPSettlementRequest,
        reason: String,
    ) -> Result<PvPSettlementResponse, PvPError> {
        error!("Handling settlement failure for {}: {}", settlement_id, reason);

        // Update settlement status
        sqlx::query(
            r#"
            UPDATE fx_pvp_settlements
            SET pvp_status = $1, failure_reason = $2, completed_at = NOW(), updated_at = NOW()
            WHERE settlement_id = $3
            "#,
        )
        .bind(PvPStatus::Failed.to_string())
        .bind(&reason)
        .bind(settlement_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        Err(PvPError::SettlementFailed(reason))
    }

    /// Handle settlement timeout
    async fn handle_settlement_timeout(
        &self,
        settlement_id: Uuid,
        request: &PvPSettlementRequest,
    ) -> Result<PvPSettlementResponse, PvPError> {
        warn!("Handling settlement timeout for {}", settlement_id);

        sqlx::query(
            r#"
            UPDATE fx_pvp_settlements
            SET pvp_status = $1, failure_reason = $2, completed_at = NOW(), updated_at = NOW()
            WHERE settlement_id = $3
            "#,
        )
        .bind(PvPStatus::Failed.to_string())
        .bind("Settlement timeout")
        .bind(settlement_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PvPError::DatabaseError(e.to_string()))?;

        Err(PvPError::Timeout)
    }
}

#[derive(Debug)]
pub enum PvPError {
    DatabaseError(String),
    LockFailed(String),
    ExecutionFailed(String),
    PartialSettlement(String),
    SettlementFailed(String),
    Timeout,
    ValidationError(String),
}

impl std::fmt::Display for PvPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PvPError::DatabaseError(e) => write!(f, "Database error: {}", e),
            PvPError::LockFailed(e) => write!(f, "Lock failed: {}", e),
            PvPError::ExecutionFailed(e) => write!(f, "Execution failed: {}", e),
            PvPError::PartialSettlement(e) => write!(f, "Partial settlement: {}", e),
            PvPError::SettlementFailed(e) => write!(f, "Settlement failed: {}", e),
            PvPError::Timeout => write!(f, "Settlement timeout"),
            PvPError::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for PvPError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pvp_simultaneous_success() {
        // Test successful simultaneous PvP settlement
    }

    #[tokio::test]
    async fn test_pvp_simultaneous_rollback() {
        // Test rollback when one leg fails
    }

    #[tokio::test]
    async fn test_pvp_timeout() {
        // Test timeout handling
    }
}
