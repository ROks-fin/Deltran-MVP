use actix_web::{HttpResponse, ResponseError};
use std::fmt;

pub type ComplianceResult<T> = Result<T, ComplianceError>;

#[derive(Debug)]
pub enum ComplianceError {
    DatabaseError(sqlx::Error),
    ConfigurationError(String),
    ValidationError(String),
    SanctionsCheckFailed(String),
    AmlCheckFailed(String),
    PepCheckFailed(String),
    ExternalServiceError(String),
    NotFound(String),
    InternalError(String),
}

impl fmt::Display for ComplianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComplianceError::DatabaseError(e) => write!(f, "Database error: {}", e),
            ComplianceError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            ComplianceError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ComplianceError::SanctionsCheckFailed(msg) => write!(f, "Sanctions check failed: {}", msg),
            ComplianceError::AmlCheckFailed(msg) => write!(f, "AML check failed: {}", msg),
            ComplianceError::PepCheckFailed(msg) => write!(f, "PEP check failed: {}", msg),
            ComplianceError::ExternalServiceError(msg) => write!(f, "External service error: {}", msg),
            ComplianceError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ComplianceError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for ComplianceError {}

impl From<sqlx::Error> for ComplianceError {
    fn from(err: sqlx::Error) -> Self {
        ComplianceError::DatabaseError(err)
    }
}

impl ResponseError for ComplianceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ComplianceError::DatabaseError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "DatabaseError",
                    "message": self.to_string()
                }))
            }
            ComplianceError::ConfigurationError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "ConfigurationError",
                    "message": self.to_string()
                }))
            }
            ComplianceError::ValidationError(_) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "ValidationError",
                    "message": self.to_string()
                }))
            }
            ComplianceError::SanctionsCheckFailed(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "SanctionsCheckFailed",
                    "message": self.to_string()
                }))
            }
            ComplianceError::AmlCheckFailed(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "AmlCheckFailed",
                    "message": self.to_string()
                }))
            }
            ComplianceError::PepCheckFailed(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "PepCheckFailed",
                    "message": self.to_string()
                }))
            }
            ComplianceError::ExternalServiceError(_) => {
                HttpResponse::BadGateway().json(serde_json::json!({
                    "error": "ExternalServiceError",
                    "message": self.to_string()
                }))
            }
            ComplianceError::NotFound(_) => {
                HttpResponse::NotFound().json(serde_json::json!({
                    "error": "NotFound",
                    "message": self.to_string()
                }))
            }
            ComplianceError::InternalError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "InternalError",
                    "message": self.to_string()
                }))
            }
        }
    }
}
