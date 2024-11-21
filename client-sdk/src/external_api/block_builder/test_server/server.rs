use async_trait::async_trait;
use intmax2_zkp::{
    common::{block_builder::BlockProposal, signature::flatten::FlatG2, tx::Tx},
    ethereum_types::u256::U256,
};
use reqwest_wasm::Client;

use crate::external_api::{block_builder::{
    interface::{BlockBuilderInterface, FeeProof},
    test_server::types::{
        PostSignatureRequest, QueryProposalRequest, QueryProposalResponse, TxRequestRequest,
    },
}, utils::retry::with_retry};
use crate::external_api::common::error::ServerError;

#[derive(Debug, Clone)]
pub struct TestBlockBuilder {
    client: Client,
}

impl TestBlockBuilder {
    pub fn new() -> Self {
        TestBlockBuilder {
            client: Client::new(),
        }
    }

    async fn post_request<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        base_url: &str,
        endpoint: &str,
        body: &T,
    ) -> Result<U, ServerError> {
        let url = format!("{}{}", base_url, endpoint);
        let response = with_retry(|| async { self.client.post(&url).json(body).send().await })
            .await
            .map_err(|e| ServerError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json::<U>()
                .await
                .map_err(|e| ServerError::DeserializationError(e.to_string()))
        } else {
            Err(ServerError::ServerError(response.status().to_string()))
        }
    }
}

#[async_trait(?Send)]
impl BlockBuilderInterface for TestBlockBuilder {
    async fn send_tx_request(
        &self,
        block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
        _fee_proof: Option<FeeProof>,
    ) -> Result<(), ServerError> {
        let request = TxRequestRequest { pubkey, tx };
        self.post_request::<_, ()>(block_builder_url, "/block-builder/tx-request", &request)
            .await
    }

    async fn query_proposal(
        &self,
        block_builder_url: &str,
        pubkey: U256,
        tx: Tx,
    ) -> Result<Option<BlockProposal>, ServerError> {
        let request = QueryProposalRequest { pubkey, tx };
        let response: QueryProposalResponse = self
            .post_request(block_builder_url, "/block-builder/query-proposal", &request)
            .await?;
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
        self.post_request::<_, ()>(block_builder_url, "/block-builder/post-signature", &request)
            .await
    }
}
