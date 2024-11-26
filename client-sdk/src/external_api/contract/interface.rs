use async_trait::async_trait;
use ethers::types::H256;
use intmax2_zkp::ethereum_types::{address::Address, bytes32::Bytes32, u256::U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum BlockchainError {
    #[error("Insufficient funds: {0}")]
    InsufficientFunds(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Decode call data error: {0}")]
    DecodeCallDataError(String),

    #[error("Token not found")]
    TokenNotFound,

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

    async fn claim_withdrawals(
        &self,
        signer_private_key: H256,
        withdrawals: &[ContractWithdrawal],
    ) -> Result<(), BlockchainError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractWithdrawal {
    pub recipient: Address,
    pub token_index: u32,
    pub amount: U256,
    pub id: u32,
}
