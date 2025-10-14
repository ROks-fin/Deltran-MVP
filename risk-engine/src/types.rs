//! Core types for risk engine

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Risk score (0-100)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RiskScore(u8);

impl RiskScore {
    /// Create new risk score (0-100)
    pub fn new(score: u8) -> Self {
        Self(score.min(100))
    }

    /// Get raw score
    pub fn score(&self) -> u8 {
        self.0
    }

    /// Check if high risk (>= 75)
    pub fn is_high_risk(&self) -> bool {
        self.0 >= 75
    }

    /// Check if medium risk (50-74)
    pub fn is_medium_risk(&self) -> bool {
        (50..75).contains(&self.0)
    }

    /// Check if low risk (< 50)
    pub fn is_low_risk(&self) -> bool {
        self.0 < 50
    }
}

/// Risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
}

impl From<RiskScore> for RiskLevel {
    fn from(score: RiskScore) -> Self {
        if score.is_high_risk() {
            RiskLevel::High
        } else if score.is_medium_risk() {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }
}

/// Risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Payment ID
    pub payment_id: Uuid,

    /// Risk score
    pub risk_score: RiskScore,

    /// Risk level
    pub risk_level: RiskLevel,

    /// Risk factors detected
    pub risk_factors: Vec<String>,

    /// Approved
    pub approved: bool,

    /// Assessment timestamp
    pub assessed_at: chrono::DateTime<chrono::Utc>,
}
