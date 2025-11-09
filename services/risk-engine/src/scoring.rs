use crate::errors::{RiskError, RiskResult};
use crate::models::*;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;
use tracing::info;

pub struct RiskScorer {
    // Could hold ML model reference in future
}

impl RiskScorer {
    pub fn new() -> Self {
        RiskScorer {}
    }

    pub async fn calculate_risk_score(
        &self,
        req: &RiskEvaluationRequest,
        pool: &PgPool,
    ) -> RiskResult<RiskScore> {
        let mut factors = Vec::new();
        let mut total_score = 0.0;

        // ===== 1. AMOUNT RISK (Weight: 25%) =====
        let amount_score = self.calculate_amount_risk(&req.amount);
        factors.push(RiskFactor {
            name: "Transaction Amount".to_string(),
            weight: 0.25,
            score: amount_score,
            reason: format!("Amount: {} {}", req.amount, req.from_currency),
        });
        total_score += amount_score * 0.25;

        // ===== 2. CORRIDOR RISK (Weight: 30%) =====
        let corridor_score = self.calculate_corridor_risk(
            &req.from_currency,
            &req.to_currency,
            &req.sender_country,
            &req.receiver_country,
        );
        factors.push(RiskFactor {
            name: "Corridor Risk".to_string(),
            weight: 0.30,
            score: corridor_score,
            reason: format!(
                "{}-{} to {}-{}",
                req.sender_country, req.from_currency, req.receiver_country, req.to_currency
            ),
        });
        total_score += corridor_score * 0.30;

        // ===== 3. VELOCITY RISK (Weight: 20%) =====
        let velocity_score = self.check_velocity(req.sender_bank_id, pool).await?;
        factors.push(RiskFactor {
            name: "Transaction Velocity".to_string(),
            weight: 0.20,
            score: velocity_score,
            reason: "Recent transaction pattern".to_string(),
        });
        total_score += velocity_score * 0.20;

        // ===== 4. BANK HISTORY (Weight: 15%) =====
        let history_score = self.evaluate_bank_history(req.sender_bank_id, pool).await?;
        factors.push(RiskFactor {
            name: "Bank History".to_string(),
            weight: 0.15,
            score: history_score,
            reason: "Historical performance".to_string(),
        });
        total_score += history_score * 0.15;

        // ===== 5. TRANSACTION TYPE RISK (Weight: 10%) =====
        let type_score = self.evaluate_transaction_type(&req.transaction_type);
        factors.push(RiskFactor {
            name: "Transaction Type".to_string(),
            weight: 0.10,
            score: type_score,
            reason: format!("{:?} transaction", req.transaction_type),
        });
        total_score += type_score * 0.10;

        // ===== DECISION LOGIC =====
        let decision = match total_score {
            s if s <= 25.0 => RiskDecision::Approve,
            s if s <= 50.0 => RiskDecision::ApproveWithLimit,
            s if s <= 75.0 => RiskDecision::Review,
            _ => RiskDecision::Reject,
        };

        let explanation = self.generate_explanation(&factors, &decision);
        let confidence = self.calculate_confidence(&factors);

        info!(
            "Risk score calculated: {:.2} (decision: {:?}) for transaction {}",
            total_score, decision, req.transaction_id
        );

        Ok(RiskScore {
            transaction_id: req.transaction_id,
            overall_score: total_score,
            factors,
            decision,
            confidence,
            explanation,
            calculated_at: Utc::now(),
        })
    }

    fn calculate_amount_risk(&self, amount: &Decimal) -> f64 {
        let amount_f64 = amount.to_string().parse::<f64>().unwrap_or(0.0);

        match amount_f64 {
            a if a <= 10_000.0 => 5.0,
            a if a <= 50_000.0 => 15.0,
            a if a <= 100_000.0 => 30.0,
            a if a <= 500_000.0 => 50.0,
            a if a <= 1_000_000.0 => 70.0,
            _ => 90.0,
        }
    }

    fn calculate_corridor_risk(
        &self,
        from_currency: &str,
        to_currency: &str,
        from_country: &str,
        to_country: &str,
    ) -> f64 {
        const HIGH_RISK_COUNTRIES: [&str; 10] = [
            "IR", "KP", "SY", "CU", "VE", "AF", "MM", "ZW", "SD", "BY",
        ];

        const MEDIUM_RISK_COUNTRIES: [&str; 15] = [
            "RU", "CN", "PK", "NG", "KE", "UG", "TZ", "ET", "BD", "LK", "NP", "KH", "LA", "MM",
            "PH",
        ];

        if HIGH_RISK_COUNTRIES.contains(&from_country)
            || HIGH_RISK_COUNTRIES.contains(&to_country)
        {
            return 95.0;
        }

        if MEDIUM_RISK_COUNTRIES.contains(&from_country)
            || MEDIUM_RISK_COUNTRIES.contains(&to_country)
        {
            return 60.0;
        }

        match (from_currency, to_currency) {
            ("USD", "EUR") | ("EUR", "USD") | ("GBP", "USD") => 5.0,
            ("INR", "AED") | ("AED", "INR") => 15.0,
            (_, _) if from_currency == to_currency => 10.0,
            _ => 25.0,
        }
    }

    async fn check_velocity(&self, bank_id: Uuid, pool: &PgPool) -> RiskResult<f64> {
        let hour_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transactions
             WHERE sender_bank_id = $1
             AND created_at > NOW() - INTERVAL '1 hour'",
        )
        .bind(bank_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let day_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transactions
             WHERE sender_bank_id = $1
             AND created_at > NOW() - INTERVAL '1 day'",
        )
        .bind(bank_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let hour_score = match hour_count {
            0..=5 => 5.0,
            6..=10 => 15.0,
            11..=20 => 35.0,
            21..=50 => 60.0,
            _ => 85.0,
        };

        let day_score = match day_count {
            0..=50 => 5.0,
            51..=100 => 20.0,
            101..=200 => 40.0,
            201..=500 => 65.0,
            _ => 90.0,
        };

        let velocity_score: f64 = hour_score * 0.7 + day_score * 0.3;
        Ok(velocity_score.min(100.0))
    }

    async fn evaluate_bank_history(&self, bank_id: Uuid, pool: &PgPool) -> RiskResult<f64> {
        let total_transactions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transactions
             WHERE sender_bank_id = $1
             AND created_at > NOW() - INTERVAL '30 days'",
        )
        .bind(bank_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if total_transactions == 0 {
            return Ok(40.0);
        }

        let failed_transactions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transactions
             WHERE sender_bank_id = $1
             AND status IN ('Failed', 'Rejected')
             AND created_at > NOW() - INTERVAL '30 days'",
        )
        .bind(bank_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let failure_rate = if total_transactions > 0 {
            (failed_transactions as f64 / total_transactions as f64) * 100.0
        } else {
            0.0
        };

        Ok(match failure_rate {
            r if r < 2.0 => 5.0,
            r if r < 5.0 => 15.0,
            r if r < 10.0 => 30.0,
            r if r < 20.0 => 60.0,
            _ => 90.0,
        })
    }

    fn evaluate_transaction_type(&self, tx_type: &TransactionType) -> f64 {
        match tx_type {
            TransactionType::Internal => 5.0,
            TransactionType::B2B => 15.0,
            TransactionType::B2C => 30.0,
            TransactionType::C2C => 50.0,
        }
    }

    fn generate_explanation(&self, factors: &[RiskFactor], decision: &RiskDecision) -> String {
        let high_risk_factors: Vec<&RiskFactor> = factors
            .iter()
            .filter(|f| f.score > 50.0)
            .collect();

        let decision_str = match decision {
            RiskDecision::Approve => "approved",
            RiskDecision::ApproveWithLimit => "approved with reduced limit",
            RiskDecision::Review => "flagged for manual review",
            RiskDecision::Reject => "rejected",
        };

        if high_risk_factors.is_empty() {
            format!("Transaction {} - all risk factors within acceptable range", decision_str)
        } else {
            let reasons: Vec<String> = high_risk_factors
                .iter()
                .map(|f| format!("{} (score: {:.1})", f.name, f.score))
                .collect();
            format!(
                "Transaction {} due to high risk in: {}",
                decision_str,
                reasons.join(", ")
            )
        }
    }

    fn calculate_confidence(&self, factors: &[RiskFactor]) -> f64 {
        let variance: f64 = factors
            .iter()
            .map(|f| {
                let dist_from_50 = (f.score - 50.0).abs();
                dist_from_50 / 50.0
            })
            .sum::<f64>()
            / factors.len() as f64;

        0.5 + (variance * 0.5)
    }

    pub async fn save_risk_score(&self, score: &RiskScore, pool: &PgPool) -> RiskResult<()> {
        let factors_json = serde_json::to_value(&score.factors)
            .map_err(|e| RiskError::InternalError(e.to_string()))?;

        sqlx::query(
            "INSERT INTO risk_scores (transaction_id, overall_score, decision, confidence, factors, explanation, calculated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (transaction_id) DO UPDATE
             SET overall_score = $2, decision = $3, confidence = $4, factors = $5, explanation = $6, calculated_at = $7"
        )
        .bind(score.transaction_id)
        .bind(score.overall_score)
        .bind(format!("{:?}", score.decision))
        .bind(score.confidence)
        .bind(factors_json)
        .bind(&score.explanation)
        .bind(score.calculated_at)
        .execute(pool)
        .await?;

        Ok(())
    }
}