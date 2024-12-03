#[derive(Debug, thiserror::Error)]
pub enum BlockchainError {
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Decode call data error: {0}")]
    DecodeCallDataError(String),

    #[error("Token not found")]
    TokenNotFound,

    #[error("Internal error: {0}")]
    InternalError(String),
}
