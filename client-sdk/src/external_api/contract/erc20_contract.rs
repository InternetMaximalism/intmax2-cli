use std::sync::Arc;

use ethers::{
    contract::abigen,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::Wallet,
    types::{Address, H256, U256},
};

use crate::external_api::utils::retry::with_retry;

use super::{
    handlers::handle_contract_call,
    interface::BlockchainError,
    utils::{get_address, get_client, get_client_with_signer},
};

abigen!(
    ERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
    ]"#,
);

#[derive(Debug, Clone)]
pub struct ERC20Contract {
    pub rpc_url: String,
    pub chain_id: u64,
    pub address: Address,
}

impl ERC20Contract {
    pub fn new(rpc_url: String, chain_id: u64, address: Address) -> Self {
        Self {
            rpc_url,
            chain_id,
            address,
        }
    }

    pub async fn get_contract(&self) -> Result<ERC20<Provider<Http>>, BlockchainError> {
        let client = get_client(&self.rpc_url).await?;
        let contract = ERC20::new(self.address, client);
        Ok(contract)
    }

    async fn get_contract_with_signer(
        &self,
        private_key: H256,
    ) -> Result<ERC20<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>, BlockchainError> {
        let client = get_client_with_signer(&self.rpc_url, self.chain_id, private_key).await?;
        let contract = ERC20::new(self.address, Arc::new(client));
        Ok(contract)
    }

    pub async fn balance_of(&self, account: Address) -> Result<U256, BlockchainError> {
        let contract = self.get_contract().await?;
        let balance = with_retry(|| async { contract.balance_of(account).call().await })
            .await
            .map_err(|e| BlockchainError::NetworkError(format!("Failed to get balance: {}", e)))?;
        Ok(balance)
    }

    pub async fn approve(
        &self,
        signer_private_key: H256,
        spender: Address,
        amount: U256,
    ) -> Result<(), BlockchainError> {
        let contract = self.get_contract_with_signer(signer_private_key).await?;
        let mut tx = contract.approve(spender, amount);
        handle_contract_call(
            &mut tx,
            get_address(self.chain_id, signer_private_key),
            "token_owner",
            "approve",
        )
        .await?;
        Ok(())
    }

    pub async fn allowance(
        &self,
        owner: Address,
        spender: Address,
    ) -> Result<U256, BlockchainError> {
        let contract = self.get_contract().await?;
        let allowance = with_retry(|| async { contract.allowance(owner, spender).call().await })
            .await
            .map_err(|e| {
                BlockchainError::NetworkError(format!("Failed to get allowance: {}", e))
            })?;
        Ok(allowance)
    }
}
