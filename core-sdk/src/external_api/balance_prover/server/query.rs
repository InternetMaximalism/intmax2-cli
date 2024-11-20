use base64::{prelude::BASE64_STANDARD, Engine as _};
use intmax2_zkp::{
    common::signature::key_set::KeySet,
    ethereum_types::{bytes32::Bytes32, u32limb_trait::U32LimbTrait as _},
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use serde::{Deserialize, Serialize};

use crate::external_api::{
    common::error::ServerError,
    utils::{retry::with_retry, time::sleep_for},
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

// todo: add encryption and decryption
pub async fn request_and_fetch_proof<I: Serialize>(
    base_url: &str,
    transition_type: &str,
    input: &I,
    _key: KeySet, // to encrypt and decrypt the data
) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
    let request_id = create_request(base_url, transition_type, input).await?;
    let mut tries = 0;
    let result = loop {
        let res = query_request(base_url, &request_id).await?;
        if res.status == "pending" {
            // wait for 1 second
            sleep_for(1).await;
        } else if res.status == "success" {
            break res.result;
        } else {
            return Err(ServerError::InvalidResponse(format!(
                "Failed to query TEE server: {}",
                res.status
            )));
        }
        tries += 1;
        if tries > 60 {
            return Err(ServerError::InvalidResponse(
                "Failed to query TEE server: timeout".to_string(),
            ));
        }
    };
    let proof_bytes = BASE64_STANDARD
        .decode(&result)
        .map_err(|e| ServerError::InternalError(format!("Failed to decode proof: {}", e)))?;
    let proof = bincode::deserialize(&proof_bytes).map_err(|e| {
        ServerError::DeserializationError(format!("Failed to deserialize proof: {}", e))
    })?;
    Ok(proof)
}

async fn create_request<I: Serialize>(
    base_url: &str,
    transition_type: &str,
    input: &I,
) -> Result<String, ServerError> {
    let url = format!("{}/v1/proof/create", base_url);
    let data = bincode::serialize(input).map_err(|e| ServerError::InternalError(e.to_string()))?;
    let data_base64 = BASE64_STANDARD.encode(&data);

    let request = CreateRequest {
        encrypted_data: data_base64,
        public_key: Bytes32::zero(), // todo: reconsider this
        transition_type: transition_type.to_string(),
    };

    let client = reqwest_wasm::Client::new();
    let res = with_retry(|| async { client.post(url.clone()).json(&request).send().await })
        .await
        .map_err(|e| {
            ServerError::NetworkError(format!("Failed to query TEE server: {}", e.to_string()))
        })?;
    if !res.status().is_success() {
        return Err(ServerError::InvalidResponse(format!(
            "Failed to query TEE server: {}",
            res.status().to_string()
        )));
    }
    let res = res.json::<CreateResponse>().await.map_err(|e| {
        ServerError::DeserializationError(format!(
            "Failed to deserialize TEE server response: {}",
            e
        ))
    })?;
    Ok(res.request_id)
}

async fn query_request(base_url: &str, request_id: &str) -> Result<QueryResponse, ServerError> {
    let url = format!("{}/v1/proof/result", base_url);
    let client = reqwest_wasm::Client::new();
    let res = with_retry(|| async {
        client
            .get(url.clone())
            .query(&[("requestId", request_id)])
            .send()
            .await
    })
    .await
    .map_err(|e| {
        ServerError::NetworkError(format!("Failed to query TEE server: {}", e.to_string()))
    })?;
    if !res.status().is_success() {
        return Err(ServerError::InvalidResponse(format!(
            "Failed to query TEE server: {}",
            res.status().to_string()
        )));
    }
    let res = res.json::<QueryResponse>().await.map_err(|e| {
        ServerError::DeserializationError(format!(
            "Failed to deserialize TEE server response: {}",
            e
        ))
    })?;
    Ok(res)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRequest {
    encrypted_data: String,
    public_key: Bytes32,
    transition_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateResponse {
    request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryResponse {
    result: String,
    status: String,
}
