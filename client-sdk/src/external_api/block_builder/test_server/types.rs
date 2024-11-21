use intmax2_zkp::{
    common::{block_builder::BlockProposal, signature::flatten::FlatG2, tx::Tx},
    ethereum_types::u256::U256,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxRequestRequest {
    pub pubkey: U256,
    pub tx: Tx,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryProposalRequest {
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
    pub pubkey: U256,
    pub tx: Tx,
    pub signature: FlatG2,
}
