use ethers::types::{Address, U256};
use intmax2_zkp::ethereum_types::u32limb_trait::U32LimbTrait as _;

use crate::{Env, EnvType};

use super::error::CliError;

pub fn convert_u256(input: U256) -> intmax2_zkp::ethereum_types::u256::U256 {
    let mut bytes = [0u8; 32];
    input.to_big_endian(&mut bytes);
    let amount = intmax2_zkp::ethereum_types::u256::U256::from_bytes_be(&bytes);
    amount
}

pub fn convert_address(input: Address) -> intmax2_zkp::ethereum_types::address::Address {
    let address =
        intmax2_zkp::ethereum_types::address::Address::from_bytes_be(&input.to_fixed_bytes());
    address
}

pub fn load_env() -> Result<Env, CliError> {
    let env = envy::from_env::<Env>()?;
    Ok(env)
}

pub fn is_local() -> Result<bool, CliError> {
    Ok(load_env()?.env == EnvType::Local)
}

pub async fn post_empty_block() -> Result<(), CliError> {
    let env = envy::from_env::<Env>()?;
    let block_builder_base_url = env.block_builder_base_url.ok_or(CliError::UnexpectedError(
        "BLOCK_BUILDER_BASE_URL".to_string(),
    ))?;
    reqwest::Client::new()
        .post(&format!(
            "{}/block-builder/post-empty-block",
            block_builder_base_url
        ))
        .send()
        .await
        .map_err(|e| CliError::UnexpectedError(e.to_string()))?;
    Ok(())
}
