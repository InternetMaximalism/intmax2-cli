use ethers::types::{Address, U256};
use intmax2_client_sdk::external_api::contract::{
    liquidity_contract::LiquidityContract, rollup_contract::RollupContract,
};
use intmax2_zkp::common::signature::key_set::KeySet;

use crate::Env;

use super::client::get_client;

pub async fn deposit(key: KeySet, amount: U256, token_address: Address) -> anyhow::Result<()> {
    let env = envy::from_env::<Env>()?;
    let client = get_client()?;

    let liquidity_contract = LiquidityContract::new(
        &env.l1_rpc_url.to_string(),
        env.l1_chain_id,
        env.liquidity_contract_address,
    );
    let rollup_contract = RollupContract::new(
        &env.l2_rpc_url.to_string(),
        env.l2_chain_id,
        env.rollup_contract_address,
        env.rollup_contract_deployed_block_number,
    );

    let deposit_call = client.prepare_deposit(key, token_index, amount).await?;

    Ok(())
}
