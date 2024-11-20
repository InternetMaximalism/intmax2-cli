use async_trait::async_trait;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};

use crate::external_api::common::error::ServerError;

use super::interface::{Fee, WithdrawalAggregatorInterface, WithdrawalInfo};

pub mod fee;

pub struct WithdrawalAggregatorServer {
    pub server_base_url: String,
}

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

impl WithdrawalAggregatorServer {
    pub fn new(server_base_url: String) -> Self {
        Self { server_base_url }
    }
}

#[async_trait(?Send)]
impl WithdrawalAggregatorInterface for WithdrawalAggregatorServer {
    async fn fee(&self) -> Result<Vec<Fee>, ServerError> {
        todo!()
    }

    async fn request_withdrawal(
        &self,
        _single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError> {
        todo!()
    }

    async fn get_withdrawal_info(&self) -> Result<Vec<WithdrawalInfo>, ServerError> {
        todo!()
    }
}
