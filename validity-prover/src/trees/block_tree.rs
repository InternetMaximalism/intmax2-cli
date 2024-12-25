use intmax2_zkp::ethereum_types::bytes32::Bytes32;

use crate::trees::incremental_merkle_tree::HistoricalIncrementalMerkleTree;

pub type HistoricalBlockHashTree<DB> = HistoricalIncrementalMerkleTree<Bytes32, DB>;
