use ethers::types::{Address, H256, U256};
use intmax2_interfaces::data::deposit_data::TokenType;
use intmax2_zkp::common::signature::key_set::KeySet;

use crate::Env;

use super::{
    client::get_client,
    error::CliError,
    utils::{convert_address, convert_u256, is_dev},
};

pub async fn deposit(
    key: KeySet,
    eth_private_key: H256,
    amount: U256,
    token_type: TokenType,
    token_address: Address,
    token_id: U256,
) -> Result<(), CliError> {
    let client = get_client()?;
    let amount = convert_u256(amount);
    let token_address = convert_address(token_address);
    let token_id = convert_u256(token_id);
    let deposit_data = client
        .prepare_deposit(key.pubkey, amount, token_type, token_address, token_id)
        .await?;

    let liquidity_contract = client.liquidity_contract.clone();

    match token_type {
        TokenType::NATIVE => {
            liquidity_contract
                .deposit_native(
                    eth_private_key,
                    deposit_data.pubkey_salt_hash,
                    deposit_data.amount,
                )
                .await?;
        }
        TokenType::ERC20 => {
            liquidity_contract
                .deposit_erc20(
                    eth_private_key,
                    deposit_data.pubkey_salt_hash,
                    deposit_data.amount,
                    deposit_data.token_address,
                )
                .await?;
        }
        TokenType::ERC721 => {
            liquidity_contract
                .deposit_erc721(
                    eth_private_key,
                    deposit_data.pubkey_salt_hash,
                    deposit_data.token_address,
                    deposit_data.token_id,
                )
                .await?;
        }
        TokenType::ERC1155 => {
            liquidity_contract
                .deposit_erc1155(
                    eth_private_key,
                    deposit_data.pubkey_salt_hash,
                    deposit_data.token_address,
                    deposit_data.token_id,
                    deposit_data.amount,
                )
                .await?;
        }
    }

    // relay deposits by self if env is dev
    if is_dev()? {
        let token_index = liquidity_contract
            .get_token_index(token_type, token_address, token_id)
            .await?
            .ok_or(CliError::UnexpectedError(
                "Cloud not find token index".to_string(),
            ))?;
        let mut deposit_data = deposit_data;
        deposit_data.set_token_index(token_index);
        client
            .rollup_contract
            .process_deposits(eth_private_key, 0, &[deposit_data.deposit_hash().unwrap()])
            .await?;
        // post empty block
        post_empty_block().await?;
    }

    Ok(())
}

async fn post_empty_block() -> Result<(), CliError> {
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
