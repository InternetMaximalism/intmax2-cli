use intmax2_zkp::ethereum_types::bytes32::Bytes32;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{
        circuit_data::VerifierCircuitData, config::PoseidonGoldilocksConfig,
        proof::ProofWithPublicInputs,
    },
};
use reqwest_wasm::Client;
use serde::{Deserialize, Serialize};

use crate::external_api::{
    common::error::ServerError,
    utils::{
        encode::encode_plonky2_proof,
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Serialize, Deserialize)]
pub struct SaveBalanceProofRequest {
    pub user: Bytes32,
    pub proof: String,
}

pub async fn save_balance_proof(
    balance_circuit_vd: &VerifierCircuitData<F, C, D>,
    server_base_url: &str,
    pubkey: Bytes32,
    proof: &ProofWithPublicInputs<F, C, D>,
) -> Result<(), ServerError> {
    let proof_encoded = encode_plonky2_proof(proof.clone(), balance_circuit_vd);
    let request = SaveBalanceProofRequest {
        user: pubkey,
        proof: proof_encoded,
    };

    let url = format!("{}/balance-proof", server_base_url,);
    let response =
        with_retry(|| async { Client::new().post(url.clone()).json(&request).send().await })
            .await
            .map_err(|e| {
                ServerError::NetworkError(format!("Failed to save balance proof to server: {}", e))
            })?;
    match handle_response(response).await? {
        ResponseType::Success(_) => Ok(()),
        ResponseType::ServerError(error) => Err(ServerError::ServerError(error.message)),
        _ => Err(ServerError::UnknownError("Unknown error".to_string())),
    }
}
