use async_trait::async_trait;
use intmax2_zkp::{
    ethereum_types::u256::U256, mock::data::meta_data::MetaData,
    utils::poseidon_hash_out::PoseidonHashOut,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};

use crate::external_api::common::error::ServerError;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[async_trait(?Send)]
pub trait StoreVaultInterface {
    async fn save_balance_proof(
        &self,
        pubkey: U256,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError>;

    async fn get_balance_proof(
        &self,
        pubkey: U256,
        block_number: u32,
        private_commitment: PoseidonHashOut,
    ) -> Result<Option<ProofWithPublicInputs<F, C, D>>, ServerError>;

    async fn save_deposit_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError>;

    async fn get_deposit_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError>;

    async fn get_deposit_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError>;

    async fn save_transfer_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError>;

    async fn get_transfer_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError>;

    async fn get_transfer_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError>;

    async fn save_tx_data(&self, pubkey: U256, encrypted_data: Vec<u8>) -> Result<(), ServerError>;

    async fn get_tx_data(&self, uuid: &str) -> Result<Option<(MetaData, Vec<u8>)>, ServerError>;

    async fn get_tx_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError>;

    async fn save_withdrawal_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError>;

    async fn get_withdrawal_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError>;

    async fn get_withdrawal_data_all_after(
        &self,
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
