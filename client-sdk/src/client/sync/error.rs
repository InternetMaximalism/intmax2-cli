use crate::client::strategy::error::StrategyError;
use intmax2_interfaces::api::error::ServerError;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Server client error: {0}")]
    ServerError(#[from] ServerError),

    #[error("Strategy error: {0}")]
    StrategyError(#[from] StrategyError),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Block number is not set for meta data")]
    BlockNumberIsNotSetForMetaData,

    #[error("Pending receives error: {0}")]
    PendingReceivesError(String),

    #[error("Pending tx error: {0}")]
    PendingTxError(String),

    #[error("Witness generation error: {0}")]
    WitnessGenerationError(String),

    #[error("Failed to update private state: {0}")]
    FailedToUpdatePrivateState(String),

    #[error("Validity prover is not up to date validity_prover_block_number: {validity_prover_block_number} < block_number: {block_number}")]
    ValidityProverIsNotUpToDate {
        validity_prover_block_number: u32,
        block_number: u32,
    },

    #[error("Invalid transfer error: {0}")]
    InvalidTransferError(String),

    #[error("Inconsistency error: {0}")]
    InconsistencyError(String),

    #[error("Block number mismatch balance_proof_block_number: {balance_proof_block_number} != block_number: {block_number}")]
    BalanceProofBlockNumberMismatch {
        balance_proof_block_number: u32,
        block_number: u32,
    },
}
