use intmax2_zkp::{
    common::{block_builder::BlockProposal, signature::flatten::FlatG2, tx::Tx},
    ethereum_types::u256::U256,
};
use serde::{Deserialize, Serialize};

use super::interface::{BlockBuilderStatus, FeeProof};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxRequestRequest {
    pub is_registration_block: bool,
    pub pubkey: U256,
    pub tx: Tx,
    pub fee_proof: Option<FeeProof>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryProposalRequest {
    pub is_registration_block: bool,
    pub pubkey: U256,
    pub tx: Tx,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryProposalResponse {
    pub block_proposal: Option<BlockProposal>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostSignatureRequest {
    pub is_registration_block: bool,
    pub pubkey: U256,
    pub tx: Tx,
    pub signature: FlatG2,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockBuilderStatusQuery {
    pub is_registration_block: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockBuilderStatusResponse {
    pub status: BlockBuilderStatus,
}
