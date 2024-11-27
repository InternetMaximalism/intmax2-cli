use intmax2_zkp::common::signature::key_set::KeySet;

use super::{client::get_client, error::CliError};

pub async fn sync(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    client.sync(key).await?;
    Ok(())
}

pub async fn sync_withdrawals(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    client.sync_withdrawals(key).await?;
    Ok(())
}
