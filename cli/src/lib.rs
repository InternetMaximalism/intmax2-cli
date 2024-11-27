use ethers::types::Address;
use serde::Deserialize;
use url::Url;

pub mod cli;

#[derive(Deserialize)]
pub struct Env {
    pub env: String,

    // client settings
    pub indexer_base_url: Url,
    pub store_vault_server_base_url: Url,
    pub validity_prover_base_url: Url,
    pub balance_prover_base_url: Url,
    pub withdrawal_server_base_url: Url,
    pub deposit_timeout: u64,
    pub tx_timeout: u64,

    // blockchain settings
    pub l1_rpc_url: Url,
    pub l1_chain_id: u64,
    pub liquidity_contract_address: Address,
    pub l2_rpc_url: Url,
    pub l2_chain_id: u64,
    pub rollup_contract_address: Address,
    pub rollup_contract_deployed_block_number: u64,

    // optional block builder base url
    pub block_builder_base_url: Option<Url>,
}
