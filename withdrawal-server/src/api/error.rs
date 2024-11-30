#[derive(Debug, thiserror::Error)]
pub enum WithdrawalServerError {
    #[error("Database error {0}")]
    DBError(#[from] sqlx::Error),

    #[error("Single withdrawal proof verification error")]
    SingleWithdrawalVerificationError,

    #[error("Serialization error {0}")]
    SerializationError(String),
}
