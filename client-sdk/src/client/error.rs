use intmax2_interfaces::api::error::ServerError;

use crate::external_api::contract::error::BlockchainError;

use super::strategy::error::StrategyError;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Server client error: {0}")]
    ServerError(#[from] ServerError),

    #[error("Blockchain error: {0}")]
    BlockchainError(#[from] BlockchainError),

    #[error("Send tx request error: {0}")]
    SendTxRequestError(String),

    #[error("Witness generation error: {0}")]
    WitnessGenerationError(String),

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

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Server client error: {0}")]
    ServerError(#[from] ServerError),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Block number is not set for meta data")]
    BlockNumberIsNotSetForMetaData,

    #[error("Pending receives error: {0}")]
    PendingReceivesError(String),

    #[error("Pending tx error: {0}")]
    PendingTxError(String),

    #[error("Pending withdrawal error: {0}")]
    PendingWithdrawalError(String),

    #[error("Block number mismatch balance_proof_block_number: {balance_proof_block_number} != block_number: {block_number}")]
    BlockNumberMismatch {
        balance_proof_block_number: u64,
        block_number: u64,
    },

    #[error("Balance proof not found")]
    BalanceProofNotFound,
}
