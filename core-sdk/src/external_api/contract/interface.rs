use async_trait::async_trait;
use ethers::types::H256;
use intmax2_zkp::ethereum_types::{bytes32::Bytes32, u256::U256};

#[derive(Debug, thiserror::Error)]
pub enum BlockchainError {
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

#[async_trait(?Send)]
pub trait ContractInterface {
    async fn deposit(
        &self,
        signer_private_key: H256,
        pubkey_salt_hash: Bytes32,
        token_index: u32,
        amount: U256,
    ) -> Result<(), BlockchainError>;
}
