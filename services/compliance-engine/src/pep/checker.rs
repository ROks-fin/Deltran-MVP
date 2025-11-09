use crate::errors::ComplianceResult;
use crate::models::*;
use strsim::jaro_winkler;

pub struct PepChecker {}

impl PepChecker {
    pub fn new() -> Self {
        PepChecker {}
    }

    pub fn check_pep(&self, name: &str, _country: &str) -> ComplianceResult<PepResult> {
        // MVP: Simplified PEP check with hardcoded list
        let pep_list = self.get_pep_list();

        let normalized_name = name.to_lowercase();

        for pep in &pep_list {
            let score = jaro_winkler(&normalized_name, &pep.normalized_name);

            if score > 0.85 {
                return Ok(PepResult {
                    is_pep: true,
                    pep_type: Some(pep.pep_type.clone()),
                    position: Some(pep.position.clone()),
                    country: Some(pep.country.clone()),
                    risk_level: Some(pep.risk_level.clone()),
                });
            }
        }

        // No PEP match found
        Ok(PepResult {
            is_pep: false,
            pep_type: None,
            position: None,
            country: None,
            risk_level: None,
        })
    }

    fn get_pep_list(&self) -> Vec<PepEntry> {
        vec![
            // Sample PEPs for MVP
            PepEntry {
                name: "Vladimir Putin".to_string(),
                normalized_name: "vladimir putin".to_string(),
                pep_type: PepType::HeadOfState,
                position: "President of Russia".to_string(),
                country: "RU".to_string(),
                risk_level: PepRiskLevel::VeryHigh,
            },
            PepEntry {
                name: "Xi Jinping".to_string(),
                normalized_name: "xi jinping".to_string(),
                pep_type: PepType::HeadOfState,
                position: "President of China".to_string(),
                country: "CN".to_string(),
                risk_level: PepRiskLevel::High,
            },
        ]
    }
}

impl Default for PepChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct PepEntry {
    name: String,
    normalized_name: String,
    pep_type: PepType,
    position: String,
    country: String,
    risk_level: PepRiskLevel,
}
