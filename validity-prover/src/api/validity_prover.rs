use std::sync::{Arc, OnceLock};

use hashbrown::HashMap;
use intmax2_client_sdk::external_api::contract::rollup_contract::RollupContract;
use intmax2_zkp::{
    circuits::validity::validity_processor::ValidityProcessor,
    common::{
        block::Block,
        trees::{
            account_tree::AccountTree,
            block_hash_tree::{BlockHashMerkleProof, BlockHashTree},
            sender_tree::SenderLeaf,
        },
        witness::update_witness::UpdateWitness,
    },
    constants::BLOCK_HASH_TREE_HEIGHT,
    ethereum_types::{bytes32::Bytes32, u256::U256},
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use tokio::sync::RwLock;

use crate::utils::deposit_hash_tree::DepositHashTree;

use super::observer::{Observer, ObserverError};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, thiserror::Error)]
pub enum ValidityProverError {
    #[error("Observer error: {0}")]
    ObserverError(#[from] ObserverError),

    #[error("Block witness generation error: {0}")]
    BlockWitnessGenerationError(String),

    #[error("Failed to update trees: {0}")]
    FailedToUpdateTrees(String),

    #[error("Validity prove error: {0}")]
    ValidityProveError(String),

    #[error("Deposit tree root mismatch")]
    DepositTreeRootMismatch(Bytes32, Bytes32),

    #[error("Validity proof not found for block number {0}")]
    ValidityProofNotFound(u32),

    #[error("Input error {0}")]
    InputError(String),
}

pub struct ValidityProver {
    validity_processor: OnceLock<ValidityProcessor<F, C, D>>, // delayed initialization
    observer: Observer,

    // TODO: make these DB backed & more efficient snaphots (e.g. DB merkle tree)
    data: Arc<RwLock<Data>>,
}

struct Data {
    last_block_number: u32,
    validity_proofs: HashMap<u32, ProofWithPublicInputs<F, C, D>>,
    account_trees: HashMap<u32, AccountTree>,
    block_trees: HashMap<u32, BlockHashTree>,
    deposit_hash_trees: HashMap<u32, DepositHashTree>,
    tx_tree_roots: HashMap<Bytes32, u32>,
    sender_leaves: HashMap<u32, Vec<SenderLeaf>>,
}

impl Data {
    pub fn new() -> Self {
        let last_block_number = 0;
        let account_tree = AccountTree::initialize();
        let mut block_tree = BlockHashTree::new(BLOCK_HASH_TREE_HEIGHT);
        block_tree.push(Block::genesis().hash());

        let mut account_trees = HashMap::new();
        account_trees.insert(last_block_number, account_tree);
        let mut block_trees = HashMap::new();
        block_trees.insert(last_block_number, block_tree);

        let deposit_hash_tree = DepositHashTree::new();
        let mut deposit_hash_trees = HashMap::new();
        deposit_hash_trees.insert(last_block_number, deposit_hash_tree);

        let mut sender_leaves = HashMap::new();
        sender_leaves.insert(last_block_number, vec![]);

        Self {
            last_block_number,
            validity_proofs: HashMap::new(),
            account_trees,
            block_trees,
            deposit_hash_trees,
            tx_tree_roots: HashMap::new(),
            sender_leaves,
        }
    }
}

impl ValidityProver {
    pub fn new(
        rpc_url: &str,
        chain_id: u64,
        rollup_contract_address: ethers::types::Address,
        rollup_contract_deployed_block_number: u64,
    ) -> Self {
        let rollup_contract = RollupContract::new(
            rpc_url,
            chain_id,
            rollup_contract_address,
            rollup_contract_deployed_block_number,
        );
        let observer = Observer::new(rollup_contract);
        let validity_processor = OnceLock::new();
        let data = Arc::new(RwLock::new(Data::new()));
        Self {
            validity_processor,
            observer,
            data,
        }
    }

    pub async fn sync_observer(&self) -> Result<(), ValidityProverError> {
        self.observer.sync().await?;
        Ok(())
    }

    pub async fn get_validity_proof(
        &self,
        block_number: u32,
    ) -> Option<ProofWithPublicInputs<F, C, D>> {
        self.data
            .read()
            .await
            .validity_proofs
            .get(&block_number)
            .cloned()
    }

    pub async fn sync(&self) -> Result<(), ValidityProverError> {
        self.sync_observer().await?; // todo: make this independent job

        // todo: make it without loading to memory
        let data = self.data.read().await;
        let last_block_number = data.last_block_number;
        let mut account_tree = data.account_trees.get(&last_block_number).unwrap().clone();
        let mut block_tree = data.block_trees.get(&last_block_number).unwrap().clone();
        let mut deposit_hash_tree = data
            .deposit_hash_trees
            .get(&last_block_number)
            .unwrap()
            .clone();

        let next_block_number = self.observer.get_next_block_number().await;
        for block_number in (last_block_number + 1)..next_block_number {
            log::info!(
                "Sync validity prover: syncing block number {}",
                block_number
            );
            let prev_validity_proof = self.get_validity_proof(block_number - 1).await;
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
            let deposit_events = self
                .observer
                .get_deposits_between_blocks(block_number)
                .await;
            for event in deposit_events {
                deposit_hash_tree.push(event.deposit_hash);
            }

            // assertion
            if full_block.block.deposit_tree_root != deposit_hash_tree.get_root() {
                return Err(ValidityProverError::DepositTreeRootMismatch(
                    full_block.block.deposit_tree_root,
                    deposit_hash_tree.get_root(),
                ));
            }

            // update self
            let mut data = self.data.write().await;
            data.last_block_number = block_number;
            data.account_trees
                .insert(block_number, account_tree.clone());
            data.block_trees.insert(block_number, block_tree.clone());
            data.deposit_hash_trees
                .insert(block_number, deposit_hash_tree.clone());
            data.validity_proofs.insert(block_number, validity_proof);
            data.sender_leaves
                .insert(block_number, block_witness.get_sender_tree().leaves());
            let tx_tree_root = full_block.signature.tx_tree_root;
            if tx_tree_root != Bytes32::default()
                && validity_witness.to_validity_pis().unwrap().is_valid_block
            {
                // even if there are duplicate tx_tree_roots, it's fine to overwrite
                data.tx_tree_roots.insert(tx_tree_root, block_number);
            }
        }
        Ok(())
    }

    // pub async fn get_update_witness(
    //     &self,
    //     pubkey: U256,
    //     root_block_number: u32,
    //     leaf_block_number: u32,
    //     is_prev_account_tree: bool,
    // ) -> Result<UpdateWitness<F, C, D>, ValidityProverError> {
    //     let validity_proof = self.get_validity_proof(root_block_number).await.ok_or(
    //         ValidityProverError::ValidityProofNotFound(root_block_number),
    //     )?;
    //     let block_merkle_proof = self
    //         .get_block_merkle_proof(root_block_number, leaf_block_number)
    //         .map_err(|e| anyhow::anyhow!("failed to get block merkle proof: {}", e))?;
    //     let account_tree_block_number = if is_prev_account_tree {
    //         root_block_number - 1
    //     } else {
    //         root_block_number
    //     };
    //     let account_membership_proof = self
    //         .get_account_membership_proof(account_tree_block_number, pubkey)
    //         .map_err(|e| anyhow::anyhow!("failed to get account membership proof: {}", e))?;
    //     Ok(UpdateWitness {
    //         is_prev_account_tree,
    //         validity_proof,
    //         block_merkle_proof,
    //         account_membership_proof,
    //     })
    // }

    // // utilities
    // pub fn get_account_id(&self, pubkey: U256) -> Option<u64> {
    //     self.account_trees
    //         .get(&self.last_block_number)
    //         .unwrap()
    //         .index(pubkey)
    // }

    // // returns deposit index and block number
    // pub fn get_deposit_index_and_block_number(&self, deposit_hash: Bytes32) -> Option<(u32, u32)> {
    //     self.deposit_correspondence.get(&deposit_hash).cloned()
    // }

    // pub fn get_block_number_by_tx_tree_root(&self, tx_tree_root: Bytes32) -> Option<u32> {
    //     self.tx_tree_roots.get(&tx_tree_root).cloned()
    // }

    // pub fn get_validity_pis(&self, block_number: u32) -> Option<ValidityPublicInputs> {
    //     self.validity_proofs
    //         .get(&block_number)
    //         .map(|proof| ValidityPublicInputs::from_pis(&proof.public_inputs))
    // }

    // pub fn get_sender_leaves(&self, block_number: u32) -> Option<Vec<SenderLeaf>> {
    //     self.sender_leaves.get(&block_number).cloned()
    // }

    // pub fn get_block_merkle_proof(
    //     &self,
    //     root_block_number: u32,
    //     leaf_block_number: u32,
    // ) -> Result<BlockHashMerkleProof, ValidityProverError> {
    //     // if leaf_block_number > root_block_number {
    //     //     return Err
    //     // }

    //     // ensure!(
    //     //     leaf_block_number <= root_block_number,
    //     //     "leaf_block_number should be smaller than root_block_number"
    //     // );
    //     let block_tree = &self
    //         .block_trees
    //         .get(&root_block_number)
    //         .ok_or(anyhow::anyhow!(
    //             "block tree not found for block number {}",
    //             root_block_number
    //         ))?;
    //     Ok(block_tree.prove(leaf_block_number as u64))
    // }

    // fn get_account_membership_proof(
    //     &self,
    //     block_number: u32,
    //     pubkey: U256,
    // ) -> anyhow::Result<AccountMembershipProof> {
    //     let account_tree = &self
    //         .account_trees
    //         .get(&block_number)
    //         .ok_or(anyhow::anyhow!(
    //             "account tree not found for block number {}",
    //             block_number
    //         ))?;
    //     Ok(account_tree.prove_membership(pubkey))
    // }

    // pub fn block_number(&self) -> u32 {
    //     self.last_block_number
    // }

    // pub fn get_deposit_merkle_proof(
    //     &self,
    //     block_number: u32,
    //     deposit_index: u32,
    // ) -> anyhow::Result<DepositMerkleProof> {
    //     let deposit_tree = &self
    //         .deposit_trees
    //         .get(&block_number)
    //         .ok_or(anyhow::anyhow!(
    //             "deposit tree not found for block number {}",
    //             block_number
    //         ))?;
    //     Ok(deposit_tree.prove(deposit_index as u64))
    // }

    pub fn validity_processor(&self) -> &ValidityProcessor<F, C, D> {
        self.validity_processor
            .get_or_init(|| ValidityProcessor::new())
    }
}
