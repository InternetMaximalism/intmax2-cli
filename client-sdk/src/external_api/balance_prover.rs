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

use super::utils::query::post_request;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct BalanceProverClient {
    base_url: String,
}

impl BalanceProverClient {
    pub fn new(base_url: &str) -> Self {
        BalanceProverClient {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl BalanceProverClientInterface for BalanceProverClient {
    async fn prove_spent(
        &self,
        _key: KeySet,
        spent_witness: &SpentWitness,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let request = ProveSpentRequest {
            spent_witness: spent_witness.clone(),
        };
        let response: ProveResponse = post_request(
            &self.base_url,
            "/balance-prover/prove-spent",
            &request,
            Some(get_bearer_token()?),
        )
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
        let response: ProveResponse = post_request(
            &self.base_url,
            "/balance-prover/prove-send",
            &request,
            Some(get_bearer_token()?),
        )
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
        let response: ProveResponse = post_request(
            &self.base_url,
            "/balance-prover/prove-update",
            &request,
            Some(get_bearer_token()?),
        )
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
        let response: ProveResponse = post_request(
            &self.base_url,
            "/balance-prover/prove-receive-transfer",
            &request,
            Some(get_bearer_token()?),
        )
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
        let response: ProveResponse = post_request(
            &self.base_url,
            "/balance-prover/prove-receive-deposit",
            &request,
            Some(get_bearer_token()?),
        )
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
        let response: ProveResponse = post_request(
            &self.base_url,
            "/balance-prover/prove-single-withdrawal",
            &request,
            Some(get_bearer_token()?),
        )
        .await?;
        Ok(response.proof)
    }
}

fn get_bearer_token() -> Result<String, ServerError> {
    let token = std::env::var("BALANCE_PROVER_BEARER_TOKEN")
        .map_err(|e| ServerError::EnvError(format!("Failed to get bearer token: {}", e)))?;
    Ok(token)
}
