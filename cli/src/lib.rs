use serde::Deserialize;

pub mod cli;

#[derive(Deserialize)]
pub struct Env {
    pub indexer_base_url: String,
    pub store_vault_server_base_url: String,
    pub validity_prover_base_url: String,
    pub balance_prover_base_url: String,
    pub withdrawal_server_base_url: String,
    pub deposit_timeout: u64,
    pub tx_timeout: u64,
}
