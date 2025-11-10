use crate::config::Config;
use crate::error::{Result, SettlementError};
use crate::integration::{BankClientManager, PaymentRail, TransferRequest};
use crate::settlement::{AtomicController, AtomicOperation};
use crate::settlement::validator::SettlementValidator;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementRequest {
    pub id: Option<Uuid>,
    pub obligation_id: Uuid,
    pub from_bank: String,
    pub to_bank: String,
    pub amount: Decimal,
    pub currency: String,
    pub settlement_date: DateTime<Utc>,
    pub priority: SettlementPriority,
    pub method: PaymentRail,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SettlementPriority {
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementStatus {
    Pending,
    Validating,
    FundsLocked,
    Executing,
    Confirming,
    Completed,
    Failed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementResult {
    pub settlement_id: Uuid,
    pub status: SettlementStatus,
    pub external_reference: Option<String>,
    pub bank_confirmation: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

pub struct SettlementExecutor {
    db_pool: Arc<PgPool>,
    bank_clients: Arc<BankClientManager>,
    atomic_controller: Arc<AtomicController>,
    validator: Arc<SettlementValidator>,
    config: Arc<Config>,
}

impl SettlementExecutor {
    pub fn new(
        db_pool: Arc<PgPool>,
        bank_clients: Arc<BankClientManager>,
        atomic_controller: Arc<AtomicController>,
        validator: Arc<SettlementValidator>,
        config: Arc<Config>,
    ) -> Self {
        Self {
            db_pool,
            bank_clients,
            atomic_controller,
            validator,
            config,
        }
    }

    pub async fn execute_settlement(&self, request: SettlementRequest) -> Result<SettlementResult> {
        let settlement_id = request.id.unwrap_or_else(Uuid::new_v4);

        info!(
            "Executing settlement {} for obligation {}",
            settlement_id, request.obligation_id
        );

        // Create settlement record
        let _settlement = self.create_settlement_record(&request, settlement_id).await?;

        // Start atomic operation
        let atomic_op = self
            .atomic_controller
            .begin_operation(settlement_id)
            .await?;

        // Execute with automatic rollback on failure
        match self.perform_atomic_settlement(&request, settlement_id, &atomic_op).await {
            Ok(result) => {
                atomic_op.commit().await?;
                info!("Settlement {} completed successfully", settlement_id);
                Ok(result)
            }
            Err(e) => {
                error!("Settlement {} failed: {}", settlement_id, e);
                atomic_op.rollback(&e.to_string()).await?;

                // Update settlement status
                self.update_settlement_status(
                    settlement_id,
                    SettlementStatus::RolledBack,
                    Some(e.to_string()),
                ).await?;

                Err(e)
            }
        }
    }

    async fn create_settlement_record(
        &self,
        request: &SettlementRequest,
        settlement_id: Uuid,
    ) -> Result<Uuid> {
        sqlx::query!(
            r#"
            INSERT INTO settlement_transactions (
                id, obligation_id, from_bank, to_bank,
                amount, currency, status, priority,
                settlement_date, metadata, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            settlement_id,
            request.obligation_id,
            request.from_bank,
            request.to_bank,
            request.amount,
            request.currency,
            SettlementStatus::Pending.to_string(),
            serde_json::to_string(&request.priority)?,
            request.settlement_date.date_naive(),
            request.metadata,
            Utc::now()
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(settlement_id)
    }

    async fn perform_atomic_settlement(
        &self,
        request: &SettlementRequest,
        settlement_id: Uuid,
        atomic_op: &AtomicOperation,
    ) -> Result<SettlementResult> {
        // Step 1: Validate settlement prerequisites
        info!("Validating settlement {}", settlement_id);
        self.validator.validate_settlement(request).await?;
        self.update_settlement_status(settlement_id, SettlementStatus::Validating, None).await?;
        atomic_op
            .checkpoint("validation_complete", serde_json::json!({}), None)
            .await?;

        // Step 2: Lock funds in source account
        info!("Locking funds for settlement {}", settlement_id);
        let lock_id = self
            .lock_funds(&request.from_bank, &request.amount, &request.currency, settlement_id)
            .await?;
        self.update_settlement_status(settlement_id, SettlementStatus::FundsLocked, None).await?;

        atomic_op
            .checkpoint(
                "funds_locked",
                serde_json::json!({ "lock_id": lock_id }),
                Some(serde_json::json!({ "lock_id": lock_id })),
            )
            .await?;

        // Step 3: Initiate external transfer
        info!("Initiating external transfer for settlement {}", settlement_id);
        let transfer_ref = self.initiate_external_transfer(request, settlement_id).await?;
        self.update_settlement_status(settlement_id, SettlementStatus::Executing, None).await?;

        atomic_op
            .checkpoint(
                "transfer_initiated",
                serde_json::json!({ "reference": transfer_ref.clone() }),
                Some(serde_json::json!({ "external_reference": transfer_ref.clone() })),
            )
            .await?;

        // Step 4: Wait for confirmation with timeout
        info!("Awaiting confirmation for settlement {}", settlement_id);
        let confirmation = self
            .await_confirmation(&transfer_ref, settlement_id)
            .await?;
        self.update_settlement_status(settlement_id, SettlementStatus::Confirming, None).await?;

        atomic_op
            .checkpoint(
                "transfer_confirmed",
                serde_json::json!({ "confirmation": confirmation.clone() }),
                None,
            )
            .await?;

        // Step 5: Finalize settlement
        info!("Finalizing settlement {}", settlement_id);
        let result = self
            .finalize_settlement(settlement_id, &transfer_ref, &confirmation, lock_id)
            .await?;

        atomic_op
            .checkpoint(
                "settlement_finalized",
                serde_json::json!({ "settlement_id": settlement_id }),
                None,
            )
            .await?;

        Ok(result)
    }

    async fn lock_funds(
        &self,
        bank: &str,
        amount: &Decimal,
        currency: &str,
        settlement_id: Uuid,
    ) -> Result<Uuid> {
        let lock_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::seconds(self.config.settlement.fund_lock_expiry_seconds as i64);

        // Get nostro account
        let account = sqlx::query!(
            r#"
            SELECT id, ledger_balance, available_balance
            FROM nostro_accounts
            WHERE bank = $1 AND currency = $2 AND is_active = true
            "#,
            bank,
            currency
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| SettlementError::AccountNotFound(format!("{}:{}", bank, currency)))?;

        // Check sufficient balance
        if account.available_balance < *amount {
            return Err(SettlementError::InsufficientFunds {
                required: *amount,
                available: account.available_balance,
            });
        }

        // Create fund lock
        sqlx::query!(
            r#"
            INSERT INTO fund_locks (
                id, nostro_account_id, settlement_id, amount, currency,
                bank, status, locked_at, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, 'active', $7, $8)
            "#,
            lock_id,
            account.id,
            settlement_id,
            amount,
            currency,
            bank,
            Utc::now(),
            expires_at
        )
        .execute(&*self.db_pool)
        .await?;

        // Update available balance (locked amount reduces available)
        sqlx::query!(
            r#"
            UPDATE nostro_accounts
            SET available_balance = available_balance - $1,
                locked_balance = locked_balance + $1
            WHERE id = $2
            "#,
            amount,
            account.id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Locked {} {} for settlement {} (lock: {})", amount, currency, settlement_id, lock_id);

        Ok(lock_id)
    }

    async fn initiate_external_transfer(
        &self,
        request: &SettlementRequest,
        settlement_id: Uuid,
    ) -> Result<String> {
        let bank_client = self.bank_clients.get_client(&request.method);

        let transfer_request = TransferRequest {
            settlement_id,
            from_bank: request.from_bank.clone(),
            to_bank: request.to_bank.clone(),
            amount: request.amount,
            currency: request.currency.clone(),
            reference: format!("SETTLEMENT-{}", settlement_id),
            metadata: request.metadata.clone(),
        };

        let transfer_result = bank_client.initiate_transfer(&transfer_request).await?;

        // Store external reference
        sqlx::query!(
            r#"
            UPDATE settlement_transactions
            SET external_reference = $1,
                executed_at = $2
            WHERE id = $3
            "#,
            transfer_result.external_reference,
            Utc::now(),
            settlement_id
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(transfer_result.external_reference)
    }

    async fn await_confirmation(
        &self,
        external_reference: &str,
        settlement_id: Uuid,
    ) -> Result<String> {
        let timeout = Duration::seconds(self.config.settlement.default_timeout_seconds as i64);
        let start = Utc::now();

        loop {
            // Check if timeout exceeded
            if Utc::now() - start > timeout {
                return Err(SettlementError::TransferTimeout(
                    self.config.settlement.default_timeout_seconds,
                ));
            }

            // Poll bank for status
            // In MVP, we simulate this with a delay
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            // For MVP, we'll check the settlement_transactions table
            // In production, this would poll the external bank API
            let status = sqlx::query!(
                r#"
                SELECT status, bank_confirmation
                FROM settlement_transactions
                WHERE id = $1
                "#,
                settlement_id
            )
            .fetch_one(&*self.db_pool)
            .await?;

            // For MVP with mock bank, assume confirmation after short delay
            if Utc::now() - start > Duration::seconds(5) {
                let confirmation_code = format!("CONF-{}", Uuid::new_v4());
                return Ok(confirmation_code);
            }
        }
    }

    async fn finalize_settlement(
        &self,
        settlement_id: Uuid,
        external_reference: &str,
        confirmation: &str,
        lock_id: Uuid,
    ) -> Result<SettlementResult> {
        let completed_at = Utc::now();

        // Update settlement to completed
        sqlx::query!(
            r#"
            UPDATE settlement_transactions
            SET status = $1,
                bank_confirmation = $2,
                completed_at = $3
            WHERE id = $4
            "#,
            SettlementStatus::Completed.to_string(),
            confirmation,
            completed_at,
            settlement_id
        )
        .execute(&*self.db_pool)
        .await?;

        // Release and apply the fund lock
        self.release_and_apply_lock(lock_id).await?;

        Ok(SettlementResult {
            settlement_id,
            status: SettlementStatus::Completed,
            external_reference: Some(external_reference.to_string()),
            bank_confirmation: Some(confirmation.to_string()),
            completed_at: Some(completed_at),
            error_message: None,
        })
    }

    async fn release_and_apply_lock(&self, lock_id: Uuid) -> Result<()> {
        // Get lock details
        let lock = sqlx::query!(
            r#"
            SELECT nostro_account_id, amount, currency
            FROM fund_locks
            WHERE id = $1 AND status = 'active'
            "#,
            lock_id
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| SettlementError::LockNotFound(lock_id.to_string()))?;

        // Start transaction
        let mut tx = self.db_pool.begin().await?;

        // Update lock status to settled
        sqlx::query!(
            r#"
            UPDATE fund_locks
            SET status = 'settled',
                released_at = $1,
                released_by = 'settlement_complete'
            WHERE id = $2
            "#,
            Utc::now(),
            lock_id
        )
        .execute(&mut *tx)
        .await?;

        // Deduct from ledger balance and unlock from locked balance
        sqlx::query!(
            r#"
            UPDATE nostro_accounts
            SET ledger_balance = ledger_balance - $1,
                locked_balance = locked_balance - $1
            WHERE id = $2
            "#,
            lock.amount,
            lock.nostro_account_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        info!("Released and applied fund lock {}", lock_id);

        Ok(())
    }

    async fn update_settlement_status(
        &self,
        settlement_id: Uuid,
        status: SettlementStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE settlement_transactions
            SET status = $1,
                error_message = $2
            WHERE id = $3
            "#,
            status.to_string(),
            error_message,
            settlement_id
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn get_settlement_status(&self, settlement_id: Uuid) -> Result<SettlementResult> {
        let settlement = sqlx::query!(
            r#"
            SELECT id, status, external_reference, bank_confirmation,
                   completed_at, error_message
            FROM settlement_transactions
            WHERE id = $1
            "#,
            settlement_id
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| SettlementError::Internal(format!("Settlement {} not found", settlement_id)))?;

        Ok(SettlementResult {
            settlement_id,
            status: SettlementStatus::from_str(&settlement.status)
                .unwrap_or(SettlementStatus::Pending),
            external_reference: settlement.external_reference,
            bank_confirmation: settlement.bank_confirmation,
            completed_at: settlement.completed_at,
            error_message: settlement.error_message,
        })
    }
}

impl FromStr for SettlementStatus {
    type Err = SettlementError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "PENDING" => Ok(SettlementStatus::Pending),
            "VALIDATING" => Ok(SettlementStatus::Validating),
            "FUNDS_LOCKED" | "FUNDSLOCKED" => Ok(SettlementStatus::FundsLocked),
            "EXECUTING" => Ok(SettlementStatus::Executing),
            "CONFIRMING" => Ok(SettlementStatus::Confirming),
            "COMPLETED" => Ok(SettlementStatus::Completed),
            "FAILED" => Ok(SettlementStatus::Failed),
            "ROLLED_BACK" | "ROLLEDBACK" => Ok(SettlementStatus::RolledBack),
            _ => Err(SettlementError::Internal(format!("Unknown status: {}", s))),
        }
    }
}

impl ToString for SettlementStatus {
    fn to_string(&self) -> String {
        match self {
            SettlementStatus::Pending => "PENDING".to_string(),
            SettlementStatus::Validating => "VALIDATING".to_string(),
            SettlementStatus::FundsLocked => "FUNDS_LOCKED".to_string(),
            SettlementStatus::Executing => "EXECUTING".to_string(),
            SettlementStatus::Confirming => "CONFIRMING".to_string(),
            SettlementStatus::Completed => "COMPLETED".to_string(),
            SettlementStatus::Failed => "FAILED".to_string(),
            SettlementStatus::RolledBack => "ROLLED_BACK".to_string(),
        }
    }
}
