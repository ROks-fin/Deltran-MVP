use crate::errors::RiskResult;
use crate::models::{DynamicLimit, UpdateLimitRequest};
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use sqlx::{PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

pub struct LimitsManager {
    // Future: could cache limits in Redis
}

impl LimitsManager {
    pub fn new() -> Self {
        LimitsManager {}
    }

    /// Get current limit for a bank and corridor
    pub async fn get_limit(
        &self,
        bank_id: Uuid,
        corridor: &str,
        pool: &PgPool,
    ) -> RiskResult<DynamicLimit> {
        // Try to get existing dynamic limit
        let existing = sqlx::query_as::<_, DynamicLimitRow>(
            "SELECT bank_id, corridor, base_limit, current_limit, adjustment_factor, reason, valid_until
             FROM dynamic_limits
             WHERE bank_id = $1 AND corridor = $2 AND valid_until > NOW()",
        )
        .bind(bank_id)
        .bind(corridor)
        .fetch_optional(pool)
        .await?;

        if let Some(row) = existing {
            return Ok(DynamicLimit {
                bank_id: row.bank_id,
                corridor: row.corridor,
                base_limit: row.base_limit,
                current_limit: row.current_limit,
                adjustment_factor: row.adjustment_factor,
                reason: row.reason,
                valid_until: row.valid_until,
            });
        }

        // No existing limit, calculate new one
        self.calculate_dynamic_limit(bank_id, corridor, pool).await
    }

    /// Calculate dynamic limit based on bank performance
    pub async fn calculate_dynamic_limit(
        &self,
        bank_id: Uuid,
        corridor: &str,
        pool: &PgPool,
    ) -> RiskResult<DynamicLimit> {
        // Get base limit (default or configured)
        let base_limit = self.get_base_limit(corridor);

        // Calculate performance metrics
        let success_rate = self.calculate_success_rate(bank_id, pool).await?;
        let avg_risk_score = self.calculate_avg_risk_score(bank_id, pool).await?;
        let settlement_ratio = self.calculate_settlement_ratio(bank_id, pool).await?;

        // Adjustment factors
        let performance_factor = match success_rate {
            r if r >= 0.98 => 1.5,  // Excellent
            r if r >= 0.95 => 1.2,  // Good
            r if r >= 0.90 => 1.0,  // Normal
            r if r >= 0.85 => 0.8,  // Caution
            _ => 0.5,                // Restricted
        };

        let risk_factor = match avg_risk_score {
            s if s <= 20.0 => 1.3,
            s if s <= 40.0 => 1.1,
            s if s <= 60.0 => 0.9,
            s if s <= 80.0 => 0.7,
            _ => 0.4,
        };

        let final_factor = (performance_factor * risk_factor * settlement_ratio)
            .max(0.1)
            .min(2.0);

        let current_limit = base_limit * Decimal::from_f64(final_factor).unwrap();

        let limit = DynamicLimit {
            bank_id,
            corridor: corridor.to_string(),
            current_limit,
            base_limit,
            adjustment_factor: final_factor,
            reason: format!(
                "Performance: {:.0}%, Risk: {:.1}, Settlement: {:.0}%",
                success_rate * 100.0,
                avg_risk_score,
                settlement_ratio * 100.0
            ),
            valid_until: Utc::now() + Duration::hours(1),
        };

        // Save to database
        self.save_limit(&limit, pool).await?;

        Ok(limit)
    }

    /// Update base limit for a bank
    pub async fn update_limit(
        &self,
        bank_id: Uuid,
        req: UpdateLimitRequest,
        pool: &PgPool,
    ) -> RiskResult<()> {
        sqlx::query(
            "INSERT INTO dynamic_limits (bank_id, corridor, base_limit, current_limit, adjustment_factor, reason, valid_until)
             VALUES ($1, $2, $3, $3, 1.0, 'Manual update', NOW() + INTERVAL '1 day')
             ON CONFLICT (bank_id, corridor) DO UPDATE
             SET base_limit = $3, current_limit = $3, adjustment_factor = 1.0, reason = 'Manual update', valid_until = NOW() + INTERVAL '1 day'"
        )
        .bind(bank_id)
        .bind(&req.corridor)
        .bind(req.base_limit)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn save_limit(&self, limit: &DynamicLimit, pool: &PgPool) -> RiskResult<()> {
        sqlx::query(
            "INSERT INTO dynamic_limits (bank_id, corridor, base_limit, current_limit, adjustment_factor, reason, valid_until)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (bank_id, corridor) DO UPDATE
             SET current_limit = $4, adjustment_factor = $5, reason = $6, valid_until = $7"
        )
        .bind(limit.bank_id)
        .bind(&limit.corridor)
        .bind(limit.base_limit)
        .bind(limit.current_limit)
        .bind(limit.adjustment_factor)
        .bind(&limit.reason)
        .bind(limit.valid_until)
        .execute(pool)
        .await?;

        Ok(())
    }

    fn get_base_limit(&self, corridor: &str) -> Decimal {
        // Default limits per corridor
        match corridor {
            "INR-AED" | "AED-INR" => Decimal::from_str("10000000").unwrap(),
            "USD-EUR" | "EUR-USD" => Decimal::from_str("50000000").unwrap(),
            "USD-AED" | "AED-USD" => Decimal::from_str("20000000").unwrap(),
            _ => Decimal::from_str("5000000").unwrap(),
        }
    }

    async fn calculate_success_rate(&self, bank_id: Uuid, pool: &PgPool) -> RiskResult<f64> {
        let result = sqlx::query(
            "SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'Completed') as completed
             FROM transactions
             WHERE sender_bank_id = $1
             AND created_at > NOW() - INTERVAL '30 days'"
        )
        .bind(bank_id)
        .fetch_one(pool)
        .await?;

        let total: i64 = result.try_get("total").unwrap_or(0);
        if total == 0 {
            return Ok(0.9); // Default for new banks
        }

        let completed: i64 = result.try_get("completed").unwrap_or(0);
        Ok(completed as f64 / total as f64)
    }

    async fn calculate_avg_risk_score(&self, bank_id: Uuid, pool: &PgPool) -> RiskResult<f64> {
        let avg_score: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(rs.overall_score)
             FROM risk_scores rs
             JOIN transactions t ON t.id = rs.transaction_id
             WHERE t.sender_bank_id = $1
             AND rs.calculated_at > NOW() - INTERVAL '30 days'",
        )
        .bind(bank_id)
        .fetch_optional(pool)
        .await?
        .flatten();

        Ok(avg_score.unwrap_or(40.0)) // Default moderate risk
    }

    async fn calculate_settlement_ratio(&self, bank_id: Uuid, pool: &PgPool) -> RiskResult<f64> {
        let result = sqlx::query(
            "SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE settlement_type = 'Instant') as instant
             FROM transactions
             WHERE sender_bank_id = $1
             AND created_at > NOW() - INTERVAL '30 days'"
        )
        .bind(bank_id)
        .fetch_one(pool)
        .await?;

        let total: i64 = result.try_get("total").unwrap_or(0);
        if total == 0 {
            return Ok(1.0);
        }

        let instant: i64 = result.try_get("instant").unwrap_or(0);
        Ok((instant as f64 / total as f64).max(0.5)) // At least 0.5 factor
    }
}

#[derive(sqlx::FromRow)]
struct DynamicLimitRow {
    bank_id: Uuid,
    corridor: String,
    base_limit: Decimal,
    current_limit: Decimal,
    adjustment_factor: f64,
    reason: String,
    valid_until: chrono::DateTime<Utc>,
}
