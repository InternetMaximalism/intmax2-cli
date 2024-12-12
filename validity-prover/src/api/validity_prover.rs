use intmax2_client_sdk::external_api::contract::rollup_contract::RollupContract;
use intmax2_interfaces::api::validity_prover::interface::{AccountInfo, DepositInfo};
use intmax2_zkp::{
    circuits::validity::{
        validity_pis::ValidityPublicInputs, validity_processor::ValidityProcessor,
    },
    common::{
        block::Block,
        trees::{
            account_tree::{AccountMembershipProof, AccountTree},
            block_hash_tree::{BlockHashMerkleProof, BlockHashTree},
            deposit_tree::DepositMerkleProof,
            sender_tree::SenderLeaf,
        },
        witness::update_witness::UpdateWitness,
    },
    constants::BLOCK_HASH_TREE_HEIGHT,
    ethereum_types::{bytes32::Bytes32, u256::U256, u32limb_trait::U32LimbTrait as _},
};

use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{sync::OnceLock, time::Duration};

use super::{error::ValidityProverError, observer::Observer};
use crate::{utils::deposit_hash_tree::DepositHashTree, Env};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;
pub struct ValidityProver {
    validity_processor: OnceLock<ValidityProcessor<F, C, D>>,
    observer: Observer,
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

        // Initialize state if empty
        let count = sqlx::query!("SELECT COUNT(*) as count FROM validity_state")
            .fetch_one(&pool)
            .await?
            .count
            .unwrap_or(0);

        if count == 0 {
            let mut tx = pool.begin().await?;

            // Initialize validity state
            sqlx::query!("INSERT INTO validity_state (id, last_block_number) VALUES (1, 0)")
                .execute(&mut *tx)
                .await?;

            // Initialize genesis state
            let account_tree = AccountTree::initialize();
            let mut block_tree = BlockHashTree::new(BLOCK_HASH_TREE_HEIGHT);
            block_tree.push(Block::genesis().hash());
            let deposit_hash_tree = DepositHashTree::new();

            sqlx::query!(
                "INSERT INTO account_trees (block_number, tree_data) VALUES (0, $1)",
                serde_json::to_value(&account_tree)?
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO block_hash_trees (block_number, tree_data) VALUES (0, $1)",
                serde_json::to_value(&block_tree)?
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO deposit_hash_trees (block_number, tree_data) VALUES (0, $1)",
                serde_json::to_value(&deposit_hash_tree)?
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO sender_leaves (block_number, leaves) VALUES (0, $1)",
                serde_json::to_value::<Vec<SenderLeaf>>(vec![])?
            )
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
        }

        Ok(Self {
            validity_processor,
            observer,
            pool,
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
        // Load current state
        let mut account_tree: AccountTree = {
            let record = sqlx::query!(
                "SELECT tree_data FROM account_trees WHERE block_number = $1",
                last_block_number as i32
            )
            .fetch_one(&self.pool)
            .await?;
            serde_json::from_value(record.tree_data)?
        };

        let mut block_tree: BlockHashTree = {
            let record = sqlx::query!(
                "SELECT tree_data FROM block_hash_trees WHERE block_number = $1",
                last_block_number as i32
            )
            .fetch_one(&self.pool)
            .await?;

            serde_json::from_value(record.tree_data)?
        };

        let mut deposit_hash_tree: DepositHashTree = {
            let record = sqlx::query!(
                "SELECT tree_data FROM deposit_hash_trees WHERE block_number = $1",
                last_block_number as i32
            )
            .fetch_one(&self.pool)
            .await?;

            serde_json::from_value(record.tree_data)?
        };

        let next_block_number = self.observer.get_next_block_number().await?;

        for block_number in (last_block_number + 1)..next_block_number {
            log::info!(
                "Sync validity prover: syncing block number {}",
                block_number
            );

            let mut tx = self.pool.begin().await?;

            let prev_validity_proof = self.get_validity_proof(block_number - 1).await?;
            assert!(
                prev_validity_proof.is_some() || block_number == 1,
                "prev validity proof not found"
            );

            let full_block = self.observer.get_full_block(block_number).await?;
            let block_witness = full_block
                .to_block_witness(&account_tree, &block_tree)
                .map_err(|e| ValidityProverError::BlockWitnessGenerationError(e.to_string()))?;

            let validity_witness = block_witness
                .update_trees(&mut account_tree, &mut block_tree)
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
                deposit_hash_tree.push(event.deposit_hash);
            }

            if full_block.block.deposit_tree_root != deposit_hash_tree.get_root() {
                return Err(ValidityProverError::DepositTreeRootMismatch(
                    full_block.block.deposit_tree_root,
                    deposit_hash_tree.get_root(),
                ));
            }

            // Update database state
            sqlx::query!(
                "UPDATE validity_state SET last_block_number = $1 WHERE id = 1",
                block_number as i32
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO account_trees (block_number, tree_data) VALUES ($1, $2)",
                block_number as i32,
                serde_json::to_value(&account_tree)?
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO block_hash_trees (block_number, tree_data) VALUES ($1, $2)",
                block_number as i32,
                serde_json::to_value(&block_tree)?
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO deposit_hash_trees (block_number, tree_data) VALUES ($1, $2)",
                block_number as i32,
                serde_json::to_value(&deposit_hash_tree)?
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
        let last_block_number = self.get_block_number().await?;

        let record = sqlx::query!(
            "SELECT tree_data FROM account_trees WHERE block_number = $1",
            last_block_number as i32
        )
        .fetch_one(&self.pool)
        .await?;

        let account_tree: AccountTree = serde_json::from_value(record.tree_data)?;

        Ok(account_tree.index(pubkey))
    }

    pub async fn get_account_info(&self, pubkey: U256) -> Result<AccountInfo, ValidityProverError> {
        let block_number = self.get_block_number().await?;

        let record = sqlx::query!(
            "SELECT tree_data FROM account_trees WHERE block_number = $1",
            block_number as i32
        )
        .fetch_one(&self.pool)
        .await?;

        let account_tree: AccountTree = serde_json::from_value(record.tree_data)?;

        let account_id = account_tree.index(pubkey);

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

        let record = sqlx::query!(
            "SELECT tree_data FROM block_hash_trees WHERE block_number = $1",
            root_block_number as i32
        )
        .fetch_one(&self.pool)
        .await?;

        let block_tree: BlockHashTree = serde_json::from_value(record.tree_data)?;

        Ok(block_tree.prove(leaf_block_number as u64))
    }

    async fn get_account_membership_proof(
        &self,
        block_number: u32,
        pubkey: U256,
    ) -> Result<AccountMembershipProof, ValidityProverError> {
        let record = sqlx::query!(
            "SELECT tree_data FROM account_trees WHERE block_number = $1",
            block_number as i32
        )
        .fetch_one(&self.pool)
        .await?;

        let account_tree: AccountTree = serde_json::from_value(record.tree_data)?;

        Ok(account_tree.prove_membership(pubkey))
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
        let record = sqlx::query!(
            "SELECT tree_data FROM deposit_hash_trees WHERE block_number = $1",
            block_number as i32
        )
        .fetch_one(&self.pool)
        .await?;

        let deposit_hash_tree: DepositHashTree = serde_json::from_value(record.tree_data)?;

        Ok(deposit_hash_tree.prove(deposit_index))
    }

    pub async fn get_account_tree(
        &self,
        block_number: u32,
    ) -> Result<AccountTree, ValidityProverError> {
        let record = sqlx::query!(
            "SELECT tree_data FROM account_trees WHERE block_number = $1",
            block_number as i32
        )
        .fetch_one(&self.pool)
        .await?;

        let account_tree: AccountTree = serde_json::from_value(record.tree_data)?;
        Ok(account_tree)
    }

    pub async fn get_block_hash_tree(
        &self,
        block_number: u32,
    ) -> Result<BlockHashTree, ValidityProverError> {
        let record = sqlx::query!(
            "SELECT tree_data FROM block_hash_trees WHERE block_number = $1",
            block_number as i32
        )
        .fetch_one(&self.pool)
        .await?;

        let block_tree: BlockHashTree = serde_json::from_value(record.tree_data)?;
        Ok(block_tree)
    }

    pub async fn get_deposit_hash_tree(
        &self,
        block_number: u32,
    ) -> Result<DepositHashTree, ValidityProverError> {
        let record = sqlx::query!(
            "SELECT tree_data FROM deposit_hash_trees WHERE block_number = $1",
            block_number as i32
        )
        .fetch_one(&self.pool)
        .await?;

        let deposit_hash_tree: DepositHashTree = serde_json::from_value(record.tree_data)?;
        Ok(deposit_hash_tree)
    }

    pub fn validity_processor(&self) -> &ValidityProcessor<F, C, D> {
        self.validity_processor
            .get_or_init(|| ValidityProcessor::new())
    }
}
