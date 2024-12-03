use async_trait::async_trait;
use intmax2_interfaces::api::{
    error::ServerError,
    withdrawal_server::{
        interface::{Fee, WithdrawalInfo, WithdrawalServerClientInterface},
        types::{
            GetFeeResponse, GetWithdrawalInfoByRecipientRequest, GetWithdrawalInfoRequest,
            GetWithdrawalInfoResponse, RequestWithdrawalRequest,
        },
    },
};
use intmax2_zkp::{
    common::signature::{flatten::FlatG2, key_set::KeySet},
    ethereum_types::{address::Address, u256::U256},
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};

use super::utils::query::{get_request, post_request};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct WithdrawalServerClient {
    base_url: String,
}

impl WithdrawalServerClient {
    pub fn new(base_url: &str) -> Self {
        WithdrawalServerClient {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl WithdrawalServerClientInterface for WithdrawalServerClient {
    async fn fee(&self) -> Result<Vec<Fee>, ServerError> {
        let response: GetFeeResponse =
            get_request::<(), _>(&self.base_url, "/withdrawal-server/fee", None, None).await?;
        Ok(response.fees)
    }

    async fn request_withdrawal(
        &self,
        pubkey: U256,
        single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError> {
        let request = RequestWithdrawalRequest {
            pubkey,
            single_withdrawal_proof: single_withdrawal_proof.clone(),
        };
        post_request::<_, ()>(
            &self.base_url,
            "/withdrawal-server/request-withdrawal",
            &request,
            None,
        )
        .await
    }

    async fn get_withdrawal_info(&self, key: KeySet) -> Result<Vec<WithdrawalInfo>, ServerError> {
        let pubkey = key.pubkey;
        let signature = FlatG2::default(); // todo: get signature from key
        let query = GetWithdrawalInfoRequest { pubkey, signature };
        let response: GetWithdrawalInfoResponse = get_request(
            &self.base_url,
            "/withdrawal-server/get-withdrawal-info",
            Some(query),
            None,
        )
        .await?;
        Ok(response.withdrawal_info)
    }

    async fn get_withdrawal_info_by_recipient(
        &self,
        recipient: Address,
    ) -> Result<Vec<WithdrawalInfo>, ServerError> {
        let query = GetWithdrawalInfoByRecipientRequest { recipient };
        let response: GetWithdrawalInfoResponse = get_request(
            &self.base_url,
            "/withdrawal-server/get-withdrawal-info-by-recipient",
            Some(query),
            None,
        )
        .await?;
        Ok(response.withdrawal_info)
    }
}
