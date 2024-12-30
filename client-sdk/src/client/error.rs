use intmax2_interfaces::api::error::ServerError;

use crate::external_api::contract::error::BlockchainError;

use super::{strategy::error::StrategyError, sync::error::SyncError};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Server client error: {0}")]
    ServerError(#[from] ServerError),

    #[error("Blockchain error: {0}")]
    BlockchainError(#[from] BlockchainError),

    #[error("Send tx request error: {0}")]
    SendTxRequestError(String),

    #[error("Sync error: {0}")]
    SyncError(#[from] SyncError),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Balance error: {0}")]
    BalanceError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Invalid block proposal: {0}")]
    InvalidBlockProposal(String),

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),

    #[error("Strategy error: {0}")]
    StrategyError(#[from] StrategyError),
}
