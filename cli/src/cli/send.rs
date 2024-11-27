use ethers::types::U256;
use intmax2_client_sdk::external_api::indexer::IndexerClient;
use intmax2_interfaces::api::indexer::interface::IndexerClientInterface;
use intmax2_zkp::common::{
    generic_address::GenericAddress, salt::Salt, signature::key_set::KeySet, transfer::Transfer,
};

use crate::{
    cli::{client::get_client, utils::convert_u256},
    Env,
};

use super::error::CliError;

pub async fn tx(
    key: KeySet,
    recipient: GenericAddress,
    amount: U256,
    token_index: u32,
) -> Result<(), CliError> {
    let env = envy::from_env::<Env>()?;
    let client = get_client()?;

    // override block builder base url if it is set in the env
    let block_builder_url = if let Some(block_builder_base_url) = env.block_builder_base_url {
        block_builder_base_url.to_string()
    } else {
        // get block builder info
        let indexer = IndexerClient::new(&env.indexer_base_url.to_string());
        let block_builder_info = indexer.get_block_builder_info().await?;
        if block_builder_info.is_empty() {
            return Err(CliError::UnexpectedError(
                "Block builder info is empty".to_string(),
            ));
        }
        block_builder_info.first().unwrap().url.clone()
    };

    let mut rng = rand::thread_rng();
    let salt = Salt::rand(&mut rng);

    let amount = convert_u256(amount);
    let transfer = Transfer {
        recipient,
        amount,
        token_index,
        salt,
    };
    let mut tries = 0;
    let memo = loop {
        let res = client
            .send_tx_request(&block_builder_url, key, vec![transfer])
            .await;
        if let Ok(memo) = res {
            break memo;
        }
        if tries > env.block_builder_query_limit {
            return Err(CliError::FailedToRequestTx);
        }
        tries += 1;
        log::info!(
            "Failed to request tx, retrying in {} seconds",
            env.block_builder_request_interval
        );
        tokio::time::sleep(std::time::Duration::from_secs(
            env.block_builder_request_interval,
        ))
        .await;
    };
    let is_registration_block = memo.is_registration_block;
    let tx = memo.tx.clone();

    log::info!("Waiting for block builder to build the block");
    tokio::time::sleep(std::time::Duration::from_secs(
        env.block_builder_query_wait_time,
    ))
    .await;

    let mut tries = 0;
    let proposal = loop {
        let proposal = client
            .query_proposal(&block_builder_url, key, is_registration_block, tx)
            .await?;
        if proposal.is_some() {
            break proposal.unwrap();
        }
        if tries > env.block_builder_query_limit {
            return Err(CliError::FailedToGetProposal);
        }
        tries += 1;
        log::info!(
            "Failed to get proposal, retrying in {} seconds",
            env.block_builder_query_interval
        );
        tokio::time::sleep(std::time::Duration::from_secs(
            env.block_builder_query_interval,
        ))
        .await;
    };

    log::info!("Finalizing tx");
    client
        .finalize_tx(&block_builder_url, key, &memo, &proposal)
        .await?;

    Ok(())
}
