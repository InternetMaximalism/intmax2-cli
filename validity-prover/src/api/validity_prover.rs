use std::sync::{Arc, Mutex, OnceLock};

use hashbrown::HashMap;
use intmax2_zkp::{
    circuits::validity::validity_processor::ValidityProcessor,
    common::trees::{
        account_tree::AccountTree, block_hash_tree::BlockHashTree, deposit_tree::DepositTree,
        sender_tree::SenderLeaf,
    },
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
