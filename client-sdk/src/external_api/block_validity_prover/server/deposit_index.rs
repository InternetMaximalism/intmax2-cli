use intmax2_zkp::ethereum_types::{address::Address, bytes32::Bytes32, u256::U256};
use serde::Deserialize;

use crate::external_api::{
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDepositsResponse {
    deposits: Vec<DepositInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositInfo {
    pub deposit_hash: Bytes32,
    pub deposit_id: u32,
    pub deposit_index: u32,
    pub block_number: u32,
    pub is_synchronized: bool,
    pub deposit_tx_hash: Bytes32,
    pub deposit_proccessed_tx_hash: Bytes32,
    pub from: Address,
    pub deposit_leaf: DepositLeaf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositLeaf {
    pub recipient_salt_hash: Bytes32,
    pub token_index: u32,
    pub amount: U256,
}

pub async fn get_deposit_index_and_block_number(
    server_base_url: &str,
    deposit_hash: Bytes32,
) -> Result<Option<(u32, u32)>, ServerError> {
    let url = format!(
        "{}/deposits?depositHashes={}",
        server_base_url, deposit_hash
    );
    let response = with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await })
        .await
        .map_err(|e| {
            ServerError::InternalError(format!(
                "Failed to get deposit index and block number: {}",
                e.to_string()
            ))
        })?;

    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: ServerCommonResponse<GetDepositsResponse> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to parse deposit index and block number response: {}",
                        e.to_string()
                    ))
                })?;
            if info.data.deposits.is_empty() {
                return Ok(None);
            }

            let block_number = info.data.deposits[0].block_number;
            let deposit_index = info.data.deposits[0].deposit_index;

            Ok(Some((deposit_index, block_number)))
        }
        ResponseType::NotFound(_) => Ok(None),
        _ => Err(ServerError::InternalError(
            "Failed to get deposit index and block number".to_string(),
        )),
    }
}
