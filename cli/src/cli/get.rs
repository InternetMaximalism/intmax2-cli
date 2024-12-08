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
        println!("\t Token #{}: {}", i, leaf.amount);
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
