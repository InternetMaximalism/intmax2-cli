use async_trait::async_trait;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use serde::{Deserialize, Serialize};

use crate::api::error::ServerError;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fee {
    pub native_fee: u64,
    pub erc20_fee: u64,
    pub erc721_fee: u64,
    pub erc1155_fee: u64,
}

#[async_trait(?Send)]
pub trait WithdrawalAggregatorInterface {
    async fn fee(&self) -> Result<Fee, ServerError>;

    async fn request_withdrawal(
        &self,
        single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError>;
}
