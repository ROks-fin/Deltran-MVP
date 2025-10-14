use crate::error::{ComplianceError, Result};
use crate::types::{SanctionsEntry, SanctionsList};
use dashmap::DashMap;
use regex::Regex;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// SanctionsEngine manages multiple sanctions lists and provides fast lookup
pub struct SanctionsEngine {
    // Map: list_name -> (entry_id -> SanctionsEntry)
    lists: Arc<DashMap<String, DashMap<String, SanctionsEntry>>>,
    fuzzy_threshold: f64,
}

impl SanctionsEngine {
    pub fn new(fuzzy_threshold: f64) -> Self {
        Self {
            lists: Arc::new(DashMap::new()),
            fuzzy_threshold,
        }
    }

    /// Load sanctions list into memory
    pub fn load_list(&self, list: SanctionsList, entries: Vec<SanctionsEntry>) -> Result<()> {
        let list_name = list.as_str().to_string();
        let map = DashMap::new();

        for entry in entries {
            map.insert(entry.id.clone(), entry);
        }

        self.lists.insert(list_name.clone(), map);
        info!("Loaded {} sanctions list with {} entries", list_name, self.lists.get(&list_name).unwrap().len());

        Ok(())
    }

    /// Check if name matches any sanctioned entity
    pub fn check_name(&self, name: &str) -> Result<Vec<SanctionsEntry>> {
        let normalized_name = Self::normalize_name(name);
        let mut matches = Vec::new();

        for list_entry in self.lists.iter() {
            let list_name = list_entry.key();
            let entries = list_entry.value();

            for entry_ref in entries.iter() {
                let entry = entry_ref.value();

                // Check primary names
                for sanctioned_name in &entry.names {
                    let normalized_sanctioned = Self::normalize_name(sanctioned_name);
                    if Self::fuzzy_match(&normalized_name, &normalized_sanctioned, self.fuzzy_threshold) {
                        debug!("Sanctions match found in {}: {} matches {}", list_name, name, sanctioned_name);
                        matches.push(entry.clone());
                        break;
                    }
                }

                // Check aliases
                for alias in &entry.aliases {
                    let normalized_alias = Self::normalize_name(alias);
                    if Self::fuzzy_match(&normalized_name, &normalized_alias, self.fuzzy_threshold) {
                        debug!("Sanctions alias match found in {}: {} matches {}", list_name, name, alias);
                        matches.push(entry.clone());
                        break;
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Check if country is sanctioned
    pub fn check_country(&self, country_code: &str) -> Result<Vec<SanctionsEntry>> {
        let mut matches = Vec::new();
        let country_upper = country_code.to_uppercase();

        for list_entry in self.lists.iter() {
            for entry_ref in list_entry.value().iter() {
                let entry = entry_ref.value();

                for country in &entry.countries {
                    if country.to_uppercase() == country_upper {
                        matches.push(entry.clone());
                        break;
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Check if identifier (SWIFT, IBAN, etc) is sanctioned
    pub fn check_identifier(&self, id_value: &str) -> Result<Vec<SanctionsEntry>> {
        let mut matches = Vec::new();
        let normalized_id = id_value.replace(char::is_whitespace, "").to_uppercase();

        for list_entry in self.lists.iter() {
            for entry_ref in list_entry.value().iter() {
                let entry = entry_ref.value();

                for identifier in &entry.identifiers {
                    let normalized_entry_id = identifier.value.replace(char::is_whitespace, "").to_uppercase();
                    if normalized_entry_id == normalized_id {
                        matches.push(entry.clone());
                        break;
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Get total number of entries across all lists
    pub fn total_entries(&self) -> usize {
        self.lists.iter().map(|l| l.value().len()).sum()
    }

    /// Get list of loaded sanctions lists
    pub fn loaded_lists(&self) -> Vec<String> {
        self.lists.iter().map(|l| l.key().clone()).collect()
    }

    // Normalize name for comparison (lowercase, remove special chars, collapse whitespace)
    fn normalize_name(name: &str) -> String {
        let re = Regex::new(r"[^\w\s]").unwrap();
        let cleaned = re.replace_all(name, "");
        let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
        collapsed.to_lowercase()
    }

    // Simple fuzzy matching using Levenshtein distance
    fn fuzzy_match(s1: &str, s2: &str, threshold: f64) -> bool {
        if s1 == s2 {
            return true;
        }

        let distance = Self::levenshtein_distance(s1, s2);
        let max_len = s1.len().max(s2.len()) as f64;
        let similarity = 1.0 - (distance as f64 / max_len);

        similarity >= threshold
    }

    // Levenshtein distance implementation
    fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for (i, c1) in s1.chars().enumerate() {
            for (j, c2) in s2.chars().enumerate() {
                let cost = if c1 == c2 { 0 } else { 1 };
                matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                    .min(matrix[i + 1][j] + 1)
                    .min(matrix[i][j] + cost);
            }
        }

        matrix[len1][len2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_name_normalization() {
        assert_eq!(
            SanctionsEngine::normalize_name("John O'Brien, Jr."),
            "john obrien jr"
        );
        assert_eq!(
            SanctionsEngine::normalize_name("ACME   Corp."),
            "acme corp"
        );
    }

    #[test]
    fn test_fuzzy_match() {
        assert!(SanctionsEngine::fuzzy_match("John Smith", "John Smith", 0.9));
        assert!(SanctionsEngine::fuzzy_match("John Smith", "Jon Smith", 0.85));
        assert!(!SanctionsEngine::fuzzy_match("John Smith", "Jane Doe", 0.9));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(SanctionsEngine::levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(SanctionsEngine::levenshtein_distance("saturday", "sunday"), 3);
        assert_eq!(SanctionsEngine::levenshtein_distance("test", "test"), 0);
    }
}
