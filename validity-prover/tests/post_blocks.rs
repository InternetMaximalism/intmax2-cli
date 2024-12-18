use ethers::types::H256;
use intmax2_client_sdk::external_api::contract::{
    rollup_contract::RollupContract, utils::get_latest_block_number,
};
use intmax2_zkp::common::signature::SignatureContent;
use serde::Deserialize;
use server_common::logger::init_logger;
use validity_prover::Env;

#[derive(Deserialize)]
struct PrivKeyEnv {
    pub block_builder_private_key: H256,
}

#[tokio::test]
async fn post_blocks() -> anyhow::Result<()> {
    init_logger()?;

    dotenv::dotenv().ok();
    let env = envy::from_env::<Env>().unwrap();
    let priv_key_env = envy::from_env::<PrivKeyEnv>().unwrap();

    let mut rng = rand::thread_rng();
    let rollup_contract = RollupContract::new(
        &env.l2_rpc_url,
        env.l2_chain_id,
        env.rollup_contract_address,
        env.rollup_contract_deployed_block_number,
    );

    let block_number = get_latest_block_number(&env.l2_rpc_url).await?;
    println!("block_number: {:?}", block_number);

    let (keys, signature) = SignatureContent::rand(&mut rng);
    let pubkeys = keys.iter().map(|key| key.pubkey).collect::<Vec<_>>();
    rollup_contract
        .post_registration_block(
            priv_key_env.block_builder_private_key,
            ethers::utils::parse_ether("0.3").unwrap(),
            signature.tx_tree_root,
            signature.sender_flag,
            signature.agg_pubkey,
            signature.agg_signature,
            signature.message_point,
            pubkeys,
        )
        .await?;

    Ok(())
}
