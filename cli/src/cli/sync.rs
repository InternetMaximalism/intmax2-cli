use colored::Colorize as _;
use intmax2_client_sdk::client::error::ClientError;
use intmax2_zkp::common::signature::key_set::KeySet;

use super::{client::get_client, error::CliError};

pub async fn sync(key: KeySet) -> Result<bool, CliError> {
    let client = get_client()?;
    match client.sync(key).await {
        Ok(_) => {
            log::info!("Synced successfully");
        }
        Err(e) => match e {
            ClientError::PendingError(_) => {
                println!(
                    "{}",
                    "There are pending actions. Please try again later.".red()
                );
                return Ok(false);
            }
            _ => {
                return Err(CliError::UnexpectedError(format!("{:?}", e)));
            }
        },
    }
    Ok(true)
}

pub async fn sync_withdrawals(key: KeySet) -> Result<(), CliError> {
    let client = get_client()?;
    sync(key).await?;
    client.sync_withdrawals(key).await?;
    Ok(())
}
