use intmax2_client_sdk::external_api::contract::interface::BlockchainError;

#[derive(Debug, thiserror::Error)]
pub enum BlockBuilderError {
    #[error("Error in the block builder: {0}")]
    BlockchainError(#[from] BlockchainError),
}
