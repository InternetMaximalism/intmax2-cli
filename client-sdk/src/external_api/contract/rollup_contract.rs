use std::sync::Arc;

use ethers::{
    contract::abigen,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::Wallet,
    types::{Address, H256},
};

use super::{
    interface::BlockchainError,
    utils::{get_client, get_client_with_signer},
};

const NUMBER_OF_QUERY_BLOCKS: u64 = 10000;

abigen!(Rollup, "abi/Rollup.json",);

#[derive(Debug, Clone)]
pub struct RollupContract {
    pub rpc_url: String,
    pub chain_id: u64,
    pub contract_address: Address,
    pub deployed_block_number: u64,
}

impl RollupContract {
    pub fn new(
        rpc_url: String,
        chain_id: u64,
        contract_address: Address,
        deployed_block_number: u64,
    ) -> Self {
        Self {
            rpc_url,
            chain_id,
            contract_address,
            deployed_block_number,
        }
    }

    pub async fn get_contract(&self) -> Result<rollup::Rollup<Provider<Http>>, BlockchainError> {
        let client = get_client(&self.rpc_url).await?;
        let contract = Rollup::new(self.contract_address, client);
        Ok(contract)
    }

    pub async fn get_contract_with_signer(
        &self,
        private_key: H256,
    ) -> Result<rollup::Rollup<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>, BlockchainError>
    {
        let client = get_client_with_signer(&self.rpc_url, self.chain_id, private_key).await?;
        let contract = Rollup::new(self.contract_address, Arc::new(client));
        Ok(contract)
    }

    pub async fn get_leaf_deposited_event(
        &self,
        from_block: Option<u64>,
    ) -> Result<Vec<rollup::DepositLeafInserted>, BlockchainError> {
        let contract = self.get_contract().await?;
        let from_block = from_block.unwrap_or(self.deployed_block_number);
        

        Ok(logs)
    }
}
