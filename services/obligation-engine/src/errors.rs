use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ObligationEngineError>;

#[derive(Error, Debug)]
pub enum ObligationEngineError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("NATS error: {0}")]
    Nats(String),

    #[error("Decimal parse error: {0}")]
    DecimalParse(#[from] rust_decimal::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Obligation not found: {0}")]
    ObligationNotFound(uuid::Uuid),

    #[error("Insufficient liquidity: required {required}, available {available}")]
    InsufficientLiquidity { required: String, available: String },

    #[error("Invalid corridor: {0}")]
    InvalidCorridor(String),

    #[error("Clearing window closed: {0}")]
    ClearingWindowClosed(i64),

    #[error("Netting failed: {0}")]
    NettingFailed(String),

    #[error("Settlement failed: {0}")]
    SettlementFailed(String),

    #[error("Token Engine error: {0}")]
    TokenEngineError(String),

    #[error("Bank not found: {0}")]
    BankNotFound(uuid::Uuid),

    #[error("Duplicate obligation: {0}")]
    DuplicateObligation(String),

    #[error("Risk limit exceeded: {0}")]
    RiskLimitExceeded(String),

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl ResponseError for ObligationEngineError {
    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_message = self.to_string();

        HttpResponse::build(status_code).json(json!({
            "error": {
                "code": status_code.as_u16(),
                "message": error_message,
                "type": self.error_type()
            }
        }))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ObligationEngineError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ObligationEngineError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ObligationEngineError::Nats(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ObligationEngineError::DecimalParse(_) => StatusCode::BAD_REQUEST,
            ObligationEngineError::Validation(_) => StatusCode::BAD_REQUEST,
            ObligationEngineError::ObligationNotFound(_) => StatusCode::NOT_FOUND,
            ObligationEngineError::InsufficientLiquidity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            ObligationEngineError::InvalidCorridor(_) => StatusCode::BAD_REQUEST,
            ObligationEngineError::ClearingWindowClosed(_) => StatusCode::CONFLICT,
            ObligationEngineError::NettingFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ObligationEngineError::SettlementFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ObligationEngineError::TokenEngineError(_) => StatusCode::BAD_GATEWAY,
            ObligationEngineError::BankNotFound(_) => StatusCode::NOT_FOUND,
            ObligationEngineError::DuplicateObligation(_) => StatusCode::CONFLICT,
            ObligationEngineError::RiskLimitExceeded(_) => StatusCode::FORBIDDEN,
            ObligationEngineError::Unauthorized => StatusCode::UNAUTHORIZED,
            ObligationEngineError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            ObligationEngineError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ObligationEngineError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl ObligationEngineError {
    fn error_type(&self) -> &str {
        match self {
            ObligationEngineError::Database(_) => "database_error",
            ObligationEngineError::Redis(_) => "cache_error",
            ObligationEngineError::Nats(_) => "messaging_error",
            ObligationEngineError::DecimalParse(_) => "decimal_parse_error",
            ObligationEngineError::Validation(_) => "validation_error",
            ObligationEngineError::ObligationNotFound(_) => "not_found",
            ObligationEngineError::InsufficientLiquidity { .. } => "insufficient_liquidity",
            ObligationEngineError::InvalidCorridor(_) => "invalid_corridor",
            ObligationEngineError::ClearingWindowClosed(_) => "window_closed",
            ObligationEngineError::NettingFailed(_) => "netting_failed",
            ObligationEngineError::SettlementFailed(_) => "settlement_failed",
            ObligationEngineError::TokenEngineError(_) => "external_service_error",
            ObligationEngineError::BankNotFound(_) => "not_found",
            ObligationEngineError::DuplicateObligation(_) => "duplicate_error",
            ObligationEngineError::RiskLimitExceeded(_) => "risk_limit_exceeded",
            ObligationEngineError::Unauthorized => "unauthorized",
            ObligationEngineError::RateLimitExceeded => "rate_limit",
            ObligationEngineError::ServiceUnavailable => "service_unavailable",
            ObligationEngineError::Internal(_) => "internal_error",
        }
    }
}