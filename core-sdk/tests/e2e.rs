use ethers::types::H256;
use hashbrown::HashMap;
use intmax2_core_sdk::{
    client::{client::Client, config::ClientConfig},
    external_api::{
        balance_prover::local::LocalBalanceProver,
        block_builder::local::LocalBlockBuilder,
        block_validity_prover::{
            interface::BlockValidityInterface as _, local::LocalBlockValidityProver,
        },
        contract::{interface::ContractInterface, local::LocalContract},
        store_vault_server::local::LocalStoreVaultServer,
        withdrawal_aggregator::local::LocalWithdrawalAggregator,
    },
    utils::init_logger::init_logger,
};
use intmax2_zkp::common::{signature::key_set::KeySet, trees::asset_tree::AssetLeaf};

#[tokio::test]
async fn e2e_test() -> anyhow::Result<()> {
    init_logger();

    let contract = LocalContract::new();
    let store_vault_server = LocalStoreVaultServer::new();
    let validity_prover = LocalBlockValidityProver::new(contract.0.clone());
    let block_builder = LocalBlockBuilder::new(
        contract.0.clone(),
        validity_prover.inner_block_validity_prover.clone(),
    );
    let balance_prover = LocalBalanceProver::new()?;
    let withdrawal_aggregator = LocalWithdrawalAggregator::new()?;

    let config = ClientConfig::default();
    let client = Client {
        config,
        store_vault_server,
        block_builder: block_builder.clone(),
        balance_prover,
        validity_prover: validity_prover.clone(),
        withdrawal_aggregator,
    };

    let mut rng = rand::thread_rng();
    let alice_key = KeySet::rand(&mut rng);

    // deposit 100wei ETH to alice wallet
    let deposit_call = client.prepare_deposit(alice_key, 0, 100.into()).await?;

    contract
        .deposit(
            H256::zero(),
            deposit_call.pubkey_salt_hash,
            deposit_call.token_index,
            deposit_call.amount,
        )
        .await?;

    // post empty block to reflect the deposit
    block_builder.post_empty_block().unwrap();

    // sync validity prover to the latest block
    validity_prover.sync()?;
    log::info!("synced to block {}", validity_prover.block_number().await?);

    // sync alice's balance proof to receive the deposit
    client.sync(alice_key).await?;
    let alice_data = client.get_user_data(alice_key).await?;
    log::info!(
        "Synced alice balance proof to block {}",
        alice_data.block_number
    );
    print_balances(&alice_data.balances());

    Ok(())
}

fn print_balances(balances: &HashMap<u64, AssetLeaf>) {
    for (token_index, asset_leaf) in balances {
        if asset_leaf.is_insufficient {
            continue;
        }
        println!(
            "token index; {}, balance: {}",
            token_index, asset_leaf.amount
        );
    }
}
