use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScreeningStatus {
    Clear,
    Flagged,
    Blocked,
    ManualReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningResult {
    pub screening_id: Uuid,
    pub status: ScreeningStatus,
    pub matched_lists: Vec<String>,
    pub match_details: Vec<MatchDetail>,
    pub risk_score: u8, // 0-100
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchDetail {
    pub list_name: String,
    pub entry_id: String,
    pub matched_field: String,
    pub confidence: f64, // 0.0-1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SanctionsList {
    OFAC,        // US Office of Foreign Assets Control
    EU,          // European Union
    UN,          // United Nations
    UKHMТ,       // UK His Majesty's Treasury
    Local(String), // Country-specific lists
}

impl SanctionsList {
    pub fn as_str(&self) -> &str {
        match self {
            SanctionsList::OFAC => "OFAC",
            SanctionsList::EU => "EU",
            SanctionsList::UN => "UN",
            SanctionsList::UKHMТ => "UK_HMT",
            SanctionsList::Local(name) => name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsEntry {
    pub id: String,
    pub list: SanctionsList,
    pub entity_type: EntityType,
    pub names: Vec<String>,
    pub aliases: Vec<String>,
    pub addresses: Vec<String>,
    pub countries: Vec<String>,
    pub identifiers: Vec<Identifier>,
    pub programs: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    Individual,
    Organization,
    Vessel,
    Aircraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifier {
    pub id_type: IdentifierType,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IdentifierType {
    Passport,
    NationalId,
    TaxId,
    Swift,
    Iban,
    RegisterNumber,
    Imo, // International Maritime Organization
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningRequest {
    pub name: String,
    pub aliases: Vec<String>,
    pub country: Option<String>,
    pub identifier: Option<Identifier>,
    pub address: Option<String>,
}
