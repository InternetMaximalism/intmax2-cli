use std::sync::{Arc, OnceLock};

use hashbrown::HashMap;
use intmax2_client_sdk::external_api::contract::rollup_contract::RollupContract;
use intmax2_zkp::{
    circuits::validity::validity_processor::ValidityProcessor,
    common::{
        block::Block,
        trees::{
            account_tree::AccountTree, block_hash_tree::BlockHashTree, deposit_tree::DepositTree,
            sender_tree::SenderLeaf,
        },
    },
    constants::{BLOCK_HASH_TREE_HEIGHT, DEPOSIT_TREE_HEIGHT},
    ethereum_types::bytes32::Bytes32,
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use tokio::sync::RwLock;

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
    deposit_trees: HashMap<u32, DepositTree>,
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

        let deposit_tree = DepositTree::new(DEPOSIT_TREE_HEIGHT);
        let mut deposit_trees = HashMap::new();
        deposit_trees.insert(last_block_number, deposit_tree);

        let mut sender_leaves = HashMap::new();
        sender_leaves.insert(last_block_number, vec![]);

        Self {
            last_block_number,
            validity_proofs: HashMap::new(),
            account_trees,
            block_trees,
            deposit_trees,
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
        let mut deposit_tree = data.deposit_trees.get(&last_block_number).unwrap().clone();

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
            let deposits = self
                .observer
                .get_deposits_between_blocks(block_number)
                .await;
            for deposit in deposits {
                deposit_tree.push(deposit);
            }

            // update self
            let mut data = self.data.write().await;
            data.last_block_number = block_number;
            data.account_trees
                .insert(block_number, account_tree.clone());
            data.block_trees.insert(block_number, block_tree.clone());
            data.validity_proofs.insert(block_number, validity_proof);
            data.sender_leaves
                .insert(block_number, block_witness.get_sender_tree().leaves());

            let tx_tree_root = full_block.signature.tx_tree_root;
            if tx_tree_root != Bytes32::default()
                && validity_witness.to_validity_pis().unwrap().is_valid_block
            {
                data.tx_tree_roots.insert(tx_tree_root, block_number);
            }
        }
        data.deposit_trees = contract.deposit_trees.clone();

        Ok(())
    }

    pub fn validity_processor(&self) -> &ValidityProcessor<F, C, D> {
        self.validity_processor
            .get_or_init(|| ValidityProcessor::new())
    }
}
