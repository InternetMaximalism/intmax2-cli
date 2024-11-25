use intmax2_zkp::{common::signature::flatten::FlatG2, ethereum_types::u256::U256};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use serde::{Deserialize, Serialize};

use super::interface::{Fee, WithdrawalInfo};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestWithdrawalRequest {
    pub single_withdrawal_proof: ProofWithPublicInputs<F, C, D>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFeeResponse {
    pub fees: Vec<Fee>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWithdrawalInfoRequest {
    pub pubkey: U256,
    pub signature: FlatG2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWithdrawalInfoReqponse {
    pub withdrawal_info: Vec<WithdrawalInfo>,
}
