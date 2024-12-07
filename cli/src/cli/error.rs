use envy::Error as EnvyError;
use intmax2_client_sdk::{
    client::error::ClientError, external_api::contract::interface::BlockchainError,
};
use intmax2_interfaces::api::error::ServerError;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Env error:{0}")]
    EnvError(#[from] EnvyError),

    #[error("Client error: {0}")]
    ClientError(#[from] ClientError),

    #[error("Blockchain error: {0}")]
    BlockchainError(#[from] BlockchainError),

    #[error("InsufficientBalance: {0}")]
    InsufficientBalance(String),

    #[error("Server error: {0}")]
    ServerError(#[from] ServerError),

    #[error("Failed to request tx")]
    FailedToRequestTx,

    #[error("Failed to get proposal")]
    FailedToGetProposal,

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}
