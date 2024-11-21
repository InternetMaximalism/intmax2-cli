use async_trait::async_trait;
use intmax2_zkp::{
    common::witness::{
        receive_deposit_witness::ReceiveDepositWitness,
        receive_transfer_witness::ReceiveTransferWitness, spent_witness::SpentWitness,
        tx_witness::TxWitness, update_witness::UpdateWitness,
        withdrawal_witness::WithdrawalWitness,
    },
    ethereum_types::u256::U256,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};

use crate::api::error::ServerError;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[async_trait(?Send)]
pub trait BalanceProverInterface {
    async fn prove_spent(
        &self,
        spent_witness: &SpentWitness,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError>;

    async fn prove_send(
        &self,
        pubkey: U256,
        tx_witnes: &TxWitness,
        update_witness: &UpdateWitness<F, C, D>,
        spent_proof: &ProofWithPublicInputs<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError>;

    async fn prove_update(
        &self,
        pubkey: U256,
        update_witness: &UpdateWitness<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError>;

    async fn prove_receive_transfer(
        &self,
        pubkey: U256,
        receive_transfer_witness: &ReceiveTransferWitness<F, C, D>,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError>;

    async fn prove_receive_deposit(
        &self,
        pubkey: U256,
        receive_deposit_witness: &ReceiveDepositWitness,
        prev_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError>;

    async fn prove_single_withdrawal(
        &self,
        withdrawal_witness: &WithdrawalWitness<F, C, D>,
    ) -> Result<ProofWithPublicInputs<F, C, D>, ServerError>;
}
