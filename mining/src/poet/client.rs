use anyhow::Context;
use intmax2_client_sdk::{
    client::{client::Client, config::ClientConfig},
    external_api::{
        balance_prover::BalanceProverClient,
        block_builder::BlockBuilderClient,
        contract::{liquidity_contract::LiquidityContract, rollup_contract::RollupContract},
        store_vault_server::StoreVaultServerClient,
        validity_prover::ValidityProverClient,
        withdrawal_server::WithdrawalServerClient,
    },
};

use crate::env_var::EnvVar;

type BB = BlockBuilderClient;
type S = StoreVaultServerClient;
type V = ValidityProverClient;
type B = BalanceProverClient;
type W = WithdrawalServerClient;

pub fn get_client() -> anyhow::Result<Client<BB, S, V, B, W>> {
    let env =
        envy::from_env::<EnvVar>().with_context(|| "Failed to parse environment variables")?;
    let block_builder = BB::new();
    let store_vault_server = S::new(&env.store_vault_server_base_url);

    let validity_prover = V::new(&env.validity_prover_base_url);
    let balance_prover = B::new(&env.balance_prover_base_url);
    let withdrawal_server = W::new(&env.withdrawal_server_base_url);

    let liquidity_contract = LiquidityContract::new(
        &env.l1_rpc_url,
        env.l1_chain_id,
        env.liquidity_contract_address,
    );
    let rollup_contract = RollupContract::new(
        &env.l2_rpc_url,
        env.l2_chain_id,
        env.rollup_contract_address,
        env.rollup_contract_deployed_block_number,
    );

    let config = ClientConfig {
        deposit_timeout: env.deposit_timeout,
        tx_timeout: env.tx_timeout,
        block_builder_request_interval: env.block_builder_request_interval,
        block_builder_request_limit: env.block_builder_request_limit,
    };

    let client = Client {
        block_builder,
        store_vault_server,
        validity_prover,
        balance_prover,
        withdrawal_server,
        liquidity_contract,
        rollup_contract,
        config,
    };

    Ok(client)
}
