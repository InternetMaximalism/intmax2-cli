use intmax2_zkp::{
    circuits::balance::balance_pis::BalancePublicInputs, ethereum_types::bytes32::Bytes32,
    utils::poseidon_hash_out::PoseidonHashOut,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{
        circuit_data::VerifierCircuitData, config::PoseidonGoldilocksConfig,
        proof::ProofWithPublicInputs,
    },
};
use serde::{Deserialize, Serialize};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

use crate::external_api::{
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        encode::decode_plonky2_proof,
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

#[derive(Serialize, Deserialize)]
pub struct GetBalanceProofData {
    pub proof: String,
}

pub async fn get_balance_proof(
    balance_circuit_vd: &VerifierCircuitData<F, C, D>,
    server_base_url: &str,
    pubkey: Bytes32,
    block_number: u32,
    private_commitment: PoseidonHashOut,
) -> Result<Option<ProofWithPublicInputs<F, C, D>>, ServerError> {
    let url = format!(
        "{}/balance-proof?user={}&blockNumber={}&privateCommitment={}",
        server_base_url, pubkey, block_number, private_commitment
    );
    let response = with_retry(|| async { reqwest_wasm::Client::new().get(&url).send().await })
        .await
        .map_err(|e| {
            ServerError::NetworkError(format!("Failed to get balance proof from server: {}", e))
        })?;

    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let response: ServerCommonResponse<GetBalanceProofData> = response
                .json()
                .await
                .map_err(|e| ServerError::DeserializationError(e.to_string()))?;
            if !response.success {
                return Err(ServerError::InvalidResponse(
                    "Failed to get balance proof".to_string(),
                ));
            }
            let proof = decode_plonky2_proof(&response.data.proof, balance_circuit_vd)
                .map_err(|e| ServerError::ProofDecodeError(e.to_string()))?;
            balance_circuit_vd.verify(proof.clone()).map_err(|e| {
                ServerError::ProofVerificationError(format!("Failed to verify proof: {}", e))
            })?;
            let balance_pis = BalancePublicInputs::from_pis(&proof.public_inputs);
            if balance_pis.pubkey != pubkey.into() {
                return Err(ServerError::InvalidResponse(
                    "Invalid balance proof pubkey".to_string(),
                ));
            }
            if balance_pis.public_state.block_number != block_number {
                return Err(ServerError::InvalidResponse(
                    "Invalid balance proof block number".to_string(),
                ));
            }
            if balance_pis.private_commitment != private_commitment {
                return Err(ServerError::InvalidResponse(
                    "Invalid balance proof private commitment".to_string(),
                ));
            }
            return Ok(Some(proof));
        }
        ResponseType::NotFound(error_detail) => {
            log::info!("Balance proof not found: {:?}", error_detail);
            return Ok(None);
        }
        ResponseType::ServerError(error_detail) => {
            log::error!("Server error: {:?}", error_detail);
            return Err(ServerError::ServerError(error_detail.message));
        }
        ResponseType::UnknownError(error) => {
            log::error!("Unknown error: {:?}", error);
            return Err(ServerError::ServerError(error));
        }
    }
}

#[cfg(test)]
mod tests {
    use intmax2_zkp::{ethereum_types::u256::U256, utils::poseidon_hash_out::PoseidonHashOut};

    use crate::utils::{circuit_verifiers::CircuitVerifiers, init_logger::init_logger};

    #[tokio::test]
    #[ignore]
    async fn test_get_balance_proof() -> anyhow::Result<()> {
        init_logger();

        let mut rng = rand::thread_rng();
        let circuit_data = CircuitVerifiers::load();
        let balance_circuit_vd = circuit_data.get_balance_vd();
        let mock_url = "http://localhost:4000/v1/backups";
        let pubkey = U256::rand(&mut rng);
        let block_number = 1;
        let private_commitment = PoseidonHashOut::rand(&mut rng);

        let _proof = super::get_balance_proof(
            &balance_circuit_vd,
            mock_url,
            pubkey.into(),
            block_number,
            private_commitment,
        )
        .await?;
        Ok(())
    }
}
