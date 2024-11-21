use intmax2_zkp::{
    common::trees::sender_tree::SenderLeaf,
    ethereum_types::{bytes32::Bytes32, u256::U256},
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
pub struct TxTreeRootStatus {
    pub block_type: String,
    pub tx_root: Bytes32,
    pub prev_block_hash: Bytes32,
    pub block_number: u32,
    pub deposit_root: Bytes32,
    pub signature_hash: Bytes32,
    pub message_point: Bytes32,
    pub aggregated_public_key: Bytes32,
    pub aggregated_signature: Bytes32,
    pub senders: Vec<Sender>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sender {
    pub public_key: Bytes32,
    pub account_id: u32,
    pub is_signed: bool,
}

fn from_senders(senders: Vec<Sender>) -> Vec<SenderLeaf> {
    let mut senders = senders
        .into_iter()
        .map(|s| SenderLeaf {
            sender: s.public_key.into(),
            did_return_sig: s.is_signed,
        })
        .collect::<Vec<_>>();
    let dummy_leaf = SenderLeaf {
        sender: U256::dummy_pubkey(),
        did_return_sig: false,
    };
    senders.resize(128, dummy_leaf);
    senders
}

pub async fn get_tx_tree_root_status(
    server_base_url: &str,
    tx_tree_root: Bytes32,
) -> Result<Option<(u32, Vec<SenderLeaf>)>, ServerError> {
    let url = format!("{}/tx-root/{}/status", server_base_url, tx_tree_root);
    let response = with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await })
        .await
        .map_err(|e| {
            ServerError::InternalError(format!(
                "Failed to get tx tree root status: {}",
                e.to_string()
            ))
        })?;
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let status: ServerCommonResponse<TxTreeRootStatus> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to parse tx tree root status response: {}",
                        e
                    ))
                })?;
            let sender_leaves = from_senders(status.data.senders);
            let block_number = status.data.block_number;
            Ok(Some((block_number, sender_leaves)))
        }
        ResponseType::NotFound(_) => Ok(None),
        _ => Err(ServerError::InternalError(
            "Failed to get tx tree root status".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::init_logger::init_logger;

    use super::*;

    #[tokio::test]
    async fn test_get_tx_tree_root_status() -> anyhow::Result<()> {
        init_logger();
        let server_base_url = "http://localhost:4000/v1/blockvalidity";
        let tx_tree_root = Bytes32::default();
        let _response = get_tx_tree_root_status(server_base_url, tx_tree_root).await?;
        Ok(())
    }
}
