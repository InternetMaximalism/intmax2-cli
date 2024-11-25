use async_trait::async_trait;
use intmax2_zkp::{
    common::{
        block_builder::BlockProposal, signature::flatten::FlatG2, tx::Tx,
        witness::transfer_witness::TransferWitness,
    },
    ethereum_types::u256::U256,
};
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
pub struct FeeProof {
    pub spent_proof: ProofWithPublicInputs<F, C, D>,
    pub prev_balance_proof: ProofWithPublicInputs<F, C, D>,
    pub transfer_witness: TransferWitness,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockBuilderStatus {
    Pausing,        // not accepting tx requests
    AcceptingTxs,   // accepting  tx request
    ProposingBlock, // after constructed the block, accepting signatures for the block
}

#[async_trait(?Send)]
pub trait BlockBuilderClientInterface {
    // Get the status of the block builder
    async fn get_status(
        &self,
        block_builder_url: &str,
        is_registration_block: bool,
    ) -> Result<BlockBuilderStatus, ServerError>;

    // Send tx request to the block builder
    async fn send_tx_request(
        &self,
        block_builder_url: &str,
        is_registration_block: bool,
        pubkey: U256,
        tx: Tx,
        fee_proof: Option<FeeProof>,
    ) -> Result<(), ServerError>;

    // Query tx tree root proposal from the block builder
    async fn query_proposal(
        &self,
        block_builder_url: &str,
        is_registration_block: bool,
        pubkey: U256,
        tx: Tx,
    ) -> Result<Option<BlockProposal>, ServerError>;

    // Send signature to the block builder
    async fn post_signature(
        &self,
        block_builder_url: &str,
        is_registration_block: bool,
        pubkey: U256,
        tx: Tx,
        signature: FlatG2,
    ) -> Result<(), ServerError>;
}
