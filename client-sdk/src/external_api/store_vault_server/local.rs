use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use intmax2_zkp::{
    ethereum_types::u256::U256,
    mock::{
        data::meta_data::MetaData, store_vault_server::StoreVaultServer as StoreVaultServerInner,
    },
    utils::poseidon_hash_out::PoseidonHashOut,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};

use crate::external_api::common::error::ServerError;

use super::interface::StoreVaultInterface;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone)]
pub struct LocalStoreVaultServer(Arc<Mutex<StoreVaultServerInner<F, C, D>>>);

impl LocalStoreVaultServer {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(StoreVaultServerInner::new())))
    }

    pub fn reset(&self) {
        self.0.lock().unwrap().reset();
    }
}

#[async_trait(?Send)]
impl StoreVaultInterface for LocalStoreVaultServer {
    async fn save_balance_proof(
        &self,
        pubkey: U256,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError> {
        self.0.lock().unwrap().save_balance_proof(pubkey, proof);
        Ok(())
    }

    async fn get_balance_proof(
        &self,
        pubkey: U256,
        block_number: u32,
        private_commitment: PoseidonHashOut,
    ) -> Result<Option<ProofWithPublicInputs<F, C, D>>, ServerError> {
        let proof = self
            .0
            .lock()
            .unwrap()
            .get_balance_proof(pubkey, block_number, private_commitment)
            .map_err(|e| ServerError::InternalError(e.to_string()))?;
        Ok(proof)
    }

    async fn save_deposit_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        self.0
            .lock()
            .unwrap()
            .save_deposit_data(pubkey, encrypted_data);
        Ok(())
    }

    async fn get_deposit_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let data = self
            .0
            .lock()
            .unwrap()
            .get_deposit_data_all_after(pubkey, timestamp);
        Ok(data)
    }

    async fn get_deposit_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let data = self.0.lock().unwrap().get_deposit_data(uuid);
        Ok(data)
    }

    async fn save_transfer_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        self.0
            .lock()
            .unwrap()
            .save_transfer_data(pubkey, encrypted_data);
        Ok(())
    }

    async fn get_transfer_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let data = self.0.lock().unwrap().get_transfer_data(uuid);
        Ok(data)
    }

    async fn get_transfer_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let data = self
            .0
            .lock()
            .unwrap()
            .get_transfer_data_all_after(pubkey, timestamp);
        Ok(data)
    }

    async fn save_tx_data(&self, pubkey: U256, encrypted_data: Vec<u8>) -> Result<(), ServerError> {
        self.0.lock().unwrap().save_tx_data(pubkey, encrypted_data);
        Ok(())
    }

    async fn get_tx_data(&self, uuid: &str) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let data = self.0.lock().unwrap().get_tx_data(uuid);
        Ok(data)
    }

    async fn get_tx_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let data = self
            .0
            .lock()
            .unwrap()
            .get_tx_data_all_after(pubkey, timestamp);
        Ok(data)
    }

    async fn save_withdrawal_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        self.0
            .lock()
            .unwrap()
            .save_withdrawal_data(pubkey, encrypted_data);
        Ok(())
    }

    async fn get_withdrawal_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let data = self.0.lock().unwrap().get_withdrawal_data(uuid);
        Ok(data)
    }

    async fn get_withdrawal_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let data = self
            .0
            .lock()
            .unwrap()
            .get_withdrawal_data_all_after(pubkey, timestamp);
        Ok(data)
    }

    async fn save_user_data(
        &self,
        pubkey: U256,
        encrypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        self.0
            .lock()
            .unwrap()
            .save_user_data(pubkey, encrypted_data);
        Ok(())
    }

    async fn get_user_data(&self, pubkey: U256) -> Result<Option<Vec<u8>>, ServerError> {
        let data = self.0.lock().unwrap().get_user_data(pubkey);
        Ok(data)
    }
}
