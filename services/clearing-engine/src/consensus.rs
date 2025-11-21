//! Consensus Service Module
//! Provides transaction consensus logic across all services
//!
//! This module implements the voting-based consensus model where each service
//! contributes a decision and the final outcome is computed based on priority:
//! 1. Compliance (veto power)
//! 2. Risk (veto power)
//! 3. Balance (blocking)
//! 4. Advisory (liquidity, clearing)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, FromRow};
use uuid::Uuid;
use rust_decimal::Decimal;
use tracing::{info, warn};

/// Decision status from each service
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceDecision {
    Approve,
    Review,
    Reject,
    Pending,
}

impl ServiceDecision {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "approve" | "approved" => Self::Approve,
            "review" | "reviewrequired" | "hold" => Self::Review,
            "reject" | "rejected" => Self::Reject,
            _ => Self::Pending,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "Approve",
            Self::Review => "Review",
            Self::Reject => "Reject",
            Self::Pending => "Pending",
        }
    }
}

/// Final consensus decision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FinalDecision {
    RejectedCompliance,
    RejectedRisk,
    RejectedInsufficientFunds,
    PendingReview,
    ApprovedPendingSettlement,
    SettlementInProgress,
    SettlementFailed,
    Settled,
    Processing,
}

impl FinalDecision {
    pub fn from_str(s: &str) -> Self {
        match s {
            "REJECTED_COMPLIANCE" => Self::RejectedCompliance,
            "REJECTED_RISK" => Self::RejectedRisk,
            "REJECTED_INSUFFICIENT_FUNDS" => Self::RejectedInsufficientFunds,
            "PENDING_REVIEW" => Self::PendingReview,
            "APPROVED_PENDING_SETTLEMENT" => Self::ApprovedPendingSettlement,
            "SETTLEMENT_IN_PROGRESS" => Self::SettlementInProgress,
            "SETTLEMENT_FAILED" => Self::SettlementFailed,
            "SETTLED" => Self::Settled,
            _ => Self::Processing,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RejectedCompliance => "REJECTED_COMPLIANCE",
            Self::RejectedRisk => "REJECTED_RISK",
            Self::RejectedInsufficientFunds => "REJECTED_INSUFFICIENT_FUNDS",
            Self::PendingReview => "PENDING_REVIEW",
            Self::ApprovedPendingSettlement => "APPROVED_PENDING_SETTLEMENT",
            Self::SettlementInProgress => "SETTLEMENT_IN_PROGRESS",
            Self::SettlementFailed => "SETTLEMENT_FAILED",
            Self::Settled => "SETTLED",
            Self::Processing => "PROCESSING",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::RejectedCompliance
                | Self::RejectedRisk
                | Self::RejectedInsufficientFunds
                | Self::Settled
                | Self::SettlementFailed
        )
    }

    pub fn is_successful(&self) -> bool {
        matches!(self, Self::Settled | Self::ApprovedPendingSettlement)
    }
}

/// Individual service decisions for a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDecisions {
    pub transaction_id: Uuid,

    // Compliance decision
    pub compliance_status: Option<ServiceDecision>,
    pub compliance_risk_rating: Option<String>,
    pub compliance_checked_at: Option<DateTime<Utc>>,

    // Risk decision
    pub risk_decision: Option<ServiceDecision>,
    pub risk_score: Option<Decimal>,
    pub risk_confidence: Option<Decimal>,
    pub risk_evaluated_at: Option<DateTime<Utc>>,

    // Token balance check
    pub token_balance_sufficient: Option<bool>,
    pub token_balance_available: Option<Decimal>,
    pub token_balance_required: Option<Decimal>,
    pub token_checked_at: Option<DateTime<Utc>>,

    // Liquidity recommendation
    pub liquidity_can_instant_settle: Option<bool>,
    pub liquidity_recommendation: Option<String>,
    pub liquidity_predicted_at: Option<DateTime<Utc>>,

    // Clearing status
    pub clearing_status: Option<String>,
    pub clearing_batch_id: Option<Uuid>,
    pub clearing_processed_at: Option<DateTime<Utc>>,

    // Settlement status
    pub settlement_status: Option<String>,
    pub settlement_instruction_id: Option<Uuid>,
    pub settlement_settled_at: Option<DateTime<Utc>>,

    // Final computed decision
    pub final_decision: FinalDecision,
    pub decision_reason: Option<String>,
    pub decided_at: Option<DateTime<Utc>>,
}

impl TransactionDecisions {
    pub fn new(transaction_id: Uuid) -> Self {
        Self {
            transaction_id,
            compliance_status: None,
            compliance_risk_rating: None,
            compliance_checked_at: None,
            risk_decision: None,
            risk_score: None,
            risk_confidence: None,
            risk_evaluated_at: None,
            token_balance_sufficient: None,
            token_balance_available: None,
            token_balance_required: None,
            token_checked_at: None,
            liquidity_can_instant_settle: None,
            liquidity_recommendation: None,
            liquidity_predicted_at: None,
            clearing_status: None,
            clearing_batch_id: None,
            clearing_processed_at: None,
            settlement_status: None,
            settlement_instruction_id: None,
            settlement_settled_at: None,
            final_decision: FinalDecision::Processing,
            decision_reason: None,
            decided_at: None,
        }
    }

    /// Compute final decision based on all service inputs
    /// Priority: Compliance > Risk > Balance > Advisory
    pub fn compute_final_decision(&mut self) -> FinalDecision {
        // Priority 1: Compliance rejection (veto)
        if matches!(self.compliance_status, Some(ServiceDecision::Reject)) {
            self.final_decision = FinalDecision::RejectedCompliance;
            self.decision_reason = Some("Transaction rejected by compliance check".to_string());
            self.decided_at = Some(Utc::now());
            return self.final_decision.clone();
        }

        // Priority 2: Risk rejection (veto)
        if matches!(self.risk_decision, Some(ServiceDecision::Reject)) {
            self.final_decision = FinalDecision::RejectedRisk;
            self.decision_reason = Some(format!(
                "Transaction rejected by risk engine (score: {})",
                self.risk_score.unwrap_or_default()
            ));
            self.decided_at = Some(Utc::now());
            return self.final_decision.clone();
        }

        // Priority 3: Insufficient funds (blocking)
        if matches!(self.token_balance_sufficient, Some(false)) {
            self.final_decision = FinalDecision::RejectedInsufficientFunds;
            self.decision_reason = Some(format!(
                "Insufficient balance: required {}, available {}",
                self.token_balance_required.unwrap_or_default(),
                self.token_balance_available.unwrap_or_default()
            ));
            self.decided_at = Some(Utc::now());
            return self.final_decision.clone();
        }

        // Priority 4: Manual review required
        if matches!(self.compliance_status, Some(ServiceDecision::Review))
            || matches!(self.risk_decision, Some(ServiceDecision::Review))
        {
            self.final_decision = FinalDecision::PendingReview;
            self.decision_reason = Some("Manual review required".to_string());
            return self.final_decision.clone();
        }

        // Priority 5: Settlement status
        if let Some(ref status) = self.settlement_status {
            match status.as_str() {
                "Settled" => {
                    self.final_decision = FinalDecision::Settled;
                    self.decision_reason = Some("Transaction successfully settled".to_string());
                    self.decided_at = Some(Utc::now());
                    return self.final_decision.clone();
                }
                "InProgress" | "Processing" => {
                    self.final_decision = FinalDecision::SettlementInProgress;
                    self.decision_reason = Some("Settlement in progress".to_string());
                    return self.final_decision.clone();
                }
                "Failed" => {
                    self.final_decision = FinalDecision::SettlementFailed;
                    self.decision_reason = Some("Settlement failed".to_string());
                    self.decided_at = Some(Utc::now());
                    return self.final_decision.clone();
                }
                _ => {}
            }
        }

        // Priority 6: All approved, pending settlement
        if matches!(self.compliance_status, Some(ServiceDecision::Approve))
            && matches!(self.risk_decision, Some(ServiceDecision::Approve))
            && matches!(self.token_balance_sufficient, Some(true))
        {
            self.final_decision = FinalDecision::ApprovedPendingSettlement;
            self.decision_reason = Some("Transaction approved, awaiting settlement".to_string());
            return self.final_decision.clone();
        }

        // Default: Still processing
        self.final_decision = FinalDecision::Processing;
        self.decision_reason = Some("Transaction is being processed".to_string());
        self.final_decision.clone()
    }

    /// Check if all required decisions are present
    pub fn is_decision_complete(&self) -> bool {
        self.compliance_status.is_some()
            && self.risk_decision.is_some()
            && self.token_balance_sufficient.is_some()
    }

    /// Check if transaction can proceed to settlement
    pub fn can_proceed_to_settlement(&self) -> bool {
        matches!(
            self.final_decision,
            FinalDecision::ApprovedPendingSettlement | FinalDecision::Processing
        ) && self.is_decision_complete()
            && matches!(self.compliance_status, Some(ServiceDecision::Approve))
            && matches!(self.risk_decision, Some(ServiceDecision::Approve))
            && matches!(self.token_balance_sufficient, Some(true))
    }
}

/// Database row for transaction decisions
#[derive(Debug, FromRow)]
struct DecisionRow {
    transaction_id: Uuid,
    compliance_status: Option<String>,
    risk_decision: Option<String>,
    risk_score: Option<Decimal>,
    token_balance_sufficient: Option<bool>,
    settlement_status: Option<String>,
    final_decision: String,
}

/// Fast consensus check result
#[derive(Debug, FromRow)]
struct FastCheckRow {
    final_decision: Option<String>,
    can_proceed: Option<bool>,
    blocking_service: Option<String>,
}

/// Consensus service for coordinating decisions across services
pub struct ConsensusService {
    pool: PgPool,
}

impl ConsensusService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get or create transaction decisions record
    pub async fn get_or_create_decisions(
        &self,
        transaction_id: Uuid,
    ) -> Result<TransactionDecisions, sqlx::Error> {
        // Try to get existing decisions
        let row: Option<DecisionRow> = sqlx::query_as(
            r#"
            SELECT
                transaction_id,
                compliance_status,
                risk_decision,
                risk_score,
                token_balance_sufficient,
                settlement_status,
                final_decision
            FROM transaction_decisions
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let mut decisions = TransactionDecisions::new(transaction_id);
                if let Some(cs) = r.compliance_status {
                    decisions.compliance_status = Some(ServiceDecision::from_str(&cs));
                }
                if let Some(rd) = r.risk_decision {
                    decisions.risk_decision = Some(ServiceDecision::from_str(&rd));
                }
                decisions.risk_score = r.risk_score;
                decisions.token_balance_sufficient = r.token_balance_sufficient;
                decisions.settlement_status = r.settlement_status;
                decisions.final_decision = FinalDecision::from_str(&r.final_decision);
                Ok(decisions)
            }
            None => {
                // Create new record
                sqlx::query(
                    r#"
                    INSERT INTO transaction_decisions (transaction_id, final_decision)
                    VALUES ($1, 'PROCESSING')
                    ON CONFLICT (transaction_id) DO NOTHING
                    "#,
                )
                .bind(transaction_id)
                .execute(&self.pool)
                .await?;

                Ok(TransactionDecisions::new(transaction_id))
            }
        }
    }

    /// Update compliance decision
    pub async fn update_compliance_decision(
        &self,
        transaction_id: Uuid,
        status: ServiceDecision,
        risk_rating: Option<String>,
    ) -> Result<FinalDecision, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE transaction_decisions
            SET compliance_status = $2,
                compliance_risk_rating = $3,
                compliance_checked_at = NOW()
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .bind(status.as_str())
        .bind(risk_rating)
        .execute(&self.pool)
        .await?;

        self.get_final_decision(transaction_id).await
    }

    /// Update risk decision
    pub async fn update_risk_decision(
        &self,
        transaction_id: Uuid,
        decision: ServiceDecision,
        score: Decimal,
        confidence: Decimal,
    ) -> Result<FinalDecision, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE transaction_decisions
            SET risk_decision = $2,
                risk_score = $3,
                risk_confidence = $4,
                risk_evaluated_at = NOW()
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .bind(decision.as_str())
        .bind(score)
        .bind(confidence)
        .execute(&self.pool)
        .await?;

        self.get_final_decision(transaction_id).await
    }

    /// Update token balance check
    pub async fn update_balance_check(
        &self,
        transaction_id: Uuid,
        is_sufficient: bool,
        available: Decimal,
        required: Decimal,
    ) -> Result<FinalDecision, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE transaction_decisions
            SET token_balance_sufficient = $2,
                token_balance_available = $3,
                token_balance_required = $4,
                token_checked_at = NOW()
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .bind(is_sufficient)
        .bind(available)
        .bind(required)
        .execute(&self.pool)
        .await?;

        self.get_final_decision(transaction_id).await
    }

    /// Update settlement status
    pub async fn update_settlement_status(
        &self,
        transaction_id: Uuid,
        status: &str,
        instruction_id: Option<Uuid>,
    ) -> Result<FinalDecision, sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE transaction_decisions
            SET settlement_status = $2,
                settlement_instruction_id = $3,
                settlement_settled_at = CASE WHEN $2 = 'Settled' THEN NOW() ELSE NULL END
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .bind(status)
        .bind(instruction_id)
        .execute(&self.pool)
        .await?;

        self.get_final_decision(transaction_id).await
    }

    /// Get current final decision (uses database trigger to compute)
    pub async fn get_final_decision(
        &self,
        transaction_id: Uuid,
    ) -> Result<FinalDecision, sqlx::Error> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"SELECT final_decision FROM transaction_decisions WHERE transaction_id = $1"#,
        )
        .bind(transaction_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((fd,)) => Ok(FinalDecision::from_str(&fd)),
            None => Ok(FinalDecision::Processing),
        }
    }

    /// Fast consensus check using optimized database function
    pub async fn fast_check(
        &self,
        transaction_id: Uuid,
    ) -> Result<(FinalDecision, bool, Option<String>), sqlx::Error> {
        let row: FastCheckRow = sqlx::query_as(
            r#"SELECT final_decision, can_proceed, blocking_service FROM fast_consensus_check($1)"#,
        )
        .bind(transaction_id)
        .fetch_one(&self.pool)
        .await?;

        Ok((
            FinalDecision::from_str(&row.final_decision.unwrap_or_default()),
            row.can_proceed.unwrap_or(false),
            row.blocking_service,
        ))
    }

    /// Log event for audit trail
    pub async fn log_event(
        &self,
        transaction_id: Uuid,
        service_name: &str,
        event_type: &str,
        decision: Option<&str>,
        payload: serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO transaction_events (transaction_id, service_name, event_type, decision, payload)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(transaction_id)
        .bind(service_name)
        .bind(event_type)
        .bind(decision)
        .bind(payload)
        .execute(&self.pool)
        .await?;

        info!(
            "Event logged: {} {} {} for transaction {}",
            service_name, event_type, decision.unwrap_or("N/A"), transaction_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_final_decision_approved() {
        let mut decisions = TransactionDecisions::new(Uuid::new_v4());
        decisions.compliance_status = Some(ServiceDecision::Approve);
        decisions.risk_decision = Some(ServiceDecision::Approve);
        decisions.token_balance_sufficient = Some(true);

        let result = decisions.compute_final_decision();
        assert_eq!(result, FinalDecision::ApprovedPendingSettlement);
    }

    #[test]
    fn test_compute_final_decision_rejected_compliance() {
        let mut decisions = TransactionDecisions::new(Uuid::new_v4());
        decisions.compliance_status = Some(ServiceDecision::Reject);
        decisions.risk_decision = Some(ServiceDecision::Approve);
        decisions.token_balance_sufficient = Some(true);

        let result = decisions.compute_final_decision();
        assert_eq!(result, FinalDecision::RejectedCompliance);
    }

    #[test]
    fn test_compute_final_decision_rejected_risk() {
        let mut decisions = TransactionDecisions::new(Uuid::new_v4());
        decisions.compliance_status = Some(ServiceDecision::Approve);
        decisions.risk_decision = Some(ServiceDecision::Reject);
        decisions.token_balance_sufficient = Some(true);

        let result = decisions.compute_final_decision();
        assert_eq!(result, FinalDecision::RejectedRisk);
    }

    #[test]
    fn test_compute_final_decision_insufficient_funds() {
        let mut decisions = TransactionDecisions::new(Uuid::new_v4());
        decisions.compliance_status = Some(ServiceDecision::Approve);
        decisions.risk_decision = Some(ServiceDecision::Approve);
        decisions.token_balance_sufficient = Some(false);

        let result = decisions.compute_final_decision();
        assert_eq!(result, FinalDecision::RejectedInsufficientFunds);
    }

    #[test]
    fn test_can_proceed_to_settlement() {
        let mut decisions = TransactionDecisions::new(Uuid::new_v4());
        decisions.compliance_status = Some(ServiceDecision::Approve);
        decisions.risk_decision = Some(ServiceDecision::Approve);
        decisions.token_balance_sufficient = Some(true);
        decisions.compute_final_decision();

        assert!(decisions.can_proceed_to_settlement());
    }

    #[test]
    fn test_is_terminal_decision() {
        assert!(FinalDecision::Settled.is_terminal());
        assert!(FinalDecision::RejectedCompliance.is_terminal());
        assert!(!FinalDecision::Processing.is_terminal());
        assert!(!FinalDecision::ApprovedPendingSettlement.is_terminal());
    }
}
