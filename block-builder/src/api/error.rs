use intmax2_client_sdk::external_api::contract::interface::BlockchainError;
use intmax2_interfaces::api::error::ServerError;
use intmax2_zkp::ethereum_types::u256::U256;

#[derive(Debug, thiserror::Error)]
pub enum BlockBuilderError {
    #[error("Blockchain error: {0}")]
    BlockchainError(#[from] BlockchainError),

    #[error("Server error: {0}")]
    ServerError(#[from] ServerError),

    #[error("Not accepting transactions")]
    NotAcceptingTx,

    #[error("Block is full")]
    BlockIsFull,

    #[error("Only one sender allowed in a block")]
    OnlyOneSenderAllowed,

    #[error("Account already registered pubkey: {0}, account_id: {1}")]
    AccountAlreadyRegistered(U256, u64),

    #[error("Account not found pubkey: {0}")]
    AccountNotFound(U256),
}
