use thiserror::Error;

#[derive(Error, Debug)]
pub enum SettlementError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("NATS error: {0}")]
    Nats(String),

    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds {
        required: rust_decimal::Decimal,
        available: rust_decimal::Decimal,
    },

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Account inactive: {0}")]
    InactiveAccount(String),

    #[error("Settlement window closed for currency: {0}")]
    SettlementWindowClosed(String),

    #[error("Compliance check blocked")]
    ComplianceBlocked,

    #[error("Invalid state transition: {0}")]
    InvalidState(String),

    #[error("Bank transfer failed: {0}")]
    BankTransferFailed(String),

    #[error("Transfer timeout after {0} seconds")]
    TransferTimeout(u64),

    #[error("Rollback failed: {0}")]
    RollbackFailed(String),

    #[error("Fund lock not found: {0}")]
    LockNotFound(String),

    #[error("Fund lock expired: {0}")]
    LockExpired(String),

    #[error("Atomic operation not found: {0}")]
    AtomicOperationNotFound(String),

    #[error("Reconciliation error: {0}")]
    ReconciliationError(String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] anyhow::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Decimal parse error: {0}")]
    DecimalParse(#[from] rust_decimal::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SettlementError>;
