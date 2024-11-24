use std::sync::Arc;

use ethers::{
    contract::abigen,
    types::{Address, Bytes, H256},
};

use super::utils::{get_client_with_signer, get_latest_block_number};

abigen!(ERC1967Proxy, "abi/ERC1967Proxy.json",);

pub struct ProxyContract {
    pub rpc_url: String,
    pub chain_id: u64,
    address: ethers::types::Address,
    deployed_block_number: u64,
}

impl ProxyContract {
    pub fn address(&self) -> Address {
        self.address
    }

    pub fn deployed_block_number(&self) -> u64 {
        self.deployed_block_number
    }

    pub async fn deploy(
        rpc_url: &str,
        chain_id: u64,
        private_key: H256,
        impl_address: Address,
        constructor: &[u8],
    ) -> anyhow::Result<ProxyContract> {
        let client = get_client_with_signer(rpc_url, chain_id, private_key).await?;
        let args = (impl_address, Bytes::from(constructor.to_vec()));
        let deployed_block_number = get_latest_block_number(rpc_url).await?;
        let contract = ERC1967Proxy::deploy(Arc::new(client), args)?.send().await?;
        let address = contract.address();
        Ok(ProxyContract {
            rpc_url: rpc_url.to_string(),
            chain_id,
            address,
            deployed_block_number,
        })
    }
}
