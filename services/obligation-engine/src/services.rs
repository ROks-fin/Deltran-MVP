use crate::database::Database;
use crate::errors::{ObligationEngineError, Result};
use crate::nats::NatsProducer;
use crate::models::{
    CreateInstantObligationRequest, InstantSettlementDecision, NettingResult, Obligation,
    ObligationEvent, ObligationEventType, ObligationResponse, SettleObligationsRequest,
};
use crate::netting::NettingEngine;
use crate::token_client::TokenEngineClient;
use chrono::Utc;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct ObligationService {
    pub db: Arc<Database>,
    pub nats: Arc<NatsProducer>,
    pub token_client: Arc<TokenEngineClient>,
    pub netting_engine: NettingEngine,
    pub redis: ConnectionManager,
    pub netting_efficiency_target: f64,
    pub liquidity_confidence_threshold: f64,
}

impl ObligationService {
    pub fn new(
        db: Arc<Database>,
        nats: Arc<NatsProducer>,
        token_client: Arc<TokenEngineClient>,
        redis: ConnectionManager,
        netting_efficiency_target: f64,
        liquidity_confidence_threshold: f64,
    ) -> Self {
        let netting_engine = NettingEngine::new(netting_efficiency_target);

        ObligationService {
            db,
            nats,
            token_client,
            netting_engine,
            redis,
            netting_efficiency_target,
            liquidity_confidence_threshold,
        }
    }

    /// Create instant obligation - CRITICAL for instant settlement
    pub async fn create_instant_obligation(
        &self,
        request: CreateInstantObligationRequest,
    ) -> Result<ObligationResponse> {
        // Validate request
        validator::Validate::validate(&request)
            .map_err(|e| ObligationEngineError::Validation(e.to_string()))?;

        info!(
            "Creating instant obligation for corridor {} amount {}",
            request.corridor, request.amount_sent
        );

        // Get current clearing window
        let clearing_window = self.db.get_current_clearing_window();

        // Check if instant settlement is possible
        let instant_decision = self
            .evaluate_instant_settlement(
                &request.corridor,
                request.amount_sent,
                request.bank_debtor_id,
                request.bank_creditor_id,
            )
            .await?;

        // Create obligation in database
        let obligation = self
            .db
            .create_obligation(
                &request.corridor,
                request.amount_sent,
                request.amount_credited,
                &request.sent_currency,
                &request.credited_currency,
                request.bank_debtor_id,
                request.bank_creditor_id,
                clearing_window,
                Some(request.transaction_id),
                &request.reference,
                request.metadata.clone(),
            )
            .await?;

        // Publish event to Kafka
        let event = ObligationEvent {
            event_type: ObligationEventType::Created,
            obligation_id: obligation.id,
            corridor: request.corridor.clone(),
            amount_sent: request.amount_sent,
            amount_credited: request.amount_credited,
            bank_debtor_id: request.bank_debtor_id,
            bank_creditor_id: request.bank_creditor_id,
            clearing_window,
            timestamp: Utc::now(),
            metadata: request.metadata,
        };

        if let Err(e) = self.nats.publish_obligation_event(&event).await {
            error!("Failed to publish obligation event: {}", e);
        }

        // If instant settlement approved, mint tokens for creditor
        if instant_decision.can_settle_instant {
            match self
                .execute_instant_settlement(
                    &obligation,
                    &request.credited_currency,
                    request.amount_credited,
                )
                .await
            {
                Ok(_) => {
                    info!(
                        "Instant settlement executed for obligation {}",
                        obligation.id
                    );
                }
                Err(e) => {
                    warn!(
                        "Instant settlement failed for obligation {}: {}",
                        obligation.id, e
                    );
                }
            }
        }

        let message = if instant_decision.can_settle_instant {
            "Instant settlement approved and executed".to_string()
        } else {
            format!(
                "Obligation created, instant settlement denied: {}",
                instant_decision.reason.clone().unwrap_or_default()
            )
        };

        Ok(ObligationResponse {
            obligation,
            instant_settlement: instant_decision,
            message,
        })
    }

    /// Evaluate if instant settlement is possible
    async fn evaluate_instant_settlement(
        &self,
        corridor: &str,
        amount: Decimal,
        _debtor_bank: Uuid,
        _creditor_bank: Uuid,
    ) -> Result<InstantSettlementDecision> {
        // Check cache first
        let cache_key = format!("instant_decision:{}:{}", corridor, amount);
        if let Ok(cached) = self.redis.clone().get::<String, String>(cache_key.clone()).await {
            if let Ok(decision) = serde_json::from_str::<InstantSettlementDecision>(&cached) {
                return Ok(decision);
            }
        }

        // Get corridor statistics for ML prediction
        let stats = self.db.get_corridor_stats(corridor, 7).await?;

        // Simple heuristic for instant settlement decision
        // In production, this would use ML model
        let can_settle = self.predict_instant_settlement_feasibility(corridor, amount, &stats);

        let confidence_score = if can_settle { 0.85 } else { 0.45 };

        let decision = InstantSettlementDecision {
            can_settle_instant: can_settle && confidence_score >= self.liquidity_confidence_threshold,
            confidence_score,
            expected_netting_offset: amount * Decimal::from_str("0.7").unwrap(), // 70% expected offset
            liquidity_available: Decimal::from_str("1000000").unwrap(), // Mock value
            risk_score: 0.15,
            reason: if can_settle {
                None
            } else {
                Some("Insufficient liquidity confidence".to_string())
            },
        };

        // Cache decision for 5 minutes
        let cached = serde_json::to_string(&decision)
            .map_err(|e| ObligationEngineError::Internal(e.to_string()))?;
        let _: () = self
            .redis
            .clone()
            .set_ex(cache_key, cached, 300)
            .await
            .map_err(|e| ObligationEngineError::Redis(e))?;

        Ok(decision)
    }

    /// Predict instant settlement feasibility
    fn predict_instant_settlement_feasibility(
        &self,
        _corridor: &str,
        amount: Decimal,
        stats: &serde_json::Value,
    ) -> bool {
        // Simple heuristic - in production this would be ML model
        // Check if corridor is bidirectional and has good volume
        let avg_volume = stats["avg_volume"]
            .as_str()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(Decimal::ZERO);

        // Approve if amount is less than 50% of average volume
        amount <= avg_volume / Decimal::from(2)
    }

    /// Execute instant settlement by minting tokens
    async fn execute_instant_settlement(
        &self,
        obligation: &Obligation,
        currency: &str,
        amount: Decimal,
    ) -> Result<()> {
        // Mint tokens for creditor bank
        self.token_client
            .mint_tokens(
                currency,
                amount,
                obligation.bank_creditor_id,
                &format!("obligation:{}", obligation.id),
                Some(serde_json::json!({
                    "obligation_id": obligation.id,
                    "instant_settlement": true,
                })),
            )
            .await?;

        info!(
            "Minted {} {} for creditor bank {} (obligation {})",
            amount, currency, obligation.bank_creditor_id, obligation.id
        );

        Ok(())
    }

    /// Calculate netting for a clearing window
    pub async fn calculate_netting(&self, clearing_window: i64) -> Result<NettingResult> {
        info!("Starting netting calculation for window {}", clearing_window);

        // Get all pending obligations for this window
        let obligations = self.db.get_pending_obligations(clearing_window).await?;

        if obligations.is_empty() {
            warn!("No pending obligations found for window {}", clearing_window);
            return Ok(NettingResult {
                clearing_window,
                total_obligations: 0,
                net_positions: Vec::new(),
                netting_efficiency: 0.0,
                gross_amount: Decimal::ZERO,
                net_amount: Decimal::ZERO,
                calculated_at: Utc::now(),
            });
        }

        // Calculate net positions
        let result = self
            .netting_engine
            .calculate_net_positions(&obligations, clearing_window)?;

        // Save net positions to database
        if !result.net_positions.is_empty() {
            self.db.save_net_positions(&result.net_positions).await?;
        }

        // Mark obligations as netted
        let obligation_ids: Vec<Uuid> = obligations.iter().map(|o| o.id).collect();
        let updated_count = self.db.mark_obligations_as_netted(&obligation_ids).await?;

        info!(
            "Netting completed: {} obligations processed, {} marked as netted, efficiency: {:.2}%",
            obligations.len(),
            updated_count,
            result.netting_efficiency * 100.0
        );

        // Publish netting result to Kafka
        let result_json = serde_json::to_value(&result)
            .map_err(|e| ObligationEngineError::Internal(e.to_string()))?;
        if let Err(e) = self.nats.publish_netting_result(clearing_window, &result_json).await {
            error!("Failed to publish netting result: {}", e);
        }

        Ok(result)
    }

    /// Settle obligations for a clearing window
    pub async fn settle_obligations(
        &self,
        request: SettleObligationsRequest,
    ) -> Result<serde_json::Value> {
        info!("Starting settlement for window {}", request.clearing_window);

        // Check if window is closed
        if !request.force_settlement && self.db.is_window_open(request.clearing_window) {
            return Err(ObligationEngineError::ClearingWindowClosed(
                request.clearing_window,
            ));
        }

        // First, calculate netting
        let netting_result = self.calculate_netting(request.clearing_window).await?;

        // Optimize settlement paths
        let settlement_paths = self
            .netting_engine
            .optimize_settlement_paths(&netting_result.net_positions)?;

        info!(
            "Generated {} settlement paths for window {}",
            settlement_paths.len(),
            request.clearing_window
        );

        // Create settlement instructions
        // In production, this would trigger actual settlement flows
        let mut settled_count = 0;
        for _path in &settlement_paths {
            // TODO: Create actual settlement instruction
            settled_count += 1;
        }

        Ok(serde_json::json!({
            "clearing_window": request.clearing_window,
            "netting_result": netting_result,
            "settlement_paths": settlement_paths,
            "settled_count": settled_count,
            "status": "completed",
        }))
    }

    /// Get obligation by ID
    pub async fn get_obligation(&self, obligation_id: Uuid) -> Result<Obligation> {
        self.db
            .get_obligation(obligation_id)
            .await?
            .ok_or_else(|| ObligationEngineError::ObligationNotFound(obligation_id))
    }

    /// Get all obligations for a clearing window
    pub async fn get_obligations_by_window(&self, clearing_window: i64) -> Result<Vec<Obligation>> {
        self.db.get_obligations_by_window(clearing_window).await
    }
}
