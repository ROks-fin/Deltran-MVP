use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ComplianceError {
    #[error("Sanctions list not loaded: {0}")]
    ListNotLoaded(String),

    #[error("Sanctions match found: {details}")]
    SanctionsMatch { details: String },

    #[error("AML screening failed: {0}")]
    AmlFailed(String),

    #[error("High risk jurisdiction: {0}")]
    HighRiskJurisdiction(String),

    #[error("Invalid screening input: {0}")]
    InvalidInput(String),

    #[error("Sanctions list update failed: {0}")]
    UpdateFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, ComplianceError>;
