use intmax2_zkp::common::signature::key_set::KeySet;

use super::client::get_client;

pub async fn sync(key: KeySet) -> anyhow::Result<()> {
    let client = get_client()?;
    client.sync(key).await?;
    Ok(())
}

pub async fn sync_withdrawals(key: KeySet) -> anyhow::Result<()> {
    let client = get_client()?;
    client.sync_withdrawals(key).await?;
    Ok(())
}
