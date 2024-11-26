pub async fn balance(key: KeySet) -> anyhow::Result<()> {
    let client = get_client()?;
    client.sync(key).await?;

    let user_data = client.get_user_data(key).await?;
    let balances = user_data.balances();
    for (i, leaf) in balances.iter() {
        println!("Token {}: {}", i, leaf.amount);
    }
    println!("-----------------------------------");
    Ok(())
}
