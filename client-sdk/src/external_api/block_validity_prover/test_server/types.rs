use intmax2_zkp::{
    circuits::validity::validity_pis::ValidityPublicInputs,
    common::{
        trees::{
            block_hash_tree::BlockHashMerkleProof, deposit_tree::DepositMerkleProof,
            sender_tree::SenderLeaf,
        },
        witness::update_witness::UpdateWitness,
    },
    ethereum_types::{bytes32::Bytes32, u256::U256},
};
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use serde::{Deserialize, Serialize};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockNumberResponse {
    pub block_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAccountIdQuery {
    pub pubkey: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAccountIdResponse {
    pub account_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUpdateWitnessQuery {
    pub pubkey: U256,
    pub root_block_number: u32,
    pub leaf_block_number: u32,
    pub is_prev_account_tree: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUpdateWitnessResponse {
    pub update_witness: UpdateWitness<F, C, D>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDepositIndexAndBlockNumberQuery {
    pub deposit_hash: Bytes32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDepositIndexAndBlockNumberResponse {
    pub deposit_index_and_block_number: Option<(u32, u32)>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockNumberByTxTreeRootQuery {
    pub tx_tree_root: Bytes32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockNumberByTxTreeRootResponse {
    pub block_number: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetValidityPisQuery {
    pub block_number: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetValidityPisResponse {
    pub validity_pis: Option<ValidityPublicInputs>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSenderLeavesQuery {
    pub block_number: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSenderLeavesResponse {
    pub sender_leaves: Option<Vec<SenderLeaf>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockMerkleProofQuery {
    pub root_block_number: u32,
    pub leaf_block_number: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockMerkleProofResponse {
    pub block_merkle_proof: BlockHashMerkleProof,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDepositMerkleProofQuery {
    pub block_number: u32,
    pub deposit_index: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDepositMerkleProofResponse {
    pub deposit_merkle_proof: DepositMerkleProof,
}
