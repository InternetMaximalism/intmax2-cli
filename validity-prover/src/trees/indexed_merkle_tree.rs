use intmax2_zkp::{
    ethereum_types::u256::U256,
    utils::{
        poseidon_hash_out::PoseidonHashOut,
        trees::indexed_merkle_tree::{leaf::IndexedMerkleLeaf, IndexedMerkleProof},
    },
};

use crate::trees::{
    incremental_merkle_tree::HistoricalIncrementalMerkleTree, merkle_tree::MerkleTreeClient,
};
use anyhow::{ensure, Result};

type V = IndexedMerkleLeaf;

#[derive(Debug, Clone)]
pub struct HistoricalIndexedMerkleTree<DB: MerkleTreeClient<V>>(
    pub HistoricalIncrementalMerkleTree<V, DB>,
);

impl<DB: MerkleTreeClient<V>> HistoricalIndexedMerkleTree<DB> {
    pub async fn get_root(&self, timestamp: u64) -> Result<PoseidonHashOut> {
        let root = self.0.get_root(timestamp).await?;
        Ok(root)
    }

    pub async fn get_leaf(&self, timestamp: u64, index: u64) -> Result<IndexedMerkleLeaf> {
        let leaf = self.0.get_leaf(timestamp, index).await?;
        Ok(leaf)
    }

    pub async fn prove(&self, timestamp: u64, index: u64) -> Result<IndexedMerkleProof> {
        let proof = self.0.prove(timestamp, index).await?;
        Ok(proof)
    }

    pub async fn low_index(&self, timestamp: u64, key: U256) -> Result<u64> {
        let low_leaf_candidates = self
            .0
            .get_leaves(timestamp)
            .await?
            .into_iter()
            .enumerate()
            .filter(|(_, leaf)| {
                (leaf.key < key) && (key < leaf.next_key || leaf.next_key == U256::default())
            })
            .collect::<Vec<_>>();
        ensure!(0 < low_leaf_candidates.len(), "key already exists");
        ensure!(
            low_leaf_candidates.len() == 1,
            "low_index: too many candidates"
        );
        let (low_leaf_index, _) = low_leaf_candidates[0];
        Ok(low_leaf_index as u64)
    }

    pub async fn index(&self, timestamp: u64, key: U256) -> Result<Option<u64>> {
        let leaf_candidates = self
            .0
            .get_leaves(timestamp)
            .await?
            .into_iter()
            .enumerate()
            .filter(|(_, leaf)| leaf.key == key)
            .collect::<Vec<_>>();
        if leaf_candidates.is_empty() {
            return Ok(None);
        }
        assert!(
            leaf_candidates.len() == 1,
            "find_index: too many candidates"
        );
        let (leaf_index, _) = leaf_candidates[0];
        Ok(Some(leaf_index as u64))
    }

    pub async fn key(&self, timestamp: u64, index: u64) -> Result<U256> {
        let key = self.0.get_leaf(timestamp, index).await?.key;
        Ok(key)
    }

    pub async fn update(&self, timestamp: u64, key: U256, value: u64) -> Result<()> {
        let index = self
            .index(timestamp, key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Error: key doesn't exist"))?;
        let mut leaf = self.0.get_leaf(timestamp, index).await?;
        leaf.value = value;
        self.0.update(timestamp, index, leaf).await?;
        Ok(())
    }

    pub async fn len(&self, timestamp: u64) -> Result<usize> {
        let len = self.0.len(timestamp).await?;
        Ok(len)
    }
}
