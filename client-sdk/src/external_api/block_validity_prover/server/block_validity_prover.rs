use async_trait::async_trait;
use intmax2_zkp::{
    circuits::validity::validity_pis::ValidityPublicInputs,
    common::{
        trees::{
            block_hash_tree::BlockHashMerkleProof, deposit_tree::DepositMerkleProof,
            sender_tree::SenderLeaf,
        },
        witness::update_witness::UpdateWitness,
    },
    ethereum_types::{bytes32::Bytes32, u256::U256},
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{circuit_data::VerifierCircuitData, config::PoseidonGoldilocksConfig},
};

use crate::{
    external_api::{
        block_validity_prover::{
            interface::BlockValidityInterface,
            server::{
                account_id::get_account_id, block_merkle_proof::get_block_merkle_proof,
                deposit_index::get_deposit_index_and_block_number,
                deposit_merkle_proof::get_deposit_merkle_proof, info::get_info,
                tx_tree_root_status::get_tx_tree_root_status, update_witness::get_update_witness,
                validity_pis::get_validity_pis,
            },
        },
        common::error::ServerError,
    },
    utils::circuit_verifiers::CircuitVerifiers,
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct BlockValidityProver {
    pub server_base_url: String,
    pub validity_vd: VerifierCircuitData<F, C, D>,
}

impl BlockValidityProver {
    pub fn new(server_base_url: String) -> Self {
        let verifiers = CircuitVerifiers::load();
        let validity_vd = verifiers.get_validity_vd();
        Self {
            server_base_url,
            validity_vd: validity_vd.clone(),
        }
    }
}

#[async_trait(?Send)]
impl BlockValidityInterface for BlockValidityProver {
    async fn block_number(&self) -> Result<u32, ServerError> {
        log::info!("Getting block_number");
        let block_number = get_info(&self.server_base_url).await?;
        Ok(block_number)
    }

    async fn get_account_id(&self, pubkey: U256) -> Result<Option<u64>, ServerError> {
        log::info!("Getting account_id");
        let account_id = get_account_id(&self.server_base_url, pubkey.into()).await?;
        Ok(account_id)
    }

    async fn get_update_witness(
        &self,
        pubkey: U256,
        root_block_number: u32,
        leaf_block_number: u32,
        is_prev_account_tree: bool,
    ) -> Result<UpdateWitness<F, C, D>, ServerError> {
        log::info!("Getting update_witness");
        let update_witness = get_update_witness(
            &self.server_base_url,
            pubkey.into(),
            root_block_number,
            leaf_block_number,
            is_prev_account_tree,
        )
        .await?;
        Ok(update_witness)
    }

    async fn get_deposit_index_and_block_number(
        &self,
        deposit_hash: Bytes32,
    ) -> Result<Option<(u32, u32)>, ServerError> {
        log::info!("Getting deposit_index_and_block_number");
        let deposit_index_and_block_number =
            get_deposit_index_and_block_number(&self.server_base_url, deposit_hash).await?;
        Ok(deposit_index_and_block_number)
    }

    async fn get_block_number_by_tx_tree_root(
        &self,
        tx_tree_root: Bytes32,
    ) -> Result<Option<u32>, ServerError> {
        log::info!("Getting block_number_by_tx_tree_root");
        let status = get_tx_tree_root_status(&self.server_base_url, tx_tree_root).await?;
        let block_number = status.map(|(block_number, _)| block_number);
        Ok(block_number)
    }

    async fn get_validity_pis(
        &self,
        block_number: u32,
    ) -> Result<Option<ValidityPublicInputs>, ServerError> {
        log::info!("Getting validity_pis");
        let validity_pis = get_validity_pis(&self.server_base_url, block_number).await?;
        Ok(validity_pis)
    }

    async fn get_sender_leaves(
        &self,
        block_number: u32,
    ) -> Result<Option<Vec<SenderLeaf>>, ServerError> {
        let validity_pis = self.get_validity_pis(block_number).await?;
        let tx_tree_root = validity_pis
            .map(|validity_pis| validity_pis.tx_tree_root)
            .ok_or(ServerError::InternalError(
                "Failed to get tx tree root from validity pis".to_string(),
            ))?;

        log::info!("Getting sender_leaves");
        let status = get_tx_tree_root_status(&self.server_base_url, tx_tree_root).await?;
        let sender_leaves = status.map(|(_, sender_leaves)| sender_leaves);
        Ok(sender_leaves)
    }

    async fn get_block_merkle_proof(
        &self,
        root_block_number: u32,
        leaf_block_number: u32,
    ) -> Result<BlockHashMerkleProof, ServerError> {
        log::info!("Getting block_merkle_proof");
        let block_merkle_proof =
            get_block_merkle_proof(&self.server_base_url, root_block_number, leaf_block_number)
                .await?;
        Ok(block_merkle_proof)
    }

    async fn get_deposit_merkle_proof(
        &self,
        block_number: u32,
        deposit_index: u32,
    ) -> Result<DepositMerkleProof, ServerError> {
        log::info!("Getting deposit_merkle_proof");
        let deposit_merkle_proof =
            get_deposit_merkle_proof(&self.server_base_url, block_number, deposit_index).await?;
        Ok(deposit_merkle_proof)
    }
}
