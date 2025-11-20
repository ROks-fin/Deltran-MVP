// UETR Matcher - Matches bank confirmations with pending settlements

use crate::error::{Result, SettlementError};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankConfirmation {
    pub uetr: Option<String>,                // End-to-end transaction reference
    pub bank_reference: String,              // Bank's internal reference
    pub amount: Decimal,
    pub currency: String,
    pub beneficiary_account: Option<String>,
    pub execution_timestamp: DateTime<Utc>,
    pub status: String,                      // COMPLETED, FAILED, PENDING
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub matched: bool,
    pub settlement_id: Option<Uuid>,
    pub confidence: MatchConfidence,
    pub match_details: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchConfidence {
    Exact,      // Perfect match on UETR + amount + currency
    High,       // Match on bank_reference + amount + currency
    Medium,     // Match on amount + currency + time window
    Low,        // Partial match, manual review needed
    None,       // No match found
}

pub struct UetrMatcher {
    pool: PgPool,
    match_tolerance_seconds: i64,
    amount_tolerance_percentage: Decimal,
}

impl UetrMatcher {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            match_tolerance_seconds: 1800, // 30 minutes
            amount_tolerance_percentage: Decimal::new(1, 2), // 0.01 = 1%
        }
    }

    /// Match incoming bank confirmation with pending settlement
    pub async fn match_confirmation(
        &self,
        confirmation: &BankConfirmation,
    ) -> Result<MatchResult> {
        info!(
            "Attempting to match bank confirmation: bank_ref={}, amount={} {}",
            confirmation.bank_reference,
            confirmation.amount,
            confirmation.currency
        );

        // Strategy 1: Exact UETR match (highest confidence)
        if let Some(uetr) = &confirmation.uetr {
            if let Some(settlement_id) = self.match_by_uetr(uetr).await? {
                info!("EXACT match found by UETR: {} -> {}", uetr, settlement_id);
                return Ok(MatchResult {
                    matched: true,
                    settlement_id: Some(settlement_id),
                    confidence: MatchConfidence::Exact,
                    match_details: format!("Matched by UETR: {}", uetr),
                });
            }
        }

        // Strategy 2: Bank reference match (high confidence)
        if let Some(settlement_id) = self.match_by_bank_reference(
            &confirmation.bank_reference,
            confirmation.amount,
            &confirmation.currency,
        ).await? {
            info!(
                "HIGH confidence match found by bank_reference: {} -> {}",
                confirmation.bank_reference, settlement_id
            );
            return Ok(MatchResult {
                matched: true,
                settlement_id: Some(settlement_id),
                confidence: MatchConfidence::High,
                match_details: format!("Matched by bank_reference: {}", confirmation.bank_reference),
            });
        }

        // Strategy 3: Fuzzy match by amount + currency + time window (medium confidence)
        if let Some(settlement_id) = self.match_by_amount_and_time(
            confirmation.amount,
            &confirmation.currency,
            confirmation.execution_timestamp,
        ).await? {
            warn!(
                "MEDIUM confidence match by amount+time: {} {} -> {}",
                confirmation.amount, confirmation.currency, settlement_id
            );
            return Ok(MatchResult {
                matched: true,
                settlement_id: Some(settlement_id),
                confidence: MatchConfidence::Medium,
                match_details: format!(
                    "Fuzzy match: {} {} around {}",
                    confirmation.amount,
                    confirmation.currency,
                    confirmation.execution_timestamp
                ),
            });
        }

        // No match found
        warn!(
            "No match found for bank confirmation: bank_ref={}, amount={} {}",
            confirmation.bank_reference, confirmation.amount, confirmation.currency
        );

        Ok(MatchResult {
            matched: false,
            settlement_id: None,
            confidence: MatchConfidence::None,
            match_details: "No matching settlement found".to_string(),
        })
    }

    /// Match by UETR (Unique End-to-End Transaction Reference)
    async fn match_by_uetr(&self, uetr: &str) -> Result<Option<Uuid>> {
        let result = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT id
            FROM settlement_transactions
            WHERE metadata->>'uetr' = $1
              AND status IN ('EXECUTING', 'CONFIRMING')
            LIMIT 1
            "#,
        )
        .bind(uetr)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(id,)| id))
    }

    /// Match by bank reference + amount + currency
    async fn match_by_bank_reference(
        &self,
        bank_reference: &str,
        amount: Decimal,
        currency: &str,
    ) -> Result<Option<Uuid>> {
        let amount_tolerance = amount * self.amount_tolerance_percentage;

        let result = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT id
            FROM settlement_transactions
            WHERE external_reference = $1
              AND currency = $2
              AND amount BETWEEN $3 AND $4
              AND status IN ('EXECUTING', 'CONFIRMING')
            LIMIT 1
            "#,
        )
        .bind(bank_reference)
        .bind(currency)
        .bind(amount - amount_tolerance)
        .bind(amount + amount_tolerance)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(id,)| id))
    }

    /// Fuzzy match by amount + currency + time window
    async fn match_by_amount_and_time(
        &self,
        amount: Decimal,
        currency: &str,
        execution_time: DateTime<Utc>,
    ) -> Result<Option<Uuid>> {
        let amount_tolerance = amount * self.amount_tolerance_percentage;
        let time_window_start = execution_time - Duration::seconds(self.match_tolerance_seconds);
        let time_window_end = execution_time + Duration::seconds(self.match_tolerance_seconds);

        let result = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT id
            FROM settlement_transactions
            WHERE currency = $1
              AND amount BETWEEN $2 AND $3
              AND executed_at BETWEEN $4 AND $5
              AND status IN ('EXECUTING', 'CONFIRMING')
            ORDER BY ABS(EXTRACT(EPOCH FROM (executed_at - $6)))
            LIMIT 1
            "#,
        )
        .bind(currency)
        .bind(amount - amount_tolerance)
        .bind(amount + amount_tolerance)
        .bind(time_window_start)
        .bind(time_window_end)
        .bind(execution_time)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(id,)| id))
    }

    /// Store unmatched confirmation for manual review
    pub async fn store_unmatched_confirmation(
        &self,
        confirmation: &BankConfirmation,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO unmatched_confirmations (
                id, uetr, bank_reference, amount, currency,
                beneficiary_account, execution_timestamp, status,
                created_at, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(id)
        .bind(&confirmation.uetr)
        .bind(&confirmation.bank_reference)
        .bind(confirmation.amount)
        .bind(&confirmation.currency)
        .bind(&confirmation.beneficiary_account)
        .bind(confirmation.execution_timestamp)
        .bind(&confirmation.status)
        .bind(Utc::now())
        .bind(serde_json::to_value(confirmation)?)
        .execute(&self.pool)
        .await?;

        info!("Stored unmatched confirmation {} for manual review", id);
        Ok(id)
    }

    /// Update settlement with confirmation details
    pub async fn update_settlement_confirmation(
        &self,
        settlement_id: Uuid,
        confirmation: &BankConfirmation,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE settlement_transactions
            SET bank_confirmation = $1,
                status = CASE
                    WHEN $2 = 'COMPLETED' THEN 'COMPLETED'
                    WHEN $2 = 'FAILED' THEN 'FAILED'
                    ELSE status
                END,
                completed_at = CASE
                    WHEN $2 = 'COMPLETED' THEN $3
                    ELSE completed_at
                END,
                metadata = jsonb_set(
                    COALESCE(metadata, '{}'::jsonb),
                    '{confirmation}',
                    $4::jsonb
                )
            WHERE id = $5
            "#,
        )
        .bind(&confirmation.bank_reference)
        .bind(&confirmation.status)
        .bind(Utc::now())
        .bind(serde_json::to_value(confirmation)?)
        .bind(settlement_id)
        .execute(&self.pool)
        .await?;

        info!(
            "Updated settlement {} with confirmation from bank_ref={}",
            settlement_id, confirmation.bank_reference
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_match_confidence_ordering() {
        assert!(MatchConfidence::Exact > MatchConfidence::High);
        assert!(MatchConfidence::High > MatchConfidence::Medium);
        assert!(MatchConfidence::Medium > MatchConfidence::Low);
        assert!(MatchConfidence::Low > MatchConfidence::None);
    }
}

impl PartialOrd for MatchConfidence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MatchConfidence {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let self_score = match self {
            MatchConfidence::Exact => 5,
            MatchConfidence::High => 4,
            MatchConfidence::Medium => 3,
            MatchConfidence::Low => 2,
            MatchConfidence::None => 1,
        };

        let other_score = match other {
            MatchConfidence::Exact => 5,
            MatchConfidence::High => 4,
            MatchConfidence::Medium => 3,
            MatchConfidence::Low => 2,
            MatchConfidence::None => 1,
        };

        self_score.cmp(&other_score)
    }
}

impl Eq for MatchConfidence {}
