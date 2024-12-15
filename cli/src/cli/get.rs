use intmax2_interfaces::data::deposit_data::TokenType;
use intmax2_zkp::common::{signature::key_set::KeySet, trees::asset_tree::AssetLeaf};

use crate::cli::{client::get_client, sync::sync};

use super::error::CliError;

pub async fn balance(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    if !sync(key.clone()).await? {
        return Ok(());
    }
    let user_data = client.get_user_data(key).await?;
    let mut balances: Vec<(u64, AssetLeaf)> = user_data.balances().into_iter().collect();
    balances.sort_by_key(|(i, _leaf)| *i);

    println!("Balances:");
    for (i, leaf) in balances.iter() {
        let (token_type, address, token_id) =
            client.liquidity_contract.get_token_info(*i as u32).await?;
        println!("\t Token #{}:", i);
        println!("\t\t Amount: {}", leaf.amount);
        println!("\t\t Type: {}", token_type.to_string());

        match token_type {
            TokenType::NATIVE => {}
            TokenType::ERC20 => {
                println!("\t\t Address: {}", address);
            }
            TokenType::ERC721 => {
                println!("\t\t Address: {}", address);
                println!("\t\t Token ID: {}", token_id);
            }
            TokenType::ERC1155 => {
                println!("\t\t Address: {}", address);
                println!("\t\t Token ID: {}", token_id);
            }
        }
    }
    Ok(())
}

pub async fn withdrawal_status(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    let withdrawal_info = client.get_withdrawal_info(key).await?;
    println!("Withdrawal status:");
    for (i, withdrawal_info) in withdrawal_info.iter().enumerate() {
        let withdrawal = withdrawal_info.contract_withdrawal.clone();
        println!(
            "#{}: recipient: {}, token_index: {}, amount: {}, status: {}",
            i,
            withdrawal.recipient,
            withdrawal.token_index,
            withdrawal.amount,
            withdrawal_info.status
        );
    }
    Ok(())
}

pub async fn history(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    let history = client.fetch_history(key).await?;
    println!("History:");
    for entry in history {
        println!("{}", entry);
    }
    Ok(())
}
