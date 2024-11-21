use async_trait::async_trait;
use intmax2_interfaces::api::{
    block_builder::{
        interface::{BlockBuilderClientInterface, FeeProof},
        types::{
            PostSignatureRequest, QueryProposalRequest, QueryProposalResponse, TxRequestRequest,
        },
    },
    error::ServerError,
};
use intmax2_zkp::{
    common::{block_builder::BlockProposal, signature::flatten::FlatG2, tx::Tx},
    ethereum_types::u256::U256,
};

use super::utils::query::post_request;

#[derive(Debug, Clone)]
pub struct BlockBuilderClient;

impl BlockBuilderClient {
    pub fn new() -> Self {
        BlockBuilderClient
    }
}

#[async_trait(?Send)]
impl BlockBuilderClientInterface for BlockBuilderClient {
    async fn send_tx_request(
        &self,
        block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
        fee_proof: Option<FeeProof>,
    ) -> Result<(), ServerError> {
        let request = TxRequestRequest {
            pubkey,
            tx,
            fee_proof,
        };
        post_request::<_, ()>(block_builder_url, "/block-builder/tx-request", &request).await
    }

    async fn query_proposal(
        &self,
        block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
    ) -> Result<Option<BlockProposal>, ServerError> {
        let request = QueryProposalRequest { pubkey, tx };
        let response: QueryProposalResponse =
            post_request(block_builder_url, "/block-builder/query-proposal", &request).await?;
        Ok(response.block_proposal)
    }

    async fn post_signature(
        &self,
        block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
        signature: FlatG2,
    ) -> Result<(), ServerError> {
        let request = PostSignatureRequest {
            pubkey,
            tx,
            signature,
        };
        post_request::<_, ()>(block_builder_url, "/block-builder/post-signature", &request).await
    }
}
