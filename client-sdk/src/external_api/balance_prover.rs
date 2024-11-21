use async_trait::async_trait;
use intmax2_interfaces::api::{
    balance_prover::{
        interface::BalanceProverClientInterface,
        types::{
            ProveReceiveDepositRequest, ProveReceiveTransferRequest, ProveResponse,
            ProveSendRequest, ProveSingleWithdrawalRequest, ProveSpentRequest, ProveUpdateRequest,
        },
    },
    error::ServerError,
};
use intmax2_zkp::{
    common::{
        signature::key_set::KeySet,
        witness::{
            receive_deposit_witness::ReceiveDepositWitness,
            receive_transfer_witness::ReceiveTransferWitness, spent_witness::SpentWitness,
            tx_witness::TxWitness, update_witness::UpdateWitness,
            withdrawal_witness::WithdrawalWitness,
        },
    },
    ethereum_types::u256::U256,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use reqwest_wasm::Client;

use super::utils::retry::with_retry;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct TestBalanceProver {
    base_url: String,
    client: Client,
}

impl TestBalanceProver {
    pub fn new(base_url: String) -> Self {
        TestBalanceProver {
            base_url,
            client: Client::new(),
        }
    }

    async fn post_request<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        body: &T,
    ) -> Result<U, ServerError> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = with_retry(|| async { self.client.post(&url).json(body).send().await })
            .await
            .map_err(|e| ServerError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json::<U>()
                .await
                .map_err(|e| ServerError::DeserializationError(e.to_string()))
        } else {
            Err(ServerError::ServerError(response.status().to_string()))
        }
    }
}

#[async_trait(?Send)]
impl BalanceProverClientInterface for TestBalanceProver {
    async fn prove_spent(
        &self,
        _key: KeySet,
        spent_witness: &SpentWitness,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let request = ProveSpentRequest {
            spent_witness: spent_witness.clone(),
        };
        let response: ProveResponse = self
            .post_request("/balance-prover/prove-spent", &request)
            .await?;
        Ok(response.proof)
    }

    async fn prove_send(
        &self,
        _key: KeySet,
        pubkey: U256,
        tx_witnes: &TxWitness,
        update_witness: &UpdateWitness<F, C, D>,
        spent_proof: &ProofWithPublicInputs<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let request = ProveSendRequest {
            pubkey,
            tx_witnes: tx_witnes.clone(),
            update_witness: update_witness.clone(),
            spent_proof: spent_proof.clone(),
            prev_proof: prev_proof.clone(),
        };
        let response: ProveResponse = self
            .post_request("/balance-prover/prove-send", &request)
            .await?;
        Ok(response.proof)
    }

    async fn prove_update(
        &self,
        _key: KeySet,
        pubkey: U256,
        update_witness: &UpdateWitness<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let request = ProveUpdateRequest {
            pubkey,
            update_witness: update_witness.clone(),
            prev_proof: prev_proof.clone(),
        };
        let response: ProveResponse = self
            .post_request("/balance-prover/prove-update", &request)
            .await?;
        Ok(response.proof)
    }

    async fn prove_receive_transfer(
        &self,
        _key: KeySet,
        pubkey: U256,
        receive_transfer_witness: &ReceiveTransferWitness<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let request = ProveReceiveTransferRequest {
            pubkey,
            receive_transfer_witness: receive_transfer_witness.clone(),
            prev_proof: prev_proof.clone(),
        };
        let response: ProveResponse = self
            .post_request("/balance-prover/prove-receive-transfer", &request)
            .await?;
        Ok(response.proof)
    }

    async fn prove_receive_deposit(
        &self,
        _key: KeySet,
        pubkey: U256,
        receive_deposit_witness: &ReceiveDepositWitness,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let request = ProveReceiveDepositRequest {
            pubkey,
            receive_deposit_witness: receive_deposit_witness.clone(),
            prev_proof: prev_proof.clone(),
        };
        let response: ProveResponse = self
            .post_request("/balance-prover/prove-receive-deposit", &request)
            .await?;
        Ok(response.proof)
    }

    async fn prove_single_withdrawal(
        &self,
        _key: KeySet,
        withdrawal_witness: &WithdrawalWitness<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let request = ProveSingleWithdrawalRequest {
            withdrawal_witness: withdrawal_witness.clone(),
        };
        let response: ProveResponse = self
            .post_request("/balance-prover/prove-single-withdrawal", &request)
            .await?;
        Ok(response.proof)
    }
}
