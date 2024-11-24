use std::sync::Arc;

use ethers::{
    contract::abigen,
    types::{Address, Bytes, H256},
};

use super::utils::get_client_with_signer;

abigen!(ERC1967Proxy, "abi/ERC1967Proxy.json",);

pub struct ProxyContract {
    pub rpc_url: String,
    pub chain_id: u64,
    pub contract_address: ethers::types::Address,
}

impl ProxyContract {
    pub fn new(rpc_url: String, chain_id: u64, contract_address: Address) -> Self {
        Self {
            rpc_url,
            chain_id,
            contract_address,
        }
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
        let contract = ERC1967Proxy::deploy(Arc::new(client), args)?.send().await?;
        let address = contract.address();
        Ok(ProxyContract::new(rpc_url.to_string(), chain_id, address))
    }
}
