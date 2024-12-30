use intmax2_interfaces::api::error::ServerError;

use crate::external_api::contract::error::BlockchainError;

use super::{strategy::error::StrategyError, sync::error::SyncError};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Server client error: {0}")]
    ServerError(#[from] ServerError),

    #[error("Blockchain error: {0}")]
    BlockchainError(#[from] BlockchainError),

    #[error("Strategy error: {0}")]
    StrategyError(#[from] StrategyError),

    #[error("Sync error: {0}")]
    SyncError(#[from] SyncError),

    #[error("Send tx request error: {0}")]
    SendTxRequestError(String),

    #[error("Balance error: {0}")]
    BalanceError(String),

    #[error("Invalid transfer len: {0}")]
    TransferLenError(String),

    #[error("Invalid block proposal: {0}")]
    InvalidBlockProposal(String),

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}
