//! Risk scoring engine

use crate::{Result, RiskScore, RiskLevel, RiskAssessment};
use rust_decimal::Decimal;
use uuid::Uuid;

/// Risk scorer
pub struct RiskScorer {
    // Configuration would go here
}

impl RiskScorer {
    /// Create new risk scorer
    pub fn new() -> Self {
        Self {}
    }

    /// Assess payment risk
    pub fn assess_payment(
        &self,
        payment_id: Uuid,
        amount: Decimal,
        _sender_country: &str,
        _receiver_country: &str,
    ) -> Result<RiskAssessment> {
        let mut risk_factors = Vec::new();
        let mut score = 0u8;

        // Simple risk scoring logic
        if amount > Decimal::from(100_000) {
            score += 20;
            risk_factors.push("High value transaction".to_string());
        }

        if amount > Decimal::from(500_000) {
            score += 30;
            risk_factors.push("Very high value transaction".to_string());
        }

        let risk_score = RiskScore::new(score);
        let risk_level = RiskLevel::from(risk_score);

        Ok(RiskAssessment {
            payment_id,
            risk_score,
            risk_level,
            risk_factors,
            approved: !risk_score.is_high_risk(),
            assessed_at: chrono::Utc::now(),
        })
    }
}

impl Default for RiskScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_scoring() {
        let scorer = RiskScorer::new();
        let assessment = scorer.assess_payment(
            Uuid::new_v4(),
            Decimal::from(50_000),
            "US",
            "GB",
        ).unwrap();

        assert!(assessment.approved);
        assert!(assessment.risk_score.is_low_risk());
    }

    #[test]
    fn test_high_value_risk() {
        let scorer = RiskScorer::new();
        let assessment = scorer.assess_payment(
            Uuid::new_v4(),
            Decimal::from(600_000),
            "US",
            "GB",
        ).unwrap();

        // Should have risk factors
        assert!(!assessment.risk_factors.is_empty());
    }
}
