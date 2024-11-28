use ethers::types::{Address, H256, U256};
use intmax2_interfaces::data::deposit_data::TokenType;
use intmax2_zkp::common::signature::key_set::KeySet;

use super::{
    client::get_client,
    error::CliError,
    utils::{convert_address, convert_u256, is_dev},
};

pub async fn deposit_ft(
    key: KeySet,
    eth_private_key: H256,
    amount: U256,
    token_type: TokenType,
    token_address: Address,
    token_id: Option<U256>,
) -> Result<(), CliError> {
    let client = get_client()?;
    let amount = convert_u256(amount);
    let token_address = convert_address(token_address);
    let token_id = token_id.map(convert_u256).unwrap_or_default();
    let deposit_data = client
        .prepare_deposit(key.pubkey, amount, token_type, token_address, token_id)
        .await?;

    let liquidity_contract = client.liquidity_contract.clone();
    if token_type == TokenType::NATIVE {
        liquidity_contract
            .deposit_native(
                eth_private_key,
                deposit_data.pubkey_salt_hash,
                deposit_data.amount,
            )
            .await?;
    } else {
        liquidity_contract
            .deposit_erc20(
                eth_private_key,
                deposit_data.pubkey_salt_hash,
                deposit_data.amount,
                deposit_data.token_address,
            )
            .await?;
    };

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
    }

    Ok(())
}
