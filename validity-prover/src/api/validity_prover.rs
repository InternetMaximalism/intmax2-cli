use std::sync::{Arc, Mutex, OnceLock};

use hashbrown::HashMap;
use intmax2_client_sdk::external_api::contract::rollup_contract::{self, RollupContract};
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

use super::observer::Observer;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub struct ValidityProver {
    validity_processor: OnceLock<ValidityProcessor<F, C, D>>, // delayed initialization
    observer: Observer,

    // TODO: make these DB backed & more efficient snaphots (e.g. DB merkle tree)
    data: Arc<Mutex<Data>>,
}

pub struct Data {
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
        let data = Arc::new(Mutex::new(Data::new()));
        Self {
            validity_processor,
            observer,
            data,
        }
    }
}
