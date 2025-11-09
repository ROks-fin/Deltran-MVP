use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===== Compliance Check Result =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplianceCheckResult {
    pub transaction_id: Uuid,
    pub overall_status: ComplianceStatus,
    pub sanctions_check: SanctionsResult,
    pub aml_check: AmlResult,
    pub pep_check: PepResult,
    pub pattern_analysis: PatternResult,
    pub risk_rating: RiskRating,
    pub required_actions: Vec<RequiredAction>,
    pub checked_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ComplianceStatus {
    Approved,
    ReviewRequired,
    Rejected,
    Hold,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum RiskRating {
    Low,
    Medium,
    High,
    VeryHigh,
    Prohibited,
}

// ===== Sanctions Check =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SanctionsResult {
    pub is_sanctioned: bool,
    pub lists_matched: Vec<SanctionsList>,
    pub match_details: Vec<SanctionMatch>,
    pub confidence: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SanctionMatch {
    pub list_name: String,
    pub entity_name: String,
    pub match_score: f64,      // 0-100 fuzzy match score
    pub match_type: MatchType,
    pub aliases: Vec<String>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MatchType {
    Exact,
    Fuzzy,
    Partial,
    AliasMatch,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum SanctionsList {
    OFAC,       // US Office of Foreign Assets Control
    EU,         // European Union
    UN,         // United Nations
    UKHMT,      // UK HM Treasury
    DFAT,       // Australia DFAT
    Custom(String),
}

// ===== AML Check =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AmlResult {
    pub risk_score: f64,        // 0-100
    pub risk_factors: Vec<AmlRiskFactor>,
    pub suspicious_patterns: Vec<SuspiciousPattern>,
    pub requires_sar: bool,     // Suspicious Activity Report needed
    pub requires_ctr: bool,     // Currency Transaction Report needed
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AmlRiskFactor {
    pub factor_type: String,
    pub weight: f64,
    pub score: f64,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SuspiciousPattern {
    pub pattern_id: String,
    pub pattern_name: String,
    pub confidence: f64,
    pub description: String,
    pub transactions_involved: Vec<Uuid>,
}

// ===== PEP Check =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PepResult {
    pub is_pep: bool,
    pub pep_type: Option<PepType>,
    pub position: Option<String>,
    pub country: Option<String>,
    pub risk_level: Option<PepRiskLevel>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PepType {
    HeadOfState,
    Government,
    Military,
    Judicial,
    StateOwnedEnterprise,
    International,
    FamilyMember,
    CloseAssociate,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PepRiskLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

// ===== Pattern Analysis =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatternResult {
    pub normal_behavior: bool,
    pub anomaly_score: f64,
    pub detected_patterns: Vec<String>,
    pub ml_confidence: f64,
}

// ===== Required Actions =====
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RequiredAction {
    ManualReview,
    EnhancedDueDiligence,
    SeniorApproval,
    FileSAR,           // Suspicious Activity Report
    FileCTR,           // Currency Transaction Report
    BlockTransaction,
    FreezeAccount,
    NotifyAuthorities,
}

// ===== Compliance Request =====
#[derive(Debug, Deserialize, Clone)]
pub struct ComplianceCheckRequest {
    pub transaction_id: Uuid,
    pub sender_name: String,
    pub sender_account: String,
    pub sender_country: String,
    pub sender_bank_id: Uuid,
    pub receiver_name: String,
    pub receiver_account: String,
    pub receiver_country: String,
    pub receiver_bank_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub purpose: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ===== Internal Structures =====

// For pattern detection
#[derive(Debug)]
pub struct PatternDetectionResult {
    pub suspicious_patterns: Vec<SuspiciousPattern>,
    pub risk_score: f64,
}

// For sanctions entity storage
#[derive(Debug, Clone)]
pub struct SanctionedEntity {
    pub original_name: String,
    pub normalized_name: String,
    pub aliases: Vec<String>,
    pub countries: Vec<String>,
    pub reasons: Vec<String>,
    pub list_type: SanctionsList,
}

// ===== API Responses =====
#[derive(Debug, Serialize)]
pub struct ComplianceResponse {
    pub transaction_id: Uuid,
    pub overall_status: ComplianceStatus,
    pub sanctions_check: SimpleSanctionsInfo,
    pub aml_check: SimpleAmlInfo,
    pub pep_check: SimplePepInfo,
    pub risk_rating: RiskRating,
    pub required_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SimpleSanctionsInfo {
    pub is_sanctioned: bool,
    pub confidence: f64,
}

#[derive(Debug, Serialize)]
pub struct SimpleAmlInfo {
    pub risk_score: f64,
    pub requires_sar: bool,
    pub requires_ctr: bool,
}

#[derive(Debug, Serialize)]
pub struct SimplePepInfo {
    pub is_pep: bool,
}

impl From<ComplianceCheckResult> for ComplianceResponse {
    fn from(result: ComplianceCheckResult) -> Self {
        ComplianceResponse {
            transaction_id: result.transaction_id,
            overall_status: result.overall_status,
            sanctions_check: SimpleSanctionsInfo {
                is_sanctioned: result.sanctions_check.is_sanctioned,
                confidence: result.sanctions_check.confidence,
            },
            aml_check: SimpleAmlInfo {
                risk_score: result.aml_check.risk_score,
                requires_sar: result.aml_check.requires_sar,
                requires_ctr: result.aml_check.requires_ctr,
            },
            pep_check: SimplePepInfo {
                is_pep: result.pep_check.is_pep,
            },
            risk_rating: result.risk_rating,
            required_actions: result.required_actions.iter().map(|a| format!("{:?}", a)).collect(),
        }
    }
}

// ===== Health Check =====
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

// ===== Error Response =====
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}
