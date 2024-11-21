use async_trait::async_trait;
use intmax2_zkp::{
    ethereum_types::u256::U256, mock::data::meta_data::MetaData,
    utils::poseidon_hash_out::PoseidonHashOut,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use reqwest_wasm::Client;

use crate::external_api::{
    common::error::ServerError, store_vault_server::interface::StoreVaultInterface,
};
use crate::external_api::{store_vault_server::test_server::types::*, utils::retry::with_retry};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct TestStoreVaultServer {
    base_url: String,
    client: Client,
}

impl TestStoreVaultServer {
    pub fn new(base_url: String) -> Self {
        TestStoreVaultServer {
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

    async fn get_request<T, Q>(&self, endpoint: &str, query: Option<Q>) -> Result<T, ServerError>
    where
        T: serde::de::DeserializeOwned,
        Q: serde::Serialize,
    {
        let url = format!("{}{}", self.base_url, endpoint);

        let response = if let Some(params) = query {
            with_retry(|| async { self.client.get(&url).query(&params).send().await }).await
        } else {
            with_retry(|| async { self.client.get(&url).send().await }).await
        }
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
impl StoreVaultInterface for TestStoreVaultServer {
    async fn save_balance_proof(
        &self,
        pubkey: U256,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError> {
        let request = SaveBalanceProofRequest {
            pubkey,
            balance_proof: proof,
        };
        self.post_request::<_, ()>("/store-vault-server/save-balance-proof", &request)
            .await
    }

    async fn get_balance_proof(
        &self,
        pubkey: U256,
        block_number: u32,
        private_commitment: PoseidonHashOut,
    ) -> Result<Option<ProofWithPublicInputs<F, C, D>>, ServerError> {
        let query = GetBalanceProofQuery {
            pubkey,
            block_number,
            private_commitment,
        };
        let response: GetBalanceProofResponse = self
            .get_request("/store-vault-server/get-balance-proof", Some(query))
            .await?;
        Ok(response.balance_proof)
    }

    async fn save_deposit_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        let request = SaveDataRequest {
            pubkey,
            data: encrypted_data,
        };
        self.post_request::<_, ()>("/store-vault-server/deposit/save", &request)
            .await
    }

    async fn get_deposit_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataAllAfterQuery { pubkey, timestamp };
        let response: GetDataAllAfterResponse = self
            .get_request("/store-vault-server/deposit/get-all-after", Some(query))
            .await?;
        Ok(response.data)
    }

    async fn get_deposit_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataQuery {
            uuid: uuid.to_string(),
        };
        let response: GetDataResponse = self
            .get_request("/store-vault-server/deposit/get", Some(query))
            .await?;
        Ok(response.data)
    }

    async fn save_withdrawal_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        let request = SaveDataRequest {
            pubkey,
            data: encrypted_data,
        };
        self.post_request::<_, ()>("/store-vault-server/withdrawal/save", &request)
            .await
    }

    async fn get_withdrawal_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataAllAfterQuery { pubkey, timestamp };
        let response: GetDataAllAfterResponse = self
            .get_request("/store-vault-server/withdrawal/get-all-after", Some(query))
            .await?;
        Ok(response.data)
    }

    async fn get_withdrawal_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataQuery {
            uuid: uuid.to_string(),
        };
        let response: GetDataResponse = self
            .get_request("/store-vault-server/withdrawal/get", Some(query))
            .await?;
        Ok(response.data)
    }

    async fn save_transfer_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        let request = SaveDataRequest {
            pubkey,
            data: encrypted_data,
        };
        self.post_request::<_, ()>("/store-vault-server/transfer/save", &request)
            .await
    }

    async fn get_transfer_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataAllAfterQuery { pubkey, timestamp };
        let response: GetDataAllAfterResponse = self
            .get_request("/store-vault-server/transfer/get-all-after", Some(query))
            .await?;
        Ok(response.data)
    }

    async fn get_transfer_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataQuery {
            uuid: uuid.to_string(),
        };
        let response: GetDataResponse = self
            .get_request("/store-vault-server/transfer/get", Some(query))
            .await?;
        Ok(response.data)
    }

    async fn save_tx_data(&self, pubkey: U256, encrypted_data: Vec<u8>) -> Result<(), ServerError> {
        let request = SaveDataRequest {
            pubkey,
            data: encrypted_data,
        };
        self.post_request::<_, ()>("/store-vault-server/tx/save", &request)
            .await
    }

    async fn get_tx_data(&self, uuid: &str) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataQuery {
            uuid: uuid.to_string(),
        };
        let response: GetDataResponse = self
            .get_request("/store-vault-server/tx/get", Some(query))
            .await?;
        Ok(response.data)
    }

    async fn get_tx_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataAllAfterQuery { pubkey, timestamp };
        let response: GetDataAllAfterResponse = self
            .get_request("/store-vault-server/tx/get-all-after", Some(query))
            .await?;
        Ok(response.data)
    }

    async fn save_user_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        let request = SaveDataRequest {
            pubkey,
            data: encrypted_data,
        };
        self.post_request::<_, ()>("/store-vault-server/save-user-data", &request)
            .await
    }

    async fn get_user_data(&self, pubkey: U256) -> Result<Option<Vec<u8>>, ServerError> {
        let query = vec![("pubkey", pubkey.to_string())];
        let response: GetUserDataResponse = self
            .get_request("/store-vault-server/get-user-data", Some(query))
            .await?;
        Ok(response.data)
    }
}
