use crate::error::{ComplianceError, Result};
use crate::sanctions::SanctionsEngine;
use crate::types::{MatchDetail, ScreeningRequest, ScreeningResult, ScreeningStatus};
use chrono::Utc;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

/// ComplianceScreener performs comprehensive AML/sanctions screening
pub struct ComplianceScreener {
    sanctions_engine: Arc<SanctionsEngine>,
    high_risk_countries: Vec<String>,
    auto_block_threshold: u8,
    manual_review_threshold: u8,
}

impl ComplianceScreener {
    pub fn new(
        sanctions_engine: Arc<SanctionsEngine>,
        high_risk_countries: Vec<String>,
        auto_block_threshold: u8,
        manual_review_threshold: u8,
    ) -> Self {
        Self {
            sanctions_engine,
            high_risk_countries,
            auto_block_threshold,
            manual_review_threshold,
        }
    }

    /// Screen a party (sender or recipient) against sanctions lists
    pub async fn screen(&self, request: &ScreeningRequest) -> Result<ScreeningResult> {
        let screening_id = Uuid::new_v4();
        let mut match_details = Vec::new();
        let mut matched_lists = Vec::new();
        let mut risk_score = 0u8;

        // 1. Check name against sanctions lists
        match self.sanctions_engine.check_name(&request.name) {
            Ok(matches) if !matches.is_empty() => {
                risk_score += 80; // High risk for name match
                for entry in matches {
                    matched_lists.push(entry.list.as_str().to_string());
                    match_details.push(MatchDetail {
                        list_name: entry.list.as_str().to_string(),
                        entry_id: entry.id.clone(),
                        matched_field: "name".to_string(),
                        confidence: 0.95,
                    });
                }
                warn!("Sanctions name match for: {} (screening: {})", request.name, screening_id);
            }
            Ok(_) => {}
            Err(e) => {
                warn!("Error checking name sanctions: {}", e);
            }
        }

        // 2. Check aliases
        for alias in &request.aliases {
            match self.sanctions_engine.check_name(alias) {
                Ok(matches) if !matches.is_empty() => {
                    risk_score += 70;
                    for entry in matches {
                        matched_lists.push(entry.list.as_str().to_string());
                        match_details.push(MatchDetail {
                            list_name: entry.list.as_str().to_string(),
                            entry_id: entry.id.clone(),
                            matched_field: "alias".to_string(),
                            confidence: 0.90,
                        });
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("Error checking alias sanctions: {}", e);
                }
            }
        }

        // 3. Check country
        if let Some(country) = &request.country {
            match self.sanctions_engine.check_country(country) {
                Ok(matches) if !matches.is_empty() => {
                    risk_score += 60;
                    for entry in matches {
                        matched_lists.push(entry.list.as_str().to_string());
                        match_details.push(MatchDetail {
                            list_name: entry.list.as_str().to_string(),
                            entry_id: entry.id.clone(),
                            matched_field: "country".to_string(),
                            confidence: 1.0,
                        });
                    }
                }
                Ok(_) => {
                    // Check high-risk countries
                    if self.high_risk_countries.contains(country) {
                        risk_score += 20;
                        match_details.push(MatchDetail {
                            list_name: "HIGH_RISK_COUNTRIES".to_string(),
                            entry_id: country.clone(),
                            matched_field: "country".to_string(),
                            confidence: 1.0,
                        });
                    }
                }
                Err(e) => {
                    warn!("Error checking country sanctions: {}", e);
                }
            }
        }

        // 4. Check identifier (SWIFT, IBAN, etc)
        if let Some(identifier) = &request.identifier {
            match self.sanctions_engine.check_identifier(&identifier.value) {
                Ok(matches) if !matches.is_empty() => {
                    risk_score += 90; // Very high risk for identifier match
                    for entry in matches {
                        matched_lists.push(entry.list.as_str().to_string());
                        match_details.push(MatchDetail {
                            list_name: entry.list.as_str().to_string(),
                            entry_id: entry.id.clone(),
                            matched_field: format!("{:?}", identifier.id_type),
                            confidence: 1.0,
                        });
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("Error checking identifier sanctions: {}", e);
                }
            }
        }

        // Cap risk score at 100
        risk_score = risk_score.min(100);

        // Determine status based on risk score
        let status = if risk_score >= self.auto_block_threshold {
            ScreeningStatus::Blocked
        } else if risk_score >= self.manual_review_threshold {
            ScreeningStatus::ManualReview
        } else if !match_details.is_empty() {
            ScreeningStatus::Flagged
        } else {
            ScreeningStatus::Clear
        };

        // Deduplicate matched lists
        matched_lists.sort();
        matched_lists.dedup();

        let result = ScreeningResult {
            screening_id,
            status: status.clone(),
            matched_lists,
            match_details,
            risk_score,
            timestamp: Utc::now(),
        };

        if status == ScreeningStatus::Blocked {
            info!(
                "Screening BLOCKED for {} (score: {}, id: {})",
                request.name, risk_score, screening_id
            );
        } else if status == ScreeningStatus::ManualReview {
            info!(
                "Screening MANUAL_REVIEW for {} (score: {}, id: {})",
                request.name, risk_score, screening_id
            );
        }

        Ok(result)
    }

    /// Batch screening for multiple parties
    pub async fn screen_batch(&self, requests: Vec<ScreeningRequest>) -> Vec<Result<ScreeningResult>> {
        let mut results = Vec::new();
        for request in requests {
            results.push(self.screen(&request).await);
        }
        results
    }

    /// Check if screening result allows transaction to proceed
    pub fn is_approved(result: &ScreeningResult) -> bool {
        matches!(result.status, ScreeningStatus::Clear | ScreeningStatus::Flagged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EntityType, SanctionsEntry, SanctionsList};

    fn create_test_engine() -> Arc<SanctionsEngine> {
        let engine = Arc::new(SanctionsEngine::new(0.85));

        let test_entry = SanctionsEntry {
            id: "OFAC-001".to_string(),
            list: SanctionsList::OFAC,
            entity_type: EntityType::Individual,
            names: vec!["John Doe".to_string()],
            aliases: vec!["J. Doe".to_string()],
            addresses: vec![],
            countries: vec!["IR".to_string()],
            identifiers: vec![],
            programs: vec!["IRAN".to_string()],
            updated_at: Utc::now(),
        };

        engine.load_list(SanctionsList::OFAC, vec![test_entry]).unwrap();
        engine
    }

    #[tokio::test]
    async fn test_screening_blocked() {
        let engine = create_test_engine();
        let screener = ComplianceScreener::new(engine, vec![], 80, 50);

        let request = ScreeningRequest {
            name: "John Doe".to_string(),
            aliases: vec![],
            country: None,
            identifier: None,
            address: None,
        };

        let result = screener.screen(&request).await.unwrap();
        assert_eq!(result.status, ScreeningStatus::Blocked);
        assert!(result.risk_score >= 80);
    }

    #[tokio::test]
    async fn test_screening_clear() {
        let engine = create_test_engine();
        let screener = ComplianceScreener::new(engine, vec![], 80, 50);

        let request = ScreeningRequest {
            name: "Jane Smith".to_string(),
            aliases: vec![],
            country: Some("US".to_string()),
            identifier: None,
            address: None,
        };

        let result = screener.screen(&request).await.unwrap();
        assert_eq!(result.status, ScreeningStatus::Clear);
        assert_eq!(result.risk_score, 0);
    }
}
