use intmax2_zkp::common::signature::key_set::KeySet;

use crate::cli::client::get_client;

pub async fn balance(key: KeySet) -> anyhow::Result<()> {
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
