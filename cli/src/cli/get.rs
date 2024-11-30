use intmax2_zkp::common::signature::key_set::KeySet;

use crate::cli::client::get_client;

use super::error::CliError;

pub async fn balance(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    client.sync(key).await?;

    let user_data = client.get_user_data(key).await?;
    let balances = user_data.balances();

    println!("Balances:");
    for (i, leaf) in balances.iter() {
        println!("\t Token {}: {}", i, leaf.amount);
    }
    Ok(())
}

pub async fn withdrawal_status(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    let withdrawal_info = client.get_withdrawal_info(key).await?;
    for (i, withdrawal_info) in withdrawal_info.iter().enumerate() {
        let withdrawal = withdrawal_info.withdrawal.clone();
        println!(
            "#{}: recipient: {}, token_index{}, amount: {}, status: {}",
            i,
            withdrawal.recipient,
            withdrawal.token_index,
            withdrawal.amount,
            withdrawal_info.status
        );
    }
    Ok(())
}
