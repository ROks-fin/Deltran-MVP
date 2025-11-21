use thiserror::Error;
use uuid::Uuid;
use rust_decimal::Decimal;

#[derive(Error, Debug)]
pub enum ClearingError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("NATS error: {0}")]
    Nats(String),

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid window state: expected {expected}, got {actual}")]
    InvalidWindowState {
        expected: String,
        actual: String,
    },

    #[error("Window not found: {0}")]
    WindowNotFound(i64),

    #[error("Window already open for region")]
    WindowAlreadyOpen,

    #[error("Window is locked by: {locked_by}")]
    WindowLocked {
        locked_by: String,
    },

    #[error("Insufficient balance for bank {bank_id}: required {required}, available {available}")]
    InsufficientBalance {
        bank_id: Uuid,
        required: Decimal,
        available: Decimal,
    },

    #[error("Limit exceeded for bank {bank_id}: amount {amount}, limit {limit}")]
    LimitExceeded {
        bank_id: Uuid,
        amount: Decimal,
        limit: Decimal,
    },

    #[error("Netting calculation failed: {0}")]
    NettingFailed(String),

    #[error("Settlement instruction generation failed: {0}")]
    SettlementInstructionFailed(String),

    #[error("Settlement execution failed for instruction {0}")]
    SettlementFailed(Uuid),

    #[error("Risk check failed for instruction {0}")]
    RiskCheckFailed(Uuid),

    #[error("Risk check error: {0}")]
    RiskCheckError(String),

    #[error("Atomic operation failed: {operation_type}, reason: {reason}")]
    AtomicOperationFailed {
        operation_type: String,
        reason: String,
    },

    #[error("Rollback failed for operation {operation_id}: {reason}")]
    RollbackFailed {
        operation_id: Uuid,
        reason: String,
    },

    #[error("Checkpoint not found: {checkpoint_name} for operation {operation_id}")]
    CheckpointNotFound {
        checkpoint_name: String,
        operation_id: Uuid,
    },

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Calculation overflow")]
    CalculationOverflow,

    #[error("Calculation underflow")]
    CalculationUnderflow,

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Node not found in graph")]
    NodeNotFound,

    #[error("Graph error: {0}")]
    GraphError(String),

    #[error("Scheduler error: {0}")]
    SchedulerError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Obligation not found: {0}")]
    ObligationNotFound(Uuid),

    #[error("Invalid currency: {0}")]
    InvalidCurrency(String),
}

pub type Result<T> = std::result::Result<T, ClearingError>;

impl From<async_nats::Error> for ClearingError {
    fn from(err: async_nats::Error) -> Self {
        ClearingError::Nats(err.to_string())
    }
}

impl From<async_nats::jetstream::context::PublishError> for ClearingError {
    fn from(err: async_nats::jetstream::context::PublishError) -> Self {
        ClearingError::Nats(err.to_string())
    }
}
