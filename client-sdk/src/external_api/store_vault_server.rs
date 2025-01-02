use async_trait::async_trait;
use intmax2_interfaces::{
    api::{
        error::ServerError,
        store_vault_server::{
            interface::{DataType, StoreVaultClientInterface},
            types::{
                BatchGetDataQuery, BatchGetDataResponse, BatchSaveDataRequest,
                BatchSaveDataResponse, GetBalanceProofQuery, GetBalanceProofResponse,
                GetDataAllAfterQuery, GetDataAllAfterResponse, GetDataQuery, GetDataResponse,
                GetUserDataQuery, GetUserDataResponse, SaveBalanceProofRequest, SaveDataRequest,
                SaveDataResponse,
            },
        },
    },
    data::meta_data::MetaData,
};
use intmax2_zkp::{ethereum_types::u256::U256, utils::poseidon_hash_out::PoseidonHashOut};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};

use super::utils::query::{get_request, post_request};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct StoreVaultServerClient {
    base_url: String,
}

impl StoreVaultServerClient {
    pub fn new(base_url: &str) -> Self {
        StoreVaultServerClient {
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait(?Send)]
impl StoreVaultClientInterface for StoreVaultServerClient {
    async fn save_balance_proof(
        &self,
        pubkey: U256,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError> {
        let request = SaveBalanceProofRequest {
            pubkey,
            balance_proof: proof.clone(),
        };
        post_request::<_, ()>(
            &self.base_url,
            "/store-vault-server/save-balance-proof",
            &request,
        )
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
        let response: GetBalanceProofResponse = get_request(
            &self.base_url,
            "/store-vault-server/get-balance-proof",
            Some(query),
        )
        .await?;
        Ok(response.balance_proof)
    }

    async fn save_data(
        &self,
        data_type: DataType,
        pubkey: U256,
        encrypted_data: &[u8],
    ) -> Result<String, ServerError> {
        let request = SaveDataRequest {
            pubkey,
            data: encrypted_data.to_vec(),
        };
        let response: SaveDataResponse = post_request(
            &self.base_url,
            &format!("/store-vault-server/{}/save", data_type.to_string()),
            &request,
        )
        .await?;
        Ok(response.uuid)
    }

    async fn save_data_batch(
        &self,
        data_type: DataType,
        data: Vec<(U256, Vec<u8>)>,
    ) -> Result<Vec<String>, ServerError> {
        let request = BatchSaveDataRequest { requests: data };
        let response: BatchSaveDataResponse = post_request(
            &self.base_url,
            &format!("/store-vault-server/{}/batch-save", data_type.to_string()),
            &request,
        )
        .await?;
        Ok(response.uuids)
    }

    async fn get_data(
        &self,
        data_type: DataType,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataQuery {
            uuid: uuid.to_string(),
        };
        let response: GetDataResponse = get_request(
            &self.base_url,
            &format!("/store-vault-server/{}/get", data_type.to_string()),
            Some(query),
        )
        .await?;
        Ok(response.data)
    }

    async fn get_data_batch(
        &self,
        data_type: DataType,
        uuids: &[String],
    ) -> Result<Vec<Option<(MetaData, Vec<u8>)>>, ServerError> {
        let query = BatchGetDataQuery {
            uuids: uuids.to_vec(),
        };
        let response: BatchGetDataResponse = get_request(
            &self.base_url,
            &format!("/store-vault-server/{}/batch-get", data_type.to_string()),
            Some(query),
        )
        .await?;
        Ok(response.data)
    }

    async fn get_data_all_after(
        &self,
        data_type: DataType,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let query = GetDataAllAfterQuery { pubkey, timestamp };
        let response: GetDataAllAfterResponse = get_request(
            &self.base_url,
            &format!(
                "/store-vault-server/{}/get-all-after",
                data_type.to_string()
            ),
            Some(query),
        )
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
        post_request::<_, ()>(
            &self.base_url,
            "/store-vault-server/save-user-data",
            &request,
        )
        .await
    }

    async fn get_user_data(&self, pubkey: U256) -> Result<Option<Vec<u8>>, ServerError> {
        let query = GetUserDataQuery { pubkey };
        let response: GetUserDataResponse = get_request(
            &self.base_url,
            "/store-vault-server/get-user-data",
            Some(query),
        )
        .await?;
        Ok(response.data)
    }
}
