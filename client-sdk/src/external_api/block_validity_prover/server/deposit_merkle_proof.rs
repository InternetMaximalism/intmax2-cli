use intmax2_zkp::{
    common::trees::deposit_tree::DepositMerkleProof, ethereum_types::bytes32::Bytes32,
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
pub struct DepositMerkleProofRespnse {
    pub deposit_id: usize,
    pub deposit_index: usize,
    pub merkle_proof: DepositMerkleProof,
    pub root_hash: Bytes32,
}

pub async fn get_deposit_merkle_proof(
    server_base_url: &str,
    block_number: u32,
    deposit_index: u32,
) -> Result<DepositMerkleProof, ServerError> {
    let url = format!(
        "{}/deposit-tree-proof/{}/{}",
        server_base_url, block_number, deposit_index
    );
    let response = with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await })
        .await
        .map_err(|e| {
            ServerError::NetworkError(format!(
                "Failed to get deposit merkle proof: {}",
                e.to_string()
            ))
        })?;

    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: ServerCommonResponse<DepositMerkleProofRespnse> =
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

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_get_deposit_merkle_proof() -> anyhow::Result<()> {
        let server_base_url = "http://localhost:4000/v1/blockvalidity";
        let block_number = 1;
        let deposit_index = 1;
        let deposit_merkle_proof =
            super::get_deposit_merkle_proof(server_base_url, block_number, deposit_index).await?;
        println!("{:?}", deposit_merkle_proof);
        Ok(())
    }
}
