use async_trait::async_trait;
use intmax2_zkp::{
    common::{block_builder::BlockProposal, signature::flatten::FlatG2, tx::Tx},
    ethereum_types::u256::U256,
};

use crate::external_api::{
    block_builder::interface::{BlockBuilderInterface, FeeProof},
    common::error::ServerError,
};

use super::{query::query_proposal, signature::post_signature, tx_request::send_tx_request};

pub struct BlockBuilder;

impl BlockBuilder {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait(?Send)]
impl BlockBuilderInterface for BlockBuilder {
    async fn send_tx_request(
        &self,
        block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
        _fee_proof: Option<FeeProof>,
    ) -> Result<(), ServerError> {
        send_tx_request(block_builder_url, pubkey.into(), tx).await?;
        Ok(())
    }

    async fn query_proposal(
        &self,
        block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
    ) -> Result<Option<BlockProposal>, ServerError> {
        let proposal = query_proposal(block_builder_url, pubkey.into(), tx).await?;
        Ok(proposal)
    }

    async fn post_signature(
        &self,
        block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
        signature: FlatG2,
    ) -> Result<(), ServerError> {
        post_signature(block_builder_url, pubkey.into(), tx, signature, None).await?;
        Ok(())
    }
}
