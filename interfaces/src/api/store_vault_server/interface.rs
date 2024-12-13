use std::str::FromStr;

use async_trait::async_trait;
use intmax2_zkp::{ethereum_types::u256::U256, utils::poseidon_hash_out::PoseidonHashOut};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use serde::{Deserialize, Serialize};

use crate::{api::error::ServerError, data::meta_data::MetaData};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum DataType {
    Deposit,
    Transfer,
    Withdrawal,
    Tx,
}

impl ToString for DataType {
    fn to_string(&self) -> String {
        match self {
            DataType::Deposit => "deposit".to_string(),
            DataType::Transfer => "transfer".to_string(),
            DataType::Withdrawal => "withdrawal".to_string(),
            DataType::Tx => "tx".to_string(),
        }
    }
}

impl FromStr for DataType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deposit" => Ok(DataType::Deposit),
            "transfer" => Ok(DataType::Transfer),
            "withdrawal" => Ok(DataType::Withdrawal),
            "tx" => Ok(DataType::Tx),
            _ => Err(format!("Invalid data type: {}", s)),
        }
    }
}

#[async_trait(?Send)]
pub trait StoreVaultClientInterface {
    async fn save_balance_proof(
        &self,
        pubkey: U256,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError>;

    async fn get_balance_proof(
        &self,
        pubkey: U256,
        block_number: u32,
        private_commitment: PoseidonHashOut,
    ) -> Result<Option<ProofWithPublicInputs<F, C, D>>, ServerError>;

    async fn save_data(
        &self,
        data_type: DataType,
        pubkey: U256,
        encrypted_data: &[u8],
    ) -> Result<(), ServerError>;

    async fn save_data_batch(
        &self,
        data_type: DataType,
        data: Vec<(U256, Vec<u8>)>,
    ) -> Result<(), ServerError>;

    async fn get_data(
        &self,
        data_type: DataType,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError>;

    async fn get_data_batch(
        &self,
        data_type: DataType,
        uuid: &[String],
    ) -> Result<Vec<Option<(MetaData, Vec<u8>)>>, ServerError>;

    async fn get_data_all_after(
        &self,
        data_type: DataType,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError>;

    async fn save_user_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError>;

    async fn get_user_data(&self, pubkey: U256) -> Result<Option<Vec<u8>>, ServerError>;
}
