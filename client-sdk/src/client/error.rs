use intmax2_interfaces::api::error::ServerError;

use crate::external_api::contract::interface::BlockchainError;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Server error: {0}")]
    ServerError(#[from] ServerError),

    #[error("Blockchain error: {0}")]
    BlockchainError(#[from] BlockchainError),

    #[error("Witness generation error: {0}")]
    WitnessGenerationError(String),

    #[error("Sync error: {0}")]
    SyncError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Balance error: {0}")]
    BalanceError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Invalid block proposal: {0}")]
    InvalidBlockProposal(String),

    #[error("Pending error: {0}")]
    PendingError(String),
}
