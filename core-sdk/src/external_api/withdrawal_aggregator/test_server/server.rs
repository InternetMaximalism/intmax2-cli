use async_trait::async_trait;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use reqwest_wasm::Client;

use crate::external_api::{
    common::error::ServerError,
    withdrawal_aggregator::interface::{WithdrawalAggregatorInterface, WithdrawalInfo},
};
use crate::external_api::{
    utils::retry::with_retry,
    withdrawal_aggregator::{interface::Fee, test_server::types::RequestWithdrawalRequest},
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct TestWithdrawalAggregator {
    base_url: String,
    client: Client,
}

impl TestWithdrawalAggregator {
    pub fn new(base_url: String) -> Self {
        TestWithdrawalAggregator {
            base_url,
            client: Client::new(),
        }
    }

    async fn post_request<T: serde::Serialize, U: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
        body: &T,
    ) -> Result<U, ServerError> {
        let url = format!("{}{}", self.base_url, endpoint);
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

    async fn get_request<T: serde::de::DeserializeOwned>(
        &self,
        endpoint: &str,
    ) -> Result<T, ServerError> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ServerError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json::<T>()
                .await
                .map_err(|e| ServerError::DeserializationError(e.to_string()))
        } else {
            Err(ServerError::ServerError(response.status().to_string()))
        }
    }
}

#[async_trait(?Send)]
impl WithdrawalAggregatorInterface for TestWithdrawalAggregator {
    async fn fee(&self) -> Result<Vec<Fee>, ServerError> {
        self.get_request::<Vec<Fee>>("/withdrawal-aggregator/fee")
            .await
    }

    async fn request_withdrawal(
        &self,
        single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError> {
        let request = RequestWithdrawalRequest {
            single_withdrawal_proof: single_withdrawal_proof.clone(),
        };
        self.post_request::<_, ()>("/withdrawal-aggregator/request-withdrawal", &request)
            .await
    }

    async fn get_withdrawal_info(&self) -> Result<Vec<WithdrawalInfo>, ServerError> {
        Ok(vec![])
    }
}
