use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde_json::json;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, TokenEngineError>;

#[derive(Error, Debug)]
pub enum TokenEngineError {
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

    #[error("Token not found: {0}")]
    TokenNotFound(uuid::Uuid),

    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: String, available: String },

    #[error("Invalid currency pair: {from} -> {to}")]
    InvalidCurrencyPair { from: String, to: String },

    #[error("Invalid currency: {0}")]
    InvalidCurrency(String),

    #[error("Token already exists: {0}")]
    TokenAlreadyExists(String),

    #[error("Invalid token status: {0}")]
    InvalidTokenStatus(String),

    #[error("Bank not found: {0}")]
    BankNotFound(uuid::Uuid),

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Internal server error: {0}")]
    Internal(String),
}

// Implement From for async_nats errors
impl<T> From<async_nats::error::Error<T>> for TokenEngineError
where
    T: std::fmt::Debug + std::fmt::Display + Clone + PartialEq,
{
    fn from(err: async_nats::error::Error<T>) -> Self {
        TokenEngineError::Nats(format!("NATS error: {:?}", err))
    }
}

impl From<async_nats::SubscribeError> for TokenEngineError {
    fn from(err: async_nats::SubscribeError) -> Self {
        TokenEngineError::Nats(format!("NATS subscribe error: {}", err))
    }
}

impl From<async_nats::PublishError> for TokenEngineError {
    fn from(err: async_nats::PublishError) -> Self {
        TokenEngineError::Nats(format!("NATS publish error: {}", err))
    }
}

impl From<serde_json::Error> for TokenEngineError {
    fn from(err: serde_json::Error) -> Self {
        TokenEngineError::Internal(format!("JSON serialization error: {}", err))
    }
}

impl ResponseError for TokenEngineError {
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
            TokenEngineError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TokenEngineError::Redis(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TokenEngineError::Nats(_) => StatusCode::INTERNAL_SERVER_ERROR,
            TokenEngineError::DecimalParse(_) => StatusCode::BAD_REQUEST,
            TokenEngineError::Validation(_) => StatusCode::BAD_REQUEST,
            TokenEngineError::TokenNotFound(_) => StatusCode::NOT_FOUND,
            TokenEngineError::InsufficientBalance { .. } => StatusCode::BAD_REQUEST,
            TokenEngineError::InvalidCurrencyPair { .. } => StatusCode::BAD_REQUEST,
            TokenEngineError::InvalidCurrency(_) => StatusCode::BAD_REQUEST,
            TokenEngineError::TokenAlreadyExists(_) => StatusCode::CONFLICT,
            TokenEngineError::InvalidTokenStatus(_) => StatusCode::BAD_REQUEST,
            TokenEngineError::BankNotFound(_) => StatusCode::NOT_FOUND,
            TokenEngineError::Unauthorized => StatusCode::UNAUTHORIZED,
            TokenEngineError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            TokenEngineError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            TokenEngineError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl TokenEngineError {
    fn error_type(&self) -> &str {
        match self {
            TokenEngineError::Database(_) => "database_error",
            TokenEngineError::Redis(_) => "cache_error",
            TokenEngineError::Nats(_) => "messaging_error",
            TokenEngineError::DecimalParse(_) => "decimal_parse_error",
            TokenEngineError::Validation(_) => "validation_error",
            TokenEngineError::TokenNotFound(_) => "not_found",
            TokenEngineError::InsufficientBalance { .. } => "insufficient_balance",
            TokenEngineError::InvalidCurrencyPair { .. } => "invalid_currency",
            TokenEngineError::InvalidCurrency(_) => "invalid_currency",
            TokenEngineError::TokenAlreadyExists(_) => "duplicate_error",
            TokenEngineError::InvalidTokenStatus(_) => "invalid_status",
            TokenEngineError::BankNotFound(_) => "not_found",
            TokenEngineError::Unauthorized => "unauthorized",
            TokenEngineError::RateLimitExceeded => "rate_limit",
            TokenEngineError::ServiceUnavailable => "service_unavailable",
            TokenEngineError::Internal(_) => "internal_error",
        }
    }
}