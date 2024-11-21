use intmax2_zkp::{
    common::{signature::flatten::FlatG2, tx::Tx},
    ethereum_types::{bytes32::Bytes32, u32limb_trait::U32LimbTrait as _},
    utils::leafable::Leafable,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::external_api::{
    block_builder::interface::FeeProof,
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostSignatureResponse {
    message: String,
}

pub async fn post_signature(
    server_base_url: &str,
    pubkey: Bytes32,
    tx: Tx,
    signature: FlatG2,
    fee_proof: Option<FeeProof>,
) -> Result<(), ServerError> {
    let url = format!("{}/block/signature", server_base_url);
    let request = json!({
        "sender": pubkey,
        "txHash": tx.hash(),
        "signature": signature_to_hex_string(signature),
        "feeProof": fee_proof
    });
    let response = with_retry(|| async {
        reqwest_wasm::Client::new()
            .post(&url)
            .json(&request)
            .send()
            .await
    })
    .await
    .map_err(|e| ServerError::NetworkError(format!("Failed to send tx request: {}", e)))?;

    log::info!("response: {:?}", response.status());
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: ServerCommonResponse<PostSignatureResponse> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to parse tx request response: {}",
                        e
                    ))
                })?;
            if !info.success {
                return Err(ServerError::InvalidResponse(info.data.message));
            }
            Ok(())
        }
        _ => Err(ServerError::InternalError(
            "Failed to send tx request".to_string(),
        )),
    }
}

fn signature_to_hex_string(signature: FlatG2) -> String {
    let bytes = signature
        .0
        .into_iter()
        .flat_map(|x| x.to_bytes_be())
        .collect::<Vec<u8>>();
    let hex = "0x".to_string() + &hex::encode(bytes);
    hex
}
