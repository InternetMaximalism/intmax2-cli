use intmax2_zkp::{
    common::{
        block_builder::BlockProposal, signature::utils::get_pubkey_hash,
        trees::tx_tree::TxMerkleProof, tx::Tx,
    },
    constants::NUM_SENDERS_IN_BLOCK,
    ethereum_types::{bytes32::Bytes32, u256::U256},
    utils::{leafable::Leafable as _, poseidon_hash_out::PoseidonHashOut},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::external_api::{
    common::error::ServerError,
    utils::{
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryResponse {
    tx_root: Bytes32,
    tx_index: u32,
    tx_tree_merkle_proof: Vec<PoseidonHashOut>,
    public_keys: Vec<Bytes32>,
}

pub async fn query_proposal(
    server_base_url: &str,
    pubkey: Bytes32,
    tx: Tx,
) -> Result<Option<BlockProposal>, ServerError> {
    let url = format!("{}/block/proposed", server_base_url);
    let request = json!({
        "sender": pubkey,
        "txHash": tx.hash(),
        "signature": "" // TODO: implement signature
    });

    let response = with_retry(|| async {
        reqwest_wasm::Client::new()
            .post(&url)
            .json(&request)
            .send()
            .await
    })
    .await
    .map_err(|e| ServerError::NetworkError(format!("Failed to query proposal: {}", e)))?;

    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: QueryResponse = response.json().await.map_err(|e| {
                ServerError::DeserializationError(format!(
                    "Failed to parse query proposal response: {}",
                    e
                ))
            })?;
            let mut pubkeys = info
                .public_keys
                .iter()
                .map(|&x| x.into())
                .collect::<Vec<U256>>();
            pubkeys.resize(NUM_SENDERS_IN_BLOCK, U256::dummy_pubkey());
            let pubkeys_hash = get_pubkey_hash(&pubkeys);

            let tx_merkle_proof = TxMerkleProof::from_siblings(info.tx_tree_merkle_proof);
            let proposal = BlockProposal {
                tx_tree_root: info.tx_root,
                tx_index: info.tx_index,
                tx_merkle_proof,
                pubkeys,
                pubkeys_hash,
            };

            Ok(Some(proposal))
        }
        ResponseType::NotFound(_) => Ok(None),
        _ => Err(ServerError::UnknownError(
            "Failed to query proposal".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use intmax2_zkp::ethereum_types::u32limb_trait::U32LimbTrait;

    use super::*;
    use crate::utils::init_logger::init_logger;

    #[tokio::test]
    async fn test_query_proposal() -> anyhow::Result<()> {
        init_logger();
        let server_base_url = "http://localhost:4000/v1";

        let mut rng = rand::thread_rng();
        let pubkey = Bytes32::rand(&mut rng);
        let tx = Tx::rand(&mut rng);
        let _result = query_proposal(server_base_url, pubkey, tx).await?;

        Ok(())
    }
}
