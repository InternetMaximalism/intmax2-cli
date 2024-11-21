use async_trait::async_trait;
use intmax2_zkp::{
    ethereum_types::u256::U256, mock::data::meta_data::MetaData,
    utils::poseidon_hash_out::PoseidonHashOut,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{
        circuit_data::VerifierCircuitData, config::PoseidonGoldilocksConfig,
        proof::ProofWithPublicInputs,
    },
};

use crate::{
    external_api::{
        common::error::ServerError, store_vault_server::interface::StoreVaultInterface,
    },
    utils::circuit_verifiers::CircuitVerifiers,
};

use super::{
    data_type::EncryptedDataType, get_balance_proof::get_balance_proof,
    get_encrypted_data::get_encrypted_data, get_encrypted_data_all::get_encrypted_data_all,
    get_user_data::get_user_data, save_balance_proof::save_balance_proof,
    save_encrypted_data::save_encrypted_data, save_user_data::save_user_data,
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct StoreVaultServer {
    pub server_base_url: String,
    pub balance_vd: VerifierCircuitData<F, C, D>,
}

impl StoreVaultServer {
    pub fn new(server_base_url: String) -> Self {
        let verifiers = CircuitVerifiers::load();
        let balance_vd = verifiers.get_balance_vd();
        Self {
            server_base_url,
            balance_vd: balance_vd.clone(),
        }
    }
}

#[async_trait(?Send)]
impl StoreVaultInterface for StoreVaultServer {
    async fn save_balance_proof(
        &self,
        pubkey: U256,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> Result<(), ServerError> {
        save_balance_proof(
            &self.balance_vd,
            &self.server_base_url,
            pubkey.into(),
            &proof,
        )
        .await?;
        Ok(())
    }

    async fn get_balance_proof(
        &self,
        pubkey: U256,
        block_number: u32,
        private_commitment: PoseidonHashOut,
    ) -> Result<Option<ProofWithPublicInputs<F, C, D>>, ServerError> {
        let proof = get_balance_proof(
            &self.balance_vd,
            &self.server_base_url,
            pubkey.into(),
            block_number,
            private_commitment,
        )
        .await?;
        Ok(proof)
    }

    async fn save_deposit_data(
        &self,
        pubkey: U256,
        encypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        save_encrypted_data(
            &self.server_base_url,
            EncryptedDataType::Deposit,
            pubkey.into(),
            encypted_data,
        )
        .await?;
        Ok(())
    }

    async fn get_deposit_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let data = get_encrypted_data_all(
            &self.server_base_url,
            EncryptedDataType::Deposit,
            pubkey.into(),
            timestamp,
        )
        .await?;
        Ok(data)
    }

    async fn get_deposit_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let data =
            get_encrypted_data(&self.server_base_url, EncryptedDataType::Deposit, uuid).await?;
        Ok(data)
    }

    async fn save_transfer_data(
        &self,
        pubkey: U256,
        encypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        save_encrypted_data(
            &self.server_base_url,
            EncryptedDataType::Transfer,
            pubkey.into(),
            encypted_data,
        )
        .await?;
        Ok(())
    }

    async fn get_transfer_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let data =
            get_encrypted_data(&self.server_base_url, EncryptedDataType::Transfer, uuid).await?;
        Ok(data)
    }

    async fn get_transfer_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let data = get_encrypted_data_all(
            &self.server_base_url,
            EncryptedDataType::Transfer,
            pubkey.into(),
            timestamp,
        )
        .await?;
        Ok(data)
    }

    async fn save_tx_data(&self, pubkey: U256, encypted_data: Vec<u8>) -> Result<(), ServerError> {
        save_encrypted_data(
            &self.server_base_url,
            EncryptedDataType::Transaction,
            pubkey.into(),
            encypted_data,
        )
        .await?;
        Ok(())
    }

    async fn get_tx_data(&self, uuid: &str) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let data =
            get_encrypted_data(&self.server_base_url, EncryptedDataType::Transaction, uuid).await?;
        Ok(data)
    }

    async fn get_tx_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let data = get_encrypted_data_all(
            &self.server_base_url,
            EncryptedDataType::Transaction,
            pubkey.into(),
            timestamp,
        )
        .await?;
        Ok(data)
    }

    async fn save_withdrawal_data(
        &self,
        pubkey: U256,
        encypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        save_encrypted_data(
            &self.server_base_url,
            EncryptedDataType::Withdrawal,
            pubkey.into(),
            encypted_data,
        )
        .await?;
        Ok(())
    }

    async fn get_withdrawal_data(
        &self,
        uuid: &str,
    ) -> Result<Option<(MetaData, Vec<u8>)>, ServerError> {
        let data =
            get_encrypted_data(&self.server_base_url, EncryptedDataType::Withdrawal, uuid).await?;
        Ok(data)
    }

    async fn get_withdrawal_data_all_after(
        &self,
        pubkey: U256,
        timestamp: u64,
    ) -> Result<Vec<(MetaData, Vec<u8>)>, ServerError> {
        let data = get_encrypted_data_all(
            &self.server_base_url,
            EncryptedDataType::Withdrawal,
            pubkey.into(),
            timestamp,
        )
        .await?;
        Ok(data)
    }

    async fn save_user_data(
        &self,
        pubkey: U256,
        encypted_data: Vec<u8>,
    ) -> Result<(), ServerError> {
        save_user_data(&self.server_base_url, pubkey.into(), encypted_data).await?;
        Ok(())
    }

    async fn get_user_data(&self, pubkey: U256) -> Result<Option<Vec<u8>>, ServerError> {
        let data = get_user_data(&self.server_base_url, pubkey.into()).await?;
        Ok(data)
    }
}
