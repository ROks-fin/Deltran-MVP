use crate::errors::ComplianceResult;
use crate::models::*;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

pub struct AmlScorer {}

impl AmlScorer {
    pub fn new() -> Self {
        AmlScorer {}
    }

    pub async fn calculate_aml_risk(
        &self,
        req: &ComplianceCheckRequest,
        db: &PgPool,
    ) -> ComplianceResult<AmlResult> {
        let mut risk_factors = Vec::new();
        let mut total_score = 0.0;

        // ===== 1. Transaction Amount Risk (Weight: 20%) =====
        let amount_risk = self.assess_amount_risk(&req.amount, &req.currency);
        risk_factors.push(AmlRiskFactor {
            factor_type: "Transaction Amount".to_string(),
            weight: 0.20,
            score: amount_risk,
            description: format!("{} {} transaction", req.amount, req.currency),
        });
        total_score += amount_risk * 0.20;

        // ===== 2. Geographic Risk (Weight: 25%) =====
        let geo_risk = self.assess_geographic_risk(
            &req.sender_country,
            &req.receiver_country
        );
        risk_factors.push(AmlRiskFactor {
            factor_type: "Geographic Risk".to_string(),
            weight: 0.25,
            score: geo_risk,
            description: format!("{} to {} corridor",
                req.sender_country, req.receiver_country),
        });
        total_score += geo_risk * 0.25;

        // ===== 3. Customer Risk (Weight: 15%) =====
        let customer_risk = self.assess_customer_risk(
            req.sender_bank_id,
            db
        ).await?;
        risk_factors.push(AmlRiskFactor {
            factor_type: "Customer Risk".to_string(),
            weight: 0.15,
            score: customer_risk,
            description: "Based on historical behavior".to_string(),
        });
        total_score += customer_risk * 0.15;

        // ===== 4. Transaction Pattern Risk (Weight: 25%) =====
        let pattern_risk = self.detect_suspicious_patterns(
            req.sender_bank_id,
            &req.amount,
            db
        ).await?;
        risk_factors.push(AmlRiskFactor {
            factor_type: "Pattern Risk".to_string(),
            weight: 0.25,
            score: pattern_risk.risk_score,
            description: "Transaction pattern analysis".to_string(),
        });
        total_score += pattern_risk.risk_score * 0.25;

        // ===== 5. Purpose/Description Risk (Weight: 15%) =====
        let purpose_risk = self.assess_purpose_risk(&req.purpose);
        risk_factors.push(AmlRiskFactor {
            factor_type: "Purpose Risk".to_string(),
            weight: 0.15,
            score: purpose_risk,
            description: req.purpose.clone()
                .unwrap_or_else(|| "No purpose provided".to_string()),
        });
        total_score += purpose_risk * 0.15;

        // Determine if reports are required
        let amount_f64 = req.amount.to_string().parse::<f64>().unwrap_or(0.0);
        let requires_ctr = req.currency == "USD" && amount_f64 > 10000.0;
        let requires_sar = total_score > 70.0 || !pattern_risk.suspicious_patterns.is_empty();

        Ok(AmlResult {
            risk_score: total_score,
            risk_factors,
            suspicious_patterns: pattern_risk.suspicious_patterns,
            requires_sar,
            requires_ctr,
        })
    }

    fn assess_amount_risk(&self, amount: &Decimal, currency: &str) -> f64 {
        let amount_f64 = amount.to_string().parse::<f64>().unwrap_or(0.0);

        // Structuring risk (just below reporting thresholds)
        if currency == "USD" && (9000.0..10000.0).contains(&amount_f64) {
            return 85.0; // Suspicious - near CTR threshold
        }

        match amount_f64 {
            a if a < 1000.0 => 5.0,
            a if a < 5000.0 => 15.0,
            a if a < 10000.0 => 30.0,
            a if a < 50000.0 => 50.0,
            a if a < 100000.0 => 70.0,
            _ => 90.0,
        }
    }

    fn assess_geographic_risk(&self, from_country: &str, to_country: &str) -> f64 {
        const HIGH_RISK_COUNTRIES: [&str; 20] = [
            "AF", "AL", "BS", "BB", "BW", "KH", "GH", "IS", "MN", "PA",
            "PK", "LK", "SY", "TT", "UG", "VU", "YE", "ZW", "IR", "KP"
        ];

        const MEDIUM_RISK_COUNTRIES: [&str; 30] = [
            "DZ", "AO", "BD", "BO", "BF", "BI", "CM", "CF", "TD", "CN",
            "CG", "CD", "EC", "EG", "SV", "GQ", "ER", "ET", "GA", "GM",
            "GN", "GW", "HT", "HN", "IN", "ID", "IQ", "JM", "KZ", "KE"
        ];

        let from_risk = if HIGH_RISK_COUNTRIES.contains(&from_country) {
            90.0
        } else if MEDIUM_RISK_COUNTRIES.contains(&from_country) {
            60.0
        } else {
            20.0
        };

        let to_risk = if HIGH_RISK_COUNTRIES.contains(&to_country) {
            90.0
        } else if MEDIUM_RISK_COUNTRIES.contains(&to_country) {
            60.0
        } else {
            20.0
        };

        let geo_score: f64 = from_risk * 0.6 + to_risk * 0.4;
        geo_score.min(100.0)
    }

    async fn assess_customer_risk(
        &self,
        bank_id: Uuid,
        db: &PgPool,
    ) -> ComplianceResult<f64> {
        // Check historical compliance
        let failed_transactions = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM transactions
             WHERE sender_bank_id = $1
             AND status IN ('Failed', 'Rejected')
             AND created_at > NOW() - INTERVAL '30 days'"
        )
        .bind(bank_id)
        .fetch_one(db)
        .await
        .unwrap_or(0);

        let risk = match failed_transactions {
            0..=2 => 10.0,
            3..=5 => 30.0,
            6..=10 => 50.0,
            11..=20 => 70.0,
            _ => 90.0,
        };

        Ok(risk)
    }

    async fn detect_suspicious_patterns(
        &self,
        bank_id: Uuid,
        amount: &Decimal,
        db: &PgPool,
    ) -> ComplianceResult<PatternDetectionResult> {
        let mut patterns = Vec::new();
        let mut risk_score = 0.0;

        // ===== Rapid Movement Pattern =====
        let rapid_movement = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM transactions
             WHERE sender_bank_id = $1
             AND created_at > NOW() - INTERVAL '1 hour'
             AND status = 'Completed'"
        )
        .bind(bank_id)
        .fetch_one(db)
        .await
        .unwrap_or(0);

        if rapid_movement > 10 {
            patterns.push(SuspiciousPattern {
                pattern_id: "RAPID_01".to_string(),
                pattern_name: "Rapid Movement".to_string(),
                confidence: 0.85,
                description: "Multiple transactions in short time".to_string(),
                transactions_involved: vec![],
            });
            risk_score += 40.0;
        }

        // ===== Structuring Pattern =====
        let amount_f64 = amount.to_string().parse::<f64>().unwrap_or(0.0);
        let similar_amounts = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM transactions
             WHERE sender_bank_id = $1
             AND created_at > NOW() - INTERVAL '24 hours'
             AND ABS(sent_amount - $2) < 100",
        )
        .bind(bank_id)
        .bind(amount_f64)
        .fetch_one(db)
        .await
        .unwrap_or(0);

        if similar_amounts > 3 {
            patterns.push(SuspiciousPattern {
                pattern_id: "STRUCT_01".to_string(),
                pattern_name: "Potential Structuring".to_string(),
                confidence: 0.75,
                description: "Multiple similar amounts".to_string(),
                transactions_involved: vec![],
            });
            risk_score += 50.0;
        }

        // ===== Round-trip Pattern =====
        let round_trip = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM transactions t1
             WHERE EXISTS (
                 SELECT 1 FROM transactions t2
                 WHERE t2.sender_bank_id = t1.receiver_bank_id
                 AND t2.receiver_bank_id = t1.sender_bank_id
                 AND t2.created_at > t1.created_at
                 AND t2.created_at < t1.created_at + INTERVAL '48 hours'
             )
             AND t1.sender_bank_id = $1
             AND t1.created_at > NOW() - INTERVAL '7 days'"
        )
        .bind(bank_id)
        .fetch_one(db)
        .await
        .unwrap_or(0);

        if round_trip > 0 {
            patterns.push(SuspiciousPattern {
                pattern_id: "ROUND_01".to_string(),
                pattern_name: "Round-trip Transaction".to_string(),
                confidence: 0.90,
                description: "Funds returned to origin".to_string(),
                transactions_involved: vec![],
            });
            risk_score += 60.0;
        }

        Ok(PatternDetectionResult {
            suspicious_patterns: patterns,
            risk_score: (risk_score as f64).min(100.0),
        })
    }

    fn assess_purpose_risk(&self, purpose: &Option<String>) -> f64 {
        if let Some(purpose_text) = purpose {
            let suspicious_keywords = [
                "cash", "drug", "weapon", "terror", "launder",
                "hawala", "bribe", "kickback", "shell", "offshore",
                "bearer", "anonymous",
            ];

            let purpose_lower = purpose_text.to_lowercase();
            for keyword in &suspicious_keywords {
                if purpose_lower.contains(keyword) {
                    return 90.0; // Very high risk
                }
            }

            // Purpose provided, normal content
            10.0
        } else {
            // No purpose provided
            30.0
        }
    }
}

impl Default for AmlScorer {
    fn default() -> Self {
        Self::new()
    }
}
