use intmax2_client_sdk::client::{client::Client, config::ClientConfig};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

type BC = TestContract;
type BB = TestBlockBuilder;
type S = TestStoreVaultServer;
type V = TestBlockValidityProver;
type B = TestBalanceProver;
type W = TestWithdrawalAggregator;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[wasm_bindgen(getter_with_clone)]
pub struct Config {
    /// URL of the store vault server
    pub store_vault_server_url: String,

    /// URL of the block validity prover
    pub block_validity_prover_url: String,

    /// URL of the balance prover
    pub balance_prover_url: String,

    /// URL of the withdrawal aggregator
    pub withdrawal_aggregator_url: String,

    /// Time to reach the rollup contract after taking a backup of the deposit
    /// If this time is exceeded, the deposit backup will be ignored
    pub deposit_timeout: u64,

    /// Time to reach the rollup contract after sending a tx request
    /// If this time is exceeded, the tx request will be ignored
    pub tx_timeout: u64,
}

#[wasm_bindgen]
impl Config {
    #[wasm_bindgen]
    pub fn new(
        store_vault_server_url: String,
        block_validity_prover_url: String,
        balance_prover_url: String,
        withdrawal_aggregator_url: String,
        deposit_timeout: u64,
        tx_timeout: u64,
    ) -> Config {
        Config {
            store_vault_server_url,
            block_validity_prover_url,
            balance_prover_url,
            withdrawal_aggregator_url,
            deposit_timeout,
            tx_timeout,
        }
    }
}

pub fn get_client(config: &Config) -> Client<BB, S, V, B, W> {
    let block_builder = BB::new();
    let store_vault_server = S::new(config.store_vault_server_url.clone());
    let validity_prover = V::new(config.block_validity_prover_url.clone());
    let balance_prover = B::new(config.balance_prover_url.clone());
    let withdrawal_aggregator = W::new(config.withdrawal_aggregator_url.clone());

    let client_config = ClientConfig {
        deposit_timeout: config.deposit_timeout,
        tx_timeout: config.tx_timeout,
    };

    Client {
        block_builder,
        store_vault_server,
        validity_prover,
        balance_prover,
        withdrawal_aggregator,
        config: client_config,
    }
}

pub fn get_mock_contract(contract_server_url: &str) -> BC {
    BC::new(contract_server_url.to_string())
}
