use intmax2_zkp::{
    common::witness::update_witness::UpdateWitness, ethereum_types::bytes32::Bytes32,
};
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use serde_json::json;

use crate::{
    external_api::{
        common::{error::ServerError, response::ServerCommonResponse},
        utils::{
            handler::{handle_response, ResponseType},
            retry::with_retry,
        },
    },
    utils::circuit_verifiers::CircuitVerifiers,
};

use super::conversion::ConvertedUpdateWitness;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub async fn get_update_witness(
    server_base_url: &str,
    pubkey: Bytes32,
    root_block_number: u32,
    leaf_block_number: u32,
    is_prev_account_tree: bool,
) -> Result<UpdateWitness<F, C, D>, ServerError> {
    let url = format!("{}/balance-update-witness", server_base_url,);
    let request = json!({
        "user": pubkey,
        "currentBlockNumber": root_block_number,
        "targetBlockNumber": leaf_block_number,
        "isPrevAccountTree": is_prev_account_tree,
    });
    let response = with_retry(|| async {
        reqwest_wasm::Client::new()
            .post(&url)
            .json(&request)
            .send()
            .await
    })
    .await
    .map_err(|e| ServerError::NetworkError(format!("Failed to get update witness: {}", e)))?;
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let info: ServerCommonResponse<ConvertedUpdateWitness> =
                response.json().await.map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to parse update witness response: {}",
                        e
                    ))
                })?;

            let verifiers = CircuitVerifiers::load();
            let update_witness = info
                .data
                .to_update_witness(&verifiers.get_validity_vd())
                .map_err(|e| {
                    ServerError::DeserializationError(format!(
                        "Failed to convert update witness: {}",
                        e
                    ))
                })?;
            Ok(update_witness)
        }
        _ => {
            return Err(ServerError::InternalError(
                "Failed to get update witness".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use intmax2_zkp::ethereum_types::bytes32::Bytes32;

    use crate::{
        external_api::block_validity_prover::server::update_witness::get_update_witness,
        utils::init_logger::init_logger,
    };

    #[tokio::test]
    async fn test_get_update_witness() -> anyhow::Result<()> {
        init_logger();
        let server_base_url = "http://localhost:4000/v1/blockvalidity";
        let pubkey = Bytes32::default();
        let root_block_number = 1;
        let leaf_block_number = 0;
        let is_prev_account_tree = false;
        let update_witness = get_update_witness(
            server_base_url,
            pubkey,
            root_block_number,
            leaf_block_number,
            is_prev_account_tree,
        )
        .await?;
        log::info!("update_witness: {:?}", update_witness);
        Ok(())
    }
}
