use ethers::types::H256;
use intmax2_client_sdk::{
    client::{client::Client, config::ClientConfig},
    external_api::{
        balance_prover::BalanceProverClient, block_builder::BlockBuilderClient, contract::liquidity_contract, indexer::IndexerClient, store_vault_server::StoreVaultServerClient, validity_prover::ValidityProverClient, withdrawal_server::WithdrawalServerClient
    },
};
use intmax2_interfaces::api::indexer::interface::IndexerClientInterface;
use intmax2_zkp::{
    common::{
        generic_address::GenericAddress, salt::Salt, signature::key_set::KeySet, transfer::Transfer,
    },
    ethereum_types::u256::U256,
};

use crate::Env;

type BB = BlockBuilderClient;
type S = StoreVaultServerClient;
type V = ValidityProverClient;
type B = BalanceProverClient;
type W = WithdrawalServerClient;

pub fn get_client() -> anyhow::Result<Client<BB, S, V, B, W>> {
    let env = envy::from_env::<Env>()?;
    let block_builder = BB::new();
    let store_vault_server = S::new(&env.store_vault_server_base_url);
    let validity_prover = V::new(&env.validity_prover_base_url);
    let balance_prover = B::new(&env.balance_prover_base_url);
    let withdrawal_server = W::new(&env.withdrawal_server_base_url);

    let config = ClientConfig {
        deposit_timeout: env.deposit_timeout,
        tx_timeout: env.tx_timeout,
    };

    let client = Client {
        block_builder,
        store_vault_server,
        validity_prover,
        balance_prover,
        withdrawal_server,
        config,
    };

    Ok(client)
}

pub async fn deposit(key: KeySet, amount: U256, token_index: u32) -> anyhow::Result<()> {
    let client = get_client()?;
    let deposit_call = client.prepare_deposit(key, token_index, amount).await?;

    let liquidity_contract  = LiquidityContract::new(
        "http://localhost:8545".to_string(),
        1,
        "0x
    );
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
    let env = envy::from_env::<Env>()?;
    let client = get_client()?;

    // get block builder info
    let indexer = IndexerClient::new(&env.indexer_base_url);
    let block_builder_info = indexer.get_block_builder_info().await?;
    // override block builder base url if it is set in the env
    let block_builder_url = if let Some(block_builder_base_url) = env.block_builder_base_url {
        block_builder_base_url
    } else {
        if block_builder_info.is_empty() {
            anyhow::bail!("No block builder available");
        }
        block_builder_info.first().unwrap().url.clone()
    };

    let mut rng = rand::thread_rng();
    let salt = Salt::rand(&mut rng);
    let transfer = Transfer {
        recipient: to,
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
