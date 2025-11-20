// CAMT.054 Handler - Processes bank credit/debit notifications for settlement confirmations

use crate::confirmation::{BankConfirmation, MatchConfidence, UetrMatcher};
use crate::error::{Result, SettlementError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Simplified CAMT.054 notification structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camt054Notification {
    pub message_id: String,
    pub creation_date_time: String,
    pub account_id: String,
    pub currency: String,
    pub credit_debit_indicator: String,  // CRDT or DBIT
    pub amount: String,
    pub booking_date: Option<String>,
    pub value_date: Option<String>,
    pub bank_reference: String,
    pub end_to_end_id: Option<String>,   // UETR
    pub transaction_id: Option<String>,
}

pub struct Camt054Handler {
    pool: PgPool,
    uetr_matcher: Arc<UetrMatcher>,
}

impl Camt054Handler {
    pub fn new(pool: PgPool, uetr_matcher: Arc<UetrMatcher>) -> Self {
        Self {
            pool,
            uetr_matcher,
        }
    }

    /// Process incoming CAMT.054 notification for settlement confirmation
    pub async fn process_notification(
        &self,
        notification: Camt054Notification,
    ) -> Result<ProcessingResult> {
        info!(
            "Processing CAMT.054 notification {} for account {}",
            notification.message_id, notification.account_id
        );

        // Only process CREDIT notifications (debits are for outgoing, handled separately)
        if notification.credit_debit_indicator != "CRDT" {
            info!(
                "Skipping DEBIT notification {} (settlements are credit-based)",
                notification.message_id
            );
            return Ok(ProcessingResult {
                processed: false,
                settlement_id: None,
                action_taken: "Skipped DEBIT notification".to_string(),
            });
        }

        // Parse amount
        let amount = Decimal::from_str(&notification.amount)
            .map_err(|e| SettlementError::Internal(format!("Invalid amount: {}", e)))?;

        // Parse execution timestamp
        let execution_timestamp = chrono::DateTime::parse_from_rfc3339(&notification.creation_date_time)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        // Create bank confirmation object
        let confirmation = BankConfirmation {
            uetr: notification.end_to_end_id.clone(),
            bank_reference: notification.bank_reference.clone(),
            amount,
            currency: notification.currency.clone(),
            beneficiary_account: Some(notification.account_id.clone()),
            execution_timestamp,
            status: "COMPLETED".to_string(),
        };

        // Attempt to match with pending settlement
        let match_result = self.uetr_matcher.match_confirmation(&confirmation).await?;

        if match_result.matched {
            let settlement_id = match_result.settlement_id.unwrap();

            info!(
                "Matched CAMT.054 to settlement {}: confidence={:?}, {}",
                settlement_id,
                match_result.confidence,
                match_result.match_details
            );

            // Update settlement with confirmation
            self.uetr_matcher
                .update_settlement_confirmation(settlement_id, &confirmation)
                .await?;

            // If exact or high confidence match, auto-finalize
            if matches!(
                match_result.confidence,
                MatchConfidence::Exact | MatchConfidence::High
            ) {
                info!("Auto-finalizing settlement {} (high confidence)", settlement_id);
                self.finalize_settlement(settlement_id).await?;
            } else {
                warn!(
                    "Settlement {} requires manual review (medium/low confidence match)",
                    settlement_id
                );
                self.flag_for_review(settlement_id, &match_result).await?;
            }

            Ok(ProcessingResult {
                processed: true,
                settlement_id: Some(settlement_id),
                action_taken: format!(
                    "Matched and updated settlement {} ({:?})",
                    settlement_id, match_result.confidence
                ),
            })
        } else {
            // No match found - store for manual reconciliation
            warn!(
                "No matching settlement for CAMT.054 notification {}",
                notification.message_id
            );

            let unmatched_id = self
                .uetr_matcher
                .store_unmatched_confirmation(&confirmation)
                .await?;

            Ok(ProcessingResult {
                processed: false,
                settlement_id: None,
                action_taken: format!(
                    "Stored as unmatched confirmation {} for manual review",
                    unmatched_id
                ),
            })
        }
    }

    /// Finalize settlement after confirmation
    async fn finalize_settlement(&self, settlement_id: uuid::Uuid) -> Result<()> {
        // Update settlement status to COMPLETED
        sqlx::query(
            r#"
            UPDATE settlement_transactions
            SET status = 'COMPLETED',
                completed_at = $1
            WHERE id = $2 AND status = 'CONFIRMING'
            "#,
        )
        .bind(Utc::now())
        .bind(settlement_id)
        .execute(&self.pool)
        .await?;

        info!("Finalized settlement {}", settlement_id);

        // TODO: Trigger Token Engine to burn tokens
        // TODO: Update Obligation Engine to mark obligation as SETTLED
        // TODO: Publish settlement_completed event to NATS

        Ok(())
    }

    /// Flag settlement for manual review
    async fn flag_for_review(
        &self,
        settlement_id: uuid::Uuid,
        match_result: &crate::confirmation::uetr_matcher::MatchResult,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE settlement_transactions
            SET metadata = jsonb_set(
                COALESCE(metadata, '{}'::jsonb),
                '{requires_review}',
                'true'
            ),
            metadata = jsonb_set(
                metadata,
                '{review_reason}',
                $1::jsonb
            )
            WHERE id = $2
            "#,
        )
        .bind(serde_json::to_value(&match_result.match_details)?)
        .bind(settlement_id)
        .execute(&self.pool)
        .await?;

        info!(
            "Flagged settlement {} for manual review: {}",
            settlement_id, match_result.match_details
        );

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessingResult {
    pub processed: bool,
    pub settlement_id: Option<uuid::Uuid>,
    pub action_taken: String,
}
