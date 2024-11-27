use std::{thread::sleep, time::Duration};

use ethers::types::H256;
use intmax2_client_sdk::external_api::contract::rollup_contract::RollupContract;
use intmax2_zkp::{
    common::signature::SignatureContent,
    ethereum_types::{bytes32::Bytes32, u32limb_trait::U32LimbTrait},
};
use serde::Deserialize;
use validity_prover::Env;

#[derive(Deserialize)]
struct PrivKeyEnv {
    pub block_builder_private_key: H256,
}

#[tokio::test]
async fn post_blocks() -> anyhow::Result<()> {
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

    for i in 0..3 {
        let (keys, signature) = SignatureContent::rand(&mut rng);
        let pubkeys = keys.iter().map(|key| key.pubkey).collect::<Vec<_>>();

        println!("Post registration block {}", i + 1);
        rollup_contract
            .post_registration_block(
                priv_key_env.block_builder_private_key,
                ethers::utils::parse_ether("1").unwrap(),
                signature.tx_tree_root,
                signature.sender_flag,
                signature.agg_pubkey,
                signature.agg_signature,
                signature.message_point,
                pubkeys,
            )
            .await?;

        rollup_contract
            .process_deposits(
                priv_key_env.block_builder_private_key,
                0,
                &[Bytes32::rand(&mut rng)],
            )
            .await?;

        sleep(Duration::from_secs(30));
    }

    Ok(())
}
