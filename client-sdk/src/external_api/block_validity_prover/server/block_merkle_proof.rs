use intmax2_zkp::{
    common::trees::block_hash_tree::BlockHashMerkleProof, ethereum_types::bytes32::Bytes32,
};
use serde::{Deserialize, Serialize};

use crate::external_api::{
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockMerkleProofRespnse {
    pub merkle_proof: BlockHashMerkleProof,
    pub root_hash: Bytes32,
}

pub async fn get_block_merkle_proof(
    server_base_url: &str,
    root_block_number: u32,
    leaf_block_number: u32,
) -> Result<BlockHashMerkleProof, ServerError> {
    let url = format!(
        "{}/block-merkle-proof/{}/{}",
        server_base_url, root_block_number, leaf_block_number
    );
    let response = with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await })
        .await
        .map_err(|e| {
            ServerError::NetworkError(format!(
                "Failed to get block merkle proof: {}",
                e.to_string()
            ))
        })?;

    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: ServerCommonResponse<BlockMerkleProofRespnse> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to parse block merkle proof response: {}",
                        e.to_string()
                    ))
                })?;
            Ok(info.data.merkle_proof)
        }
        _ => Err(ServerError::UnknownError(
            "Failed to get block merkle proof".to_string(),
        )),
    }
}
