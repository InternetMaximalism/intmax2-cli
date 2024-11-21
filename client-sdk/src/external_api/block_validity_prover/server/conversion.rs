use intmax2_zkp::{
    common::{
        trees::{account_tree::AccountMembershipProof, block_hash_tree::BlockHashMerkleProof},
        witness::update_witness::UpdateWitness,
    },
    ethereum_types::bytes32::Bytes32,
    utils::trees::indexed_merkle_tree::{leaf::IndexedMerkleLeaf, IndexedMerkleProof},
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{circuit_data::VerifierCircuitData, config::PoseidonGoldilocksConfig},
};

use serde::{Deserialize, Serialize};

use crate::external_api::utils::encode::decode_plonky2_proof;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertedUpdateWitness {
    pub is_prev_account_tree: bool,
    pub validity_proof: String,
    pub block_merkle_proof: BlockHashMerkleProof,
    pub account_membership_proof: ConvertedMembershipProof,
}

impl ConvertedUpdateWitness {
    pub fn to_update_witness(
        &self,
        validity_vd: &VerifierCircuitData<F, C, D>,
    ) -> anyhow::Result<UpdateWitness<F, C, D>> {
        let validity_proof =
            decode_plonky2_proof(&self.validity_proof, validity_vd).map_err(|e| {
                anyhow::anyhow!(format!(
                    "Failed to decode validity proof: {}",
                    e.to_string()
                ))
            })?;
        Ok(UpdateWitness {
            is_prev_account_tree: self.is_prev_account_tree,
            validity_proof,
            block_merkle_proof: self.block_merkle_proof.clone(),
            account_membership_proof: self.account_membership_proof.clone().into(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertedMembershipProof {
    pub is_included: bool,
    pub leaf_proof: IndexedMerkleProof,
    pub leaf_index: u64,
    pub leaf: ConvertedIndexedMerkleLeaf,
}

impl From<ConvertedMembershipProof> for AccountMembershipProof {
    fn from(proof: ConvertedMembershipProof) -> Self {
        AccountMembershipProof {
            is_included: proof.is_included,
            leaf_proof: proof.leaf_proof,
            leaf_index: proof.leaf_index,
            leaf: proof.leaf.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertedIndexedMerkleLeaf {
    pub next_index: u64,
    pub key: Bytes32,
    pub next_key: Bytes32,
    pub value: u64,
}

impl From<ConvertedIndexedMerkleLeaf> for IndexedMerkleLeaf {
    fn from(leaf: ConvertedIndexedMerkleLeaf) -> Self {
        IndexedMerkleLeaf {
            next_index: leaf.next_index,
            key: leaf.key.into(),
            next_key: leaf.next_key.into(),
            value: leaf.value,
        }
    }
}
