use intmax2_client_sdk::external_api::contract::rollup_contract::RollupContract;
use intmax2_interfaces::api::validity_prover::interface::{AccountInfo, DepositInfo};
use intmax2_zkp::{
    circuits::validity::{
        validity_pis::ValidityPublicInputs, validity_processor::ValidityProcessor,
    },
    common::{
        block::Block,
        trees::{
            account_tree::AccountMembershipProof, block_hash_tree::BlockHashMerkleProof,
            deposit_tree::DepositMerkleProof, sender_tree::SenderLeaf,
        },
        witness::update_witness::UpdateWitness,
    },
    constants::{ACCOUNT_TREE_HEIGHT, BLOCK_HASH_TREE_HEIGHT, DEPOSIT_TREE_HEIGHT},
    ethereum_types::{bytes32::Bytes32, u256::U256, u32limb_trait::U32LimbTrait as _},
    utils::trees::{
        incremental_merkle_tree::IncrementalMerkleProof,
        indexed_merkle_tree::leaf::IndexedMerkleLeaf, merkle_tree::MerkleProof,
    },
};

use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{sync::OnceLock, time::Duration};

use super::{error::ValidityProverError, observer::Observer};
use crate::{
    trees::{
        account_tree::HistoricalAccountTree,
        block_tree::HistoricalBlockHashTree,
        deposit_hash_tree::{DepositHash, HistoricalDepositHashTree},
        node::{NodeDB, SqlNodeDB},
        utils::{to_block_witness, update_trees},
    },
    Env,
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

type ADB = SqlNodeDB<IndexedMerkleLeaf>;
type BDB = SqlNodeDB<Bytes32>;
type DDB = SqlNodeDB<DepositHash>;

const ACCOUNT_DB_TAG: u32 = 1;
const BLOCK_DB_TAG: u32 = 2;
const DEPOSIT_DB_TAG: u32 = 3;

pub struct ValidityProver {
    validity_processor: OnceLock<ValidityProcessor<F, C, D>>,
    observer: Observer,
    account_tree: HistoricalAccountTree<ADB>,
    block_tree: HistoricalBlockHashTree<BDB>,
    deposit_hash_tree: HistoricalDepositHashTree<DDB>,
    pool: PgPool,
}

impl ValidityProver {
    pub async fn new(env: &Env) -> Result<Self, ValidityProverError> {
        let rollup_contract = RollupContract::new(
            &env.l2_rpc_url,
            env.l2_chain_id,
            env.rollup_contract_address,
            env.rollup_contract_deployed_block_number,
        );
        let observer = Observer::new(
            rollup_contract,
            &env.database_url,
            env.database_max_connections,
            env.database_timeout,
        )
        .await?;
        let validity_processor = OnceLock::new();

        let pool = PgPoolOptions::new()
            .max_connections(env.database_max_connections)
            .idle_timeout(Duration::from_secs(env.database_timeout))
            .connect(&env.database_url)
            .await?;

        let account_db = SqlNodeDB::new(&env.database_url, ACCOUNT_DB_TAG).await?;
        let account_tree =
            HistoricalAccountTree::new(account_db, ACCOUNT_TREE_HEIGHT as u32).await?;

        let block_db = SqlNodeDB::new(&env.database_url, BLOCK_DB_TAG).await?;
        let block_tree =
            HistoricalBlockHashTree::new(block_db, BLOCK_HASH_TREE_HEIGHT as u32).await?;
        if block_tree.len().await? == 0 {
            block_tree.push(Block::genesis().hash()).await?;
        }
        let deposit_db = SqlNodeDB::new(&env.database_url, DEPOSIT_DB_TAG).await?;
        let deposit_hash_tree =
            HistoricalDepositHashTree::new(deposit_db, DEPOSIT_TREE_HEIGHT as u32).await?;

        // Initialize state if empty
        let count = sqlx::query!("SELECT COUNT(*) as count FROM validity_state")
            .fetch_one(&pool)
            .await?
            .count
            .unwrap_or(0);

        if count == 0 {
            let mut tx = pool.begin().await?;
            sqlx::query!("INSERT INTO validity_state (id, last_block_number) VALUES (1, 0)")
                .execute(&mut *tx)
                .await?;
            sqlx::query!(
                "INSERT INTO sender_leaves (block_number, leaves) VALUES (0, $1)",
                serde_json::to_value::<Vec<SenderLeaf>>(vec![])?
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;

            // save roots
            let block_number = 0u32;
            let account_tree_root = account_tree.get_current_root().await?;
            account_tree
                .node_db()
                .save_root(block_number as u64, account_tree_root)
                .await?;
            let block_tree_root = block_tree.get_current_root().await?;
            block_tree
                .node_db()
                .save_root(block_number as u64, block_tree_root)
                .await?;
            let deposit_tree_root = deposit_hash_tree.get_current_root().await?;
            deposit_hash_tree
                .node_db()
                .save_root(block_number as u64, deposit_tree_root)
                .await?;
        }

        Ok(Self {
            validity_processor,
            observer,
            pool,
            account_tree,
            block_tree,
            deposit_hash_tree,
        })
    }

    pub async fn sync_observer(&self) -> Result<(), ValidityProverError> {
        self.observer.sync().await?;
        Ok(())
    }

    pub async fn get_validity_proof(
        &self,
        block_number: u32,
    ) -> Result<Option<ProofWithPublicInputs<F, C, D>>, ValidityProverError> {
        let record = sqlx::query!(
            "SELECT proof FROM validity_proofs WHERE block_number = $1",
            block_number as i32
        )
        .fetch_optional(&self.pool)
        .await?;
        match record {
            Some(r) => {
                let proof: ProofWithPublicInputs<F, C, D> = serde_json::from_value(r.proof)?;
                Ok(Some(proof))
            }
            None => Ok(None),
        }
    }

    pub async fn sync(&self) -> Result<(), ValidityProverError> {
        log::info!("Start sync validity prover");
        self.sync_observer().await?;

        let last_block_number = self.get_block_number().await?;
        let next_block_number = self.observer.get_next_block_number().await?;

        for block_number in (last_block_number + 1)..next_block_number {
            log::info!(
                "Sync validity prover: syncing block number {}",
                block_number
            );

            let prev_validity_proof = self.get_validity_proof(block_number - 1).await?;
            assert!(
                prev_validity_proof.is_some() || block_number == 1,
                "prev validity proof not found"
            );

            let full_block = self.observer.get_full_block(block_number).await?;
            let block_witness = to_block_witness(&full_block, &self.account_tree, &self.block_tree)
                .await
                .map_err(|e| ValidityProverError::BlockWitnessGenerationError(e.to_string()))?;

            let validity_witness =
                update_trees(&block_witness, &self.account_tree, &self.block_tree)
                    .await
                    .map_err(|e| ValidityProverError::FailedToUpdateTrees(e.to_string()))?;

            let validity_proof = self
                .validity_processor()
                .prove(&prev_validity_proof, &validity_witness)
                .map_err(|e| ValidityProverError::ValidityProveError(e.to_string()))?;

            log::info!(
                "Sync validity prover: block number {} validity proof generated",
                block_number
            );

            let deposit_events = self
                .observer
                .get_deposits_between_blocks(block_number)
                .await?;

            for event in deposit_events {
                self.deposit_hash_tree
                    .push(DepositHash(event.deposit_hash))
                    .await?;
            }

            let deposit_tree_root = self.deposit_hash_tree.get_current_root().await?;
            if full_block.block.deposit_tree_root != deposit_tree_root {
                return Err(ValidityProverError::DepositTreeRootMismatch(
                    full_block.block.deposit_tree_root,
                    deposit_tree_root,
                ));
            }

            // Record tree roots
            let account_tree_root = self.account_tree.get_current_root().await?;
            self.account_tree
                .node_db()
                .save_root(block_number as u64, account_tree_root)
                .await?;
            let block_tree_root = self.block_tree.get_current_root().await?;
            self.block_tree
                .node_db()
                .save_root(block_number as u64, block_tree_root)
                .await?;
            self.deposit_hash_tree
                .node_db()
                .save_root(block_number as u64, deposit_tree_root)
                .await?;

            // Update database state
            let mut tx = self.pool.begin().await?;
            sqlx::query!(
                "UPDATE validity_state SET last_block_number = $1 WHERE id = 1",
                block_number as i32
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO validity_proofs (block_number, proof) VALUES ($1, $2)",
                block_number as i32,
                serde_json::to_value(&validity_proof)?
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO sender_leaves (block_number, leaves) VALUES ($1, $2)",
                block_number as i32,
                serde_json::to_value(&block_witness.get_sender_tree().leaves())?
            )
            .execute(&mut *tx)
            .await?;

            let tx_tree_root = full_block.signature.tx_tree_root;
            if tx_tree_root != Bytes32::default()
                && validity_witness.to_validity_pis().unwrap().is_valid_block
            {
                sqlx::query!(
                    "INSERT INTO tx_tree_roots (tx_tree_root, block_number) VALUES ($1, $2)
                     ON CONFLICT (tx_tree_root) DO UPDATE SET block_number = $2",
                    tx_tree_root.to_bytes_be(),
                    block_number as i32
                )
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
        }

        log::info!("End of sync validity prover");
        Ok(())
    }

    pub async fn get_update_witness(
        &self,
        pubkey: U256,
        root_block_number: u32,
        leaf_block_number: u32,
        is_prev_account_tree: bool,
    ) -> Result<UpdateWitness<F, C, D>, ValidityProverError> {
        let validity_proof = self.get_validity_proof(root_block_number).await?.ok_or(
            ValidityProverError::ValidityProofNotFound(root_block_number),
        )?;

        let block_merkle_proof = self
            .get_block_merkle_proof(root_block_number, leaf_block_number)
            .await?;

        let account_tree_block_number = if is_prev_account_tree {
            root_block_number - 1
        } else {
            root_block_number
        };

        let account_membership_proof = self
            .get_account_membership_proof(account_tree_block_number, pubkey)
            .await?;

        Ok(UpdateWitness {
            is_prev_account_tree,
            validity_proof,
            block_merkle_proof,
            account_membership_proof,
        })
    }

    pub async fn get_account_id(&self, pubkey: U256) -> Result<Option<u64>, ValidityProverError> {
        let leaves = self.account_tree.get_current_leaves().await?;
        let index = self.account_tree.index(&leaves, pubkey).await?;
        Ok(index)
    }

    pub async fn get_account_info(&self, pubkey: U256) -> Result<AccountInfo, ValidityProverError> {
        let block_number = self.get_block_number().await?;
        let leaves = self.account_tree.get_current_leaves().await?;
        let account_id = self.account_tree.index(&leaves, pubkey).await?;
        Ok(AccountInfo {
            block_number,
            account_id,
        })
    }

    pub async fn get_deposit_info(
        &self,
        deposit_hash: Bytes32,
    ) -> Result<Option<DepositInfo>, ValidityProverError> {
        let deposit_info = self.observer.get_deposit_info(deposit_hash).await?;
        Ok(deposit_info)
    }

    pub async fn get_block_number_by_tx_tree_root(
        &self,
        tx_tree_root: Bytes32,
    ) -> Result<Option<u32>, ValidityProverError> {
        let record = sqlx::query!(
            "SELECT block_number FROM tx_tree_roots WHERE tx_tree_root = $1",
            tx_tree_root.to_bytes_be()
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.block_number as u32))
    }

    pub async fn get_validity_pis(
        &self,
        block_number: u32,
    ) -> Result<Option<ValidityPublicInputs>, ValidityProverError> {
        let validity_proof = self.get_validity_proof(block_number).await?;
        Ok(validity_proof.map(|proof| ValidityPublicInputs::from_pis(&proof.public_inputs)))
    }

    pub async fn get_sender_leaves(
        &self,
        block_number: u32,
    ) -> Result<Option<Vec<SenderLeaf>>, ValidityProverError> {
        let record = sqlx::query!(
            "SELECT leaves FROM sender_leaves WHERE block_number = $1",
            block_number as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        match record {
            Some(r) => {
                let leaves: Vec<SenderLeaf> = serde_json::from_value(r.leaves)?;
                Ok(Some(leaves))
            }
            None => Ok(None),
        }
    }

    pub async fn get_block_merkle_proof(
        &self,
        root_block_number: u32,
        leaf_block_number: u32,
    ) -> Result<BlockHashMerkleProof, ValidityProverError> {
        if leaf_block_number > root_block_number {
            return Err(ValidityProverError::InputError(
                "leaf_block_number should be smaller than root_block_number".to_string(),
            ));
        }

        let block_tree_root = self
            .block_tree
            .node_db()
            .get_root(root_block_number as u64)
            .await?
            .ok_or(ValidityProverError::BlockTreeNotFound(root_block_number))?;

        let proof = self
            .block_tree
            .prove_by_root(block_tree_root, leaf_block_number as u64)
            .await?;

        Ok(proof)
    }

    async fn get_account_membership_proof(
        &self,
        block_number: u32,
        pubkey: U256,
    ) -> Result<AccountMembershipProof, ValidityProverError> {
        let account_tree_root = self
            .account_tree
            .node_db()
            .get_root(block_number as u64)
            .await?
            .ok_or(ValidityProverError::AccountTreeNotFound(block_number))?;
        let proof = self
            .account_tree
            .prove_membership_by_root(account_tree_root, pubkey)
            .await?;
        Ok(proof)
    }

    pub async fn get_block_number(&self) -> Result<u32, ValidityProverError> {
        let record = sqlx::query!("SELECT last_block_number FROM validity_state WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;

        Ok(record.last_block_number as u32)
    }

    pub async fn get_next_deposit_index(&self) -> Result<u32, ValidityProverError> {
        let deposit_index = self.observer.get_next_deposit_index().await?;
        Ok(deposit_index)
    }

    pub async fn get_deposit_merkle_proof(
        &self,
        block_number: u32,
        deposit_index: u32,
    ) -> Result<DepositMerkleProof, ValidityProverError> {
        let deposit_tree_root = self
            .deposit_hash_tree
            .node_db()
            .get_root(block_number as u64)
            .await?
            .ok_or(ValidityProverError::DepositTreeRootNotFound(block_number))?;
        let proof = self
            .deposit_hash_tree
            .prove_by_root(deposit_tree_root, deposit_index as u64)
            .await?;
        Ok(IncrementalMerkleProof(MerkleProof {
            siblings: proof.0.siblings,
        }))
    }

    pub fn validity_processor(&self) -> &ValidityProcessor<F, C, D> {
        self.validity_processor
            .get_or_init(|| ValidityProcessor::new())
    }
}
