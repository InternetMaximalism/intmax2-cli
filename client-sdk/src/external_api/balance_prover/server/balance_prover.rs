use async_trait::async_trait;
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

use crate::external_api::{
    balance_prover::{
        interface::BalanceProverInterface,
        test_server::types::{
            ProveReceiveDepositRequest, ProveReceiveTransferRequest, ProveSendRequest,
            ProveSingleWithdrawalRequest, ProveSpentRequest, ProveUpdateRequest,
        },
    },
    common::error::ServerError,
};

use super::query::request_and_fetch_proof;

pub struct BalanceProver {
    pub server_base_url: String,
}

impl BalanceProver {
    pub fn new(server_base_url: String) -> Self {
        Self { server_base_url }
    }
}

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[async_trait(?Send)]
impl BalanceProverInterface for BalanceProver {
    async fn prove_spent(
        &self,
        key: KeySet,
        spent_witness: &SpentWitness,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let input = ProveSpentRequest {
            spent_witness: spent_witness.clone(),
        };
        request_and_fetch_proof(&self.server_base_url, "spent", &input, key).await
    }

    async fn prove_send(
        &self,
        key: KeySet,
        pubkey: U256,
        tx_witnes: &TxWitness,
        update_witness: &UpdateWitness<F, C, D>,
        spent_proof: &ProofWithPublicInputs<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let input = ProveSendRequest {
            pubkey,
            tx_witnes: tx_witnes.clone(),
            update_witness: update_witness.clone(),
            spent_proof: spent_proof.clone(),
            prev_proof: prev_proof.clone(),
        };
        request_and_fetch_proof(&self.server_base_url, "send", &input, key).await
    }

    async fn prove_update(
        &self,
        key: KeySet,
        pubkey: U256,
        update_witness: &UpdateWitness<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let input = ProveUpdateRequest {
            pubkey,
            update_witness: update_witness.clone(),
            prev_proof: prev_proof.clone(),
        };
        request_and_fetch_proof(&self.server_base_url, "update", &input, key).await
    }

    async fn prove_receive_transfer(
        &self,
        key: KeySet,
        pubkey: U256,
        receive_transfer_witness: &ReceiveTransferWitness<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let input = ProveReceiveTransferRequest {
            pubkey,
            receive_transfer_witness: receive_transfer_witness.clone(),
            prev_proof: prev_proof.clone(),
        };
        request_and_fetch_proof(&self.server_base_url, "transfer", &input, key).await
    }

    async fn prove_receive_deposit(
        &self,
        key: KeySet,
        pubkey: U256,
        receive_deposit_witness: &ReceiveDepositWitness,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let input = ProveReceiveDepositRequest {
            pubkey,
            receive_deposit_witness: receive_deposit_witness.clone(),
            prev_proof: prev_proof.clone(),
        };
        request_and_fetch_proof(&self.server_base_url, "deposit", &input, key).await
    }

    async fn prove_single_withdrawal(
        &self,
        key: KeySet,
        withdrawal_witness: &WithdrawalWitness<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError> {
        let input = ProveSingleWithdrawalRequest {
            withdrawal_witness: withdrawal_witness.clone(),
        };
        request_and_fetch_proof(&self.server_base_url, "withdrawal", &input, key).await
    }
}
