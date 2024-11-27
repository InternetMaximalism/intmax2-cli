use ethers::types::Address;
use serde::Deserialize;

pub mod cli;

#[derive(Deserialize)]
pub struct Env {
    // client settings
    pub indexer_base_url: String,
    pub store_vault_server_base_url: String,
    pub validity_prover_base_url: String,
    pub balance_prover_base_url: String,
    pub withdrawal_server_base_url: String,
    pub deposit_timeout: u64,
    pub tx_timeout: u64,

    // blockchain settings
    pub l1_rpc_url: String,
    pub l1_chain_id: u64,
    pub liquidity_contract_address: Address,
    pub l2_rpc_url: String,
    pub l2_chain_id: u64,
    pub rollup_contract_address: Address,
    pub rollup_contract_deployed_block_number: u64,

    // optional block builder base url
    pub block_builder_base_url: Option<String>,
}
