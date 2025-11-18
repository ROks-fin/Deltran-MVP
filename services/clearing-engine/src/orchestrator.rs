// Clearing Orchestrator - Coordinates the entire clearing process

use crate::errors::{ClearingError, Result};
use crate::models::{NetPosition, SettlementInstruction, WindowStatus};
use crate::netting::NettingEngine;
use crate::window::WindowManager;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};
use uuid::Uuid;

/// Main orchestrator for clearing process
pub struct ClearingOrchestrator {
    window_manager: Arc<WindowManager>,
    db_pool: Arc<PgPool>,
    nats_client: Option<async_nats::Client>,
}

impl ClearingOrchestrator {
    pub fn new(
        window_manager: Arc<WindowManager>,
        db_pool: Arc<PgPool>,
        nats_client: Option<async_nats::Client>,
    ) -> Self {
        Self {
            window_manager,
            db_pool,
            nats_client,
        }
    }

    /// Execute complete clearing cycle for a window
    pub async fn execute_clearing(&self, window_id: i64) -> Result<ClearingResult> {
        info!("Starting clearing execution for window {}", window_id);

        let start_time = std::time::Instant::now();

        // Step 1: Validate window state
        let window = self.window_manager.get_window(window_id).await?;
        if window.status != WindowStatus::Processing.as_str() {
            return Err(ClearingError::InvalidWindowState {
                expected: WindowStatus::Processing.as_str().to_string(),
                actual: window.status.clone(),
            });
        }

        // Step 2: Collect obligations
        info!("Collecting obligations for window {}", window_id);
        let obligations = self.collect_obligations(window_id).await?;
        info!("Collected {} obligations", obligations.len());

        // Step 3: Build netting engine and add obligations
        let mut netting_engine = NettingEngine::new(window_id);
        for obligation in &obligations {
            netting_engine.add_obligation(
                obligation.currency.clone(),
                obligation.payer_id,
                obligation.payee_id,
                obligation.amount,
                obligation.id,
            )?;
        }

        // Step 4: Optimize (eliminate cycles)
        info!("Optimizing netting graph for window {}", window_id);
        let optimizer_stats = netting_engine.optimize()?;
        info!(
            "Optimization complete: {} cycles eliminated, {} saved",
            optimizer_stats.cycles_found, optimizer_stats.amount_eliminated
        );

        // Step 5: Calculate net positions
        info!("Calculating net positions for window {}", window_id);
        let net_positions = netting_engine.calculate_net_positions()?;
        info!("Calculated {} net positions", net_positions.len());

        // Step 6: Persist net positions
        self.save_net_positions(&net_positions).await?;

        // Step 7: Generate settlement instructions
        info!("Generating settlement instructions for window {}", window_id);
        let instructions = self.generate_settlement_instructions(&net_positions).await?;
        info!("Generated {} settlement instructions", instructions.len());

        // Step 8: Calculate metrics
        let stats = netting_engine.get_stats();
        let gross_value = self.calculate_gross_value(&obligations);
        let net_value = self.calculate_net_value(&net_positions);
        let efficiency = if gross_value > Decimal::ZERO {
            (gross_value - net_value)
                .checked_div(gross_value)
                .unwrap_or(Decimal::ZERO)
                * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        // Step 9: Update window metrics
        self.window_manager
            .update_metrics(
                window_id,
                obligations.len() as i32,
                gross_value,
                net_value,
                efficiency,
            )
            .await?;

        // Step 10: Update window status to Settling
        self.window_manager
            .update_status(window_id, WindowStatus::Settling)
            .await?;

        // Step 11: Publish clearing event to NATS
        if let Some(ref nats) = self.nats_client {
            self.publish_clearing_event(nats, window_id, &net_positions)
                .await?;
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        info!(
            "Clearing complete for window {} in {}ms",
            window_id, processing_time
        );

        Ok(ClearingResult {
            window_id,
            obligations_count: obligations.len(),
            net_positions_count: net_positions.len(),
            instructions_count: instructions.len(),
            gross_value,
            net_value,
            saved_amount: gross_value - net_value,
            efficiency_percent: efficiency,
            cycles_eliminated: optimizer_stats.cycles_found,
            processing_time_ms: processing_time,
        })
    }

    /// Collect all obligations for a window
    async fn collect_obligations(&self, window_id: i64) -> Result<Vec<Obligation>> {
        let obligations = sqlx::query_as::<_, Obligation>(
            r#"
            SELECT id, window_id, payer_id, payee_id, amount, currency, created_at
            FROM obligations
            WHERE window_id = $1 AND status = 'PENDING'
            ORDER BY created_at ASC
            "#,
        )
        .bind(window_id)
        .fetch_all(self.db_pool.as_ref())
        .await
        .map_err(|e| ClearingError::DatabaseError(e.to_string()))?;

        Ok(obligations)
    }

    /// Save net positions to database
    async fn save_net_positions(&self, positions: &[NetPosition]) -> Result<()> {
        for position in positions {
            sqlx::query(
                r#"
                INSERT INTO net_positions (
                    id, window_id, bank_pair_hash, bank_a_id, bank_b_id, currency,
                    gross_debit_a_to_b, gross_credit_b_to_a, net_amount, net_direction,
                    net_payer_id, net_receiver_id, obligations_netted, netting_ratio,
                    amount_saved, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                "#,
            )
            .bind(&position.id)
            .bind(&position.window_id)
            .bind(&position.bank_pair_hash)
            .bind(&position.bank_a_id)
            .bind(&position.bank_b_id)
            .bind(&position.currency)
            .bind(&position.gross_debit_a_to_b)
            .bind(&position.gross_credit_b_to_a)
            .bind(&position.net_amount)
            .bind(&position.net_direction)
            .bind(&position.net_payer_id)
            .bind(&position.net_receiver_id)
            .bind(&position.obligations_netted)
            .bind(&position.netting_ratio)
            .bind(&position.amount_saved)
            .bind(&position.created_at)
            .execute(self.db_pool.as_ref())
            .await
            .map_err(|e| ClearingError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    /// Generate settlement instructions from net positions
    async fn generate_settlement_instructions(
        &self,
        positions: &[NetPosition],
    ) -> Result<Vec<SettlementInstruction>> {
        let mut instructions = Vec::new();

        for position in positions {
            // Skip balanced positions
            if position.net_direction == "BALANCED" {
                continue;
            }

            // Skip zero amounts
            if position.net_amount == Decimal::ZERO {
                continue;
            }

            let (payer, payee) = if let (Some(payer), Some(payee)) = (position.net_payer_id, position.net_receiver_id) {
                (payer, payee)
            } else {
                continue;
            };

            let instruction = SettlementInstruction {
                id: Uuid::new_v4(),
                window_id: position.window_id,
                net_position_id: Some(position.id),
                payer_bank_id: payer,
                payee_bank_id: payee,
                amount: position.net_amount,
                currency: position.currency.clone(),
                instruction_type: "NET_SETTLEMENT".to_string(),
                priority: 1,
                deadline: Utc::now() + chrono::Duration::hours(2),
                status: "PENDING".to_string(),
                sent_to_settlement_at: None,
                settlement_id: None,
                instruction_data: serde_json::json!({
                    "bank_pair_hash": position.bank_pair_hash,
                    "obligations_netted": position.obligations_netted,
                }),
                created_at: Utc::now(),
            };

            // Save to database
            sqlx::query(
                r#"
                INSERT INTO settlement_instructions (
                    id, window_id, net_position_id, payer_bank_id, payee_bank_id,
                    amount, currency, instruction_type, priority, deadline, status,
                    instruction_data, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                "#,
            )
            .bind(&instruction.id)
            .bind(&instruction.window_id)
            .bind(&instruction.net_position_id)
            .bind(&instruction.payer_bank_id)
            .bind(&instruction.payee_bank_id)
            .bind(&instruction.amount)
            .bind(&instruction.currency)
            .bind(&instruction.instruction_type)
            .bind(&instruction.priority)
            .bind(&instruction.deadline)
            .bind(&instruction.status)
            .bind(&instruction.instruction_data)
            .bind(&instruction.created_at)
            .execute(self.db_pool.as_ref())
            .await
            .map_err(|e| ClearingError::DatabaseError(e.to_string()))?;

            instructions.push(instruction);
        }

        Ok(instructions)
    }

    /// Calculate total gross value
    fn calculate_gross_value(&self, obligations: &[Obligation]) -> Decimal {
        obligations
            .iter()
            .fold(Decimal::ZERO, |acc, o| acc + o.amount)
    }

    /// Calculate total net value
    fn calculate_net_value(&self, positions: &[NetPosition]) -> Decimal {
        positions
            .iter()
            .fold(Decimal::ZERO, |acc, p| acc + p.net_amount)
    }

    /// Publish clearing completion event to NATS
    async fn publish_clearing_event(
        &self,
        nats: &async_nats::Client,
        window_id: i64,
        positions: &[NetPosition],
    ) -> Result<()> {
        let event = serde_json::json!({
            "event_type": "clearing.completed",
            "window_id": window_id,
            "timestamp": Utc::now().to_rfc3339(),
            "positions_count": positions.len(),
        });

        nats.publish(
            "clearing.events.completed".to_string(),
            serde_json::to_vec(&event)
                .map_err(|e| ClearingError::Serialization(e))?
                .into(),
        )
        .await?;

        Ok(())
    }
}

/// Temporary obligation struct for querying
#[derive(Debug, Clone, sqlx::FromRow)]
struct Obligation {
    id: Uuid,
    window_id: i64,
    payer_id: Uuid,
    payee_id: Uuid,
    amount: Decimal,
    currency: String,
    created_at: chrono::DateTime<Utc>,
}

/// Result of clearing execution
#[derive(Debug, Clone)]
pub struct ClearingResult {
    pub window_id: i64,
    pub obligations_count: usize,
    pub net_positions_count: usize,
    pub instructions_count: usize,
    pub gross_value: Decimal,
    pub net_value: Decimal,
    pub saved_amount: Decimal,
    pub efficiency_percent: Decimal,
    pub cycles_eliminated: usize,
    pub processing_time_ms: u64,
}
