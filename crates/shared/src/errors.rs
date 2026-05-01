use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Insufficient funds: available {available}, required {required}")]
    InsufficientFunds { available: String, required: String },
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    #[error("Customer not found: {0}")]
    CustomerNotFound(String),
    #[error("Loan not found: {0}")]
    LoanNotFound(String),
    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Duplicate transaction: idempotency key {0} already processed")]
    DuplicateTransaction(String),
    #[error("Currency mismatch: expected {expected}, got {got}")]
    CurrencyMismatch { expected: String, got: String },
    #[error("Negative amount not allowed")]
    NegativeAmount,
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Concurrency conflict: {0}")]
    ConcurrencyConflict(String),
}

#[derive(Debug, Error)]
pub enum InfrastructureError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Messaging error: {0}")]
    Messaging(String),
}
