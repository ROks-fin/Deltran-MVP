use crate::errors::ComplianceResult;
use crate::models::*;
use std::collections::HashMap;
use strsim::jaro_winkler;

pub struct SanctionsMatcher {
    sanctions_data: HashMap<SanctionsList, Vec<SanctionedEntity>>,
}

impl SanctionsMatcher {
    pub fn new() -> Self {
        let mut sanctions_data = HashMap::new();

        // Load OFAC sanctioned entities
        sanctions_data.insert(SanctionsList::OFAC, Self::load_ofac_sanctions());

        // Load UN sanctioned entities
        sanctions_data.insert(SanctionsList::UN, Self::load_un_sanctions());

        // Load EU sanctioned entities
        sanctions_data.insert(SanctionsList::EU, Self::load_eu_sanctions());

        SanctionsMatcher { sanctions_data }
    }

    pub fn check_sanctions(
        &self,
        name: &str,
        country: &str,
    ) -> ComplianceResult<SanctionsResult> {
        let mut matches = Vec::new();
        let mut lists_matched = Vec::new();

        // Normalize name for matching
        let normalized_name = normalize_name(name);

        // Check each sanctions list
        for (list, entities) in &self.sanctions_data {
            for entity in entities {
                // Check main name
                let match_score = self.calculate_match_score(
                    &normalized_name,
                    &entity.normalized_name
                );

                if match_score > 85.0 {
                    // High confidence match
                    matches.push(SanctionMatch {
                        list_name: format!("{:?}", list),
                        entity_name: entity.original_name.clone(),
                        match_score,
                        match_type: if match_score > 95.0 {
                            MatchType::Exact
                        } else {
                            MatchType::Fuzzy
                        },
                        aliases: entity.aliases.clone(),
                        reasons: entity.reasons.clone(),
                    });

                    if !lists_matched.contains(list) {
                        lists_matched.push(list.clone());
                    }
                } else if match_score > 70.0 {
                    // Medium confidence - check aliases
                    for alias in &entity.aliases {
                        let alias_score = self.calculate_match_score(
                            &normalized_name,
                            &normalize_name(alias)
                        );

                        if alias_score > 80.0 {
                            matches.push(SanctionMatch {
                                list_name: format!("{:?}", list),
                                entity_name: entity.original_name.clone(),
                                match_score: alias_score,
                                match_type: MatchType::AliasMatch,
                                aliases: vec![alias.clone()],
                                reasons: entity.reasons.clone(),
                            });

                            if !lists_matched.contains(list) {
                                lists_matched.push(list.clone());
                            }
                            break;
                        }
                    }
                }
            }
        }

        // Check country-level sanctions
        if is_sanctioned_country(country) {
            matches.push(SanctionMatch {
                list_name: "Country Sanctions".to_string(),
                entity_name: country.to_string(),
                match_score: 100.0,
                match_type: MatchType::Exact,
                aliases: vec![],
                reasons: vec![format!("Country {} is under sanctions", country)],
            });
        }

        let is_sanctioned = !matches.is_empty();
        let confidence = if is_sanctioned {
            matches.iter().map(|m| m.match_score).max_by(|a, b|
                a.partial_cmp(b).unwrap()
            ).unwrap_or(0.0) / 100.0
        } else {
            1.0 // High confidence in no match
        };

        Ok(SanctionsResult {
            is_sanctioned,
            lists_matched,
            match_details: matches,
            confidence,
        })
    }

    fn calculate_match_score(&self, name1: &str, name2: &str) -> f64 {
        // Use Jaro-Winkler for fuzzy matching
        let base_score = jaro_winkler(name1, name2) * 100.0;

        // Apply additional heuristics
        let word_match_bonus = calculate_word_overlap(name1, name2) * 20.0;
        let length_penalty = (name1.len() as f64 - name2.len() as f64).abs() * 0.5;

        (base_score + word_match_bonus - length_penalty).max(0.0).min(100.0)
    }

    // Load sanctioned entities from various lists
    fn load_ofac_sanctions() -> Vec<SanctionedEntity> {
        vec![
            SanctionedEntity {
                original_name: "Bank Melli Iran".to_string(),
                normalized_name: normalize_name("Bank Melli Iran"),
                aliases: vec!["Melli Bank".to_string(), "BMI".to_string()],
                countries: vec!["IR".to_string()],
                reasons: vec!["Facilitating Iranian government transactions".to_string()],
                list_type: SanctionsList::OFAC,
            },
            SanctionedEntity {
                original_name: "Banco Bandes".to_string(),
                normalized_name: normalize_name("Banco Bandes"),
                aliases: vec!["BANDES".to_string()],
                countries: vec!["VE".to_string()],
                reasons: vec!["Venezuelan state-owned bank under sanctions".to_string()],
                list_type: SanctionsList::OFAC,
            },
            SanctionedEntity {
                original_name: "Commercial Bank of Syria".to_string(),
                normalized_name: normalize_name("Commercial Bank of Syria"),
                aliases: vec!["CBS".to_string()],
                countries: vec!["SY".to_string()],
                reasons: vec!["Syrian government bank under sanctions".to_string()],
                list_type: SanctionsList::OFAC,
            },
        ]
    }

    fn load_un_sanctions() -> Vec<SanctionedEntity> {
        vec![
            SanctionedEntity {
                original_name: "Korea Kwangson Banking Corp".to_string(),
                normalized_name: normalize_name("Korea Kwangson Banking Corp"),
                aliases: vec!["KKBC".to_string()],
                countries: vec!["KP".to_string()],
                reasons: vec!["North Korean bank under UN sanctions".to_string()],
                list_type: SanctionsList::UN,
            },
        ]
    }

    fn load_eu_sanctions() -> Vec<SanctionedEntity> {
        vec![
            SanctionedEntity {
                original_name: "VTB Bank".to_string(),
                normalized_name: normalize_name("VTB Bank"),
                aliases: vec!["VTB".to_string(), "Vneshtorgbank".to_string()],
                countries: vec!["RU".to_string()],
                reasons: vec!["Russian state-owned bank under EU sanctions".to_string()],
                list_type: SanctionsList::EU,
            },
        ]
    }
}

impl Default for SanctionsMatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn calculate_word_overlap(name1: &str, name2: &str) -> f64 {
    let words1: Vec<&str> = name1.split_whitespace().collect();
    let words2: Vec<&str> = name2.split_whitespace().collect();

    let overlap_count = words1.iter()
        .filter(|w1| words2.contains(w1))
        .count();

    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    overlap_count as f64 / words1.len().max(words2.len()) as f64
}

fn is_sanctioned_country(country: &str) -> bool {
    const SANCTIONED_COUNTRIES: [&str; 10] = [
        "IR", // Iran
        "KP", // North Korea
        "SY", // Syria
        "CU", // Cuba
        "VE", // Venezuela
        "MM", // Myanmar
        "ZW", // Zimbabwe
        "SD", // Sudan
        "BY", // Belarus
        "AF", // Afghanistan (Taliban control)
    ];
    SANCTIONED_COUNTRIES.contains(&country)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let matcher = SanctionsMatcher::new();
        let result = matcher.check_sanctions("Bank Melli Iran", "IR").unwrap();
        assert!(result.is_sanctioned);
        assert!(result.confidence > 0.95);
    }

    #[test]
    fn test_fuzzy_match() {
        let matcher = SanctionsMatcher::new();
        let result = matcher.check_sanctions("Bank Meli Iran", "IR").unwrap(); // Typo: Meli instead of Melli
        assert!(result.is_sanctioned);
    }

    #[test]
    fn test_sanctioned_country() {
        let matcher = SanctionsMatcher::new();
        let result = matcher.check_sanctions("Any Entity", "KP").unwrap();
        assert!(result.is_sanctioned);
    }

    #[test]
    fn test_clean_entity() {
        let matcher = SanctionsMatcher::new();
        let result = matcher.check_sanctions("HDFC Bank", "IN").unwrap();
        assert!(!result.is_sanctioned);
    }
}
