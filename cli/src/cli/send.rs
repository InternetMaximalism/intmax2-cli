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

pub async fn tx(
    key: KeySet,
    recipient: GenericAddress,
    amount: U256,
    token_index: u32,
) -> anyhow::Result<()> {
    let env = envy::from_env::<Env>()?;
    let client = get_client()?;

    // get block builder info
    let indexer = IndexerClient::new(&env.indexer_base_url.to_string());
    let block_builder_info = indexer.get_block_builder_info().await?;

    // override block builder base url if it is set in the env
    let block_builder_url = if let Some(block_builder_base_url) = env.block_builder_base_url {
        block_builder_base_url.to_string()
    } else {
        if block_builder_info.is_empty() {
            anyhow::bail!("No block builder available");
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
    let memo = client
        .send_tx_request(&block_builder_url, key, vec![transfer])
        .await?;
    let is_registration_block = memo.is_registration_block;
    let tx = memo.tx.clone();
    log::info!("Waiting for block builder to build the block");

    // sleep for a while to wait for the block builder to build the block
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    let mut tries = 0;
    let proposal = loop {
        let proposal = client
            .query_proposal(&block_builder_url, key, is_registration_block, tx)
            .await?;
        if proposal.is_some() {
            break proposal.unwrap();
        }
        if tries > 5 {
            anyhow::bail!("Failed to get proposal");
        }
        tries += 1;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    };

    log::info!("Finalizing tx");
    client
        .finalize_tx(&block_builder_url, key, &memo, &proposal)
        .await?;

    Ok(())
}
