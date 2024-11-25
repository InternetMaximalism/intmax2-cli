use std::env;

use ethers::types::H256;
use intmax2_client_sdk::{
    client::{client::Client, config::ClientConfig},
    external_api::{
        balance_prover::BalanceProverClient, block_builder::BlockBuilderClient,
        indexer::IndexerClient, store_vault_server::StoreVaultServerClient,
        validity_prover::ValidityProverClient, withdrawal_server::WithdrawalServerClient,
    },
};
use intmax2_interfaces::api::indexer::interface::IndexerClientInterface;
use intmax2_zkp::{
    common::{
        generic_address::GenericAddress, salt::Salt, signature::key_set::KeySet, transfer::Transfer,
    },
    ethereum_types::u256::U256,
};

type BB = BlockBuilderClient;
type S = StoreVaultServerClient;
type V = ValidityProverClient;
type B = BalanceProverClient;
type W = WithdrawalServerClient;

pub fn get_base_url() -> String {
    env::var("BASE_URL").expect("BASE_URL must be set")
}

pub fn get_indexer_url() -> String {
    // todo: remove this line in production
    "https://dev.builder.indexer.intmax.xyz".to_string()
}

pub fn get_client() -> anyhow::Result<Client<BB, S, V, B, W>> {
    let base_url = get_base_url();
    let block_builder = BB::new();
    let store_vault_server = S::new(&base_url);
    let validity_prover = V::new(&base_url);
    let balance_prover = B::new(&base_url);
    let withdrawal_aggregator = W::new(&base_url);

    let config = ClientConfig {
        deposit_timeout: 3600,
        tx_timeout: 60,
    };

    let client = Client {
        block_builder,
        store_vault_server,
        validity_prover,
        balance_prover,
        withdrawal_aggregator,
        config,
    };

    Ok(client)
}

pub fn get_contract() -> BC {
    let base_url = get_base_url();
    let contract = BC::new(base_url.clone());
    contract
}

pub async fn deposit(key: KeySet, amount: U256, token_index: u32) -> anyhow::Result<()> {
    let client = get_client()?;
    let deposit_call = client.prepare_deposit(key, token_index, amount).await?;

    let contract = get_contract();
    contract
        .deposit(
            H256::default(),
            deposit_call.pubkey_salt_hash,
            deposit_call.token_index,
            deposit_call.amount,
        )
        .await?;
    Ok(())
}

pub async fn tx(
    key: KeySet,
    to: GenericAddress,
    amount: U256,
    token_index: u32,
) -> anyhow::Result<()> {
    let client = get_client()?;

    // get block builder info
    let indexer = IndexerClient::new(&&get_indexer_url());
    let block_builder_info = indexer.get_block_builder_info().await?;
    if block_builder_info.is_empty() {
        anyhow::bail!("No block builder available");
    }
    let _block_builder_url = block_builder_info.first().unwrap().url.clone();
    let block_builder_url = &get_base_url(); // todo: remove this line in production

    let mut rng = rand::thread_rng();
    let salt = Salt::rand(&mut rng);
    let transfer = Transfer {
        recipient: to,
        amount,
        token_index,
        salt,
    };
    let memo = client
        .send_tx_request(block_builder_url, key, vec![transfer])
        .await?;
    log::info!("Waiting for block builder to build the block");

    // sleep for a while to wait for the block builder to build the block
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    let mut tries = 0;
    let proposal = loop {
        let proposal = client
            .query_proposal(block_builder_url, key, memo.tx.clone())
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
        .finalize_tx(block_builder_url, key, &memo, &proposal)
        .await?;

    Ok(())
}

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
