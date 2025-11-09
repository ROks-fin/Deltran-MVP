use actix_web::{HttpResponse, ResponseError};
use std::fmt;

#[derive(Debug)]
pub enum RiskError {
    DatabaseError(sqlx::Error),
    RedisError(redis::RedisError),
    ConfigurationError(String),
    ValidationError(String),
    CircuitBreakerOpen,
    NotFound(String),
    InternalError(String),
}

impl fmt::Display for RiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RiskError::DatabaseError(e) => write!(f, "Database error: {}", e),
            RiskError::RedisError(e) => write!(f, "Redis error: {}", e),
            RiskError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            RiskError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            RiskError::CircuitBreakerOpen => write!(f, "Circuit breaker is open"),
            RiskError::NotFound(msg) => write!(f, "Not found: {}", msg),
            RiskError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for RiskError {}

impl ResponseError for RiskError {
    fn error_response(&self) -> HttpResponse {
        match self {
            RiskError::DatabaseError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "DATABASE_ERROR",
                    "message": self.to_string()
                }))
            }
            RiskError::RedisError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "CACHE_ERROR",
                    "message": self.to_string()
                }))
            }
            RiskError::ConfigurationError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "CONFIGURATION_ERROR",
                    "message": self.to_string()
                }))
            }
            RiskError::ValidationError(_) => {
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "VALIDATION_ERROR",
                    "message": self.to_string()
                }))
            }
            RiskError::CircuitBreakerOpen => {
                HttpResponse::ServiceUnavailable().json(serde_json::json!({
                    "error": "CIRCUIT_BREAKER_OPEN",
                    "message": "Service temporarily unavailable"
                }))
            }
            RiskError::NotFound(_) => {
                HttpResponse::NotFound().json(serde_json::json!({
                    "error": "NOT_FOUND",
                    "message": self.to_string()
                }))
            }
            RiskError::InternalError(_) => {
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "INTERNAL_ERROR",
                    "message": self.to_string()
                }))
            }
        }
    }
}

impl From<sqlx::Error> for RiskError {
    fn from(err: sqlx::Error) -> Self {
        RiskError::DatabaseError(err)
    }
}

impl From<redis::RedisError> for RiskError {
    fn from(err: redis::RedisError) -> Self {
        RiskError::RedisError(err)
    }
}

pub type RiskResult<T> = Result<T, RiskError>;
