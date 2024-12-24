use intmax2_zkp::{
    ethereum_types::u256::U256,
    utils::trees::indexed_merkle_tree::{leaf::IndexedMerkleLeaf, IndexedMerkleProof},
};

use anyhow::Result;

use crate::trees::{
    error::HistoricalIndexedMerkleTreeError,
    incremental_merkle_tree::HistoricalIncrementalMerkleTree, merkle_tree::HashOut, node::NodeDB,
};

type V = IndexedMerkleLeaf;
pub type HIMTResult<T> = Result<T, HistoricalIndexedMerkleTreeError>;

#[derive(Debug, Clone)]
pub struct HistoricalIndexedMerkleTree<DB: NodeDB<V>>(pub HistoricalIncrementalMerkleTree<V, DB>);

impl<DB: NodeDB<V>> HistoricalIndexedMerkleTree<DB> {
    pub async fn new(node_db: DB, height: u32) -> HIMTResult<Self> {
        let tree = HistoricalIncrementalMerkleTree::new(node_db, height).await?;
        tree.push(IndexedMerkleLeaf::default()).await?;
        Ok(Self(tree))
    }

    pub async fn len(&self) -> HIMTResult<u32> {
        let len = self.0.len().await?;
        Ok(len)
    }

    pub async fn get_leaf_by_root(
        &self,
        root: HashOut<V>,
        index: u64,
    ) -> HIMTResult<IndexedMerkleLeaf> {
        let leaf = self.0.get_leaf_by_root(root, index).await?;
        Ok(leaf)
    }

    pub async fn get_leaves_by_root(&self, root: HashOut<V>) -> HIMTResult<Vec<IndexedMerkleLeaf>> {
        let leaves = self.0.get_leaves_by_root(root).await?;

        Ok(leaves)
    }

    pub async fn get_current_leaf(&self, index: u64) -> HIMTResult<IndexedMerkleLeaf> {
        let leaf = self.0.get_current_leaf(index).await?;
        Ok(leaf)
    }

    pub async fn get_current_leaves(&self) -> HIMTResult<Vec<IndexedMerkleLeaf>> {
        let leaves = self.0.get_current_leaves().await?;
        Ok(leaves)
    }

    pub async fn get_current_root(&self) -> HIMTResult<HashOut<V>> {
        let root = self.0.get_current_root().await?;
        Ok(root)
    }

    pub async fn prove_by_root(
        &self,
        root: HashOut<V>,
        index: u64,
    ) -> HIMTResult<IndexedMerkleProof> {
        let proof = self.0.prove_by_root(root, index).await?;
        Ok(proof)
    }

    pub async fn low_index(&self, leaves: &[V], key: U256) -> HIMTResult<u64> {
        let low_leaf_candidates = leaves
            .into_iter()
            .enumerate()
            .filter(|(_, leaf)| {
                (leaf.key < key) && (key < leaf.next_key || leaf.next_key == U256::default())
            })
            .collect::<Vec<_>>();
        if low_leaf_candidates.is_empty() {
            return Err(HistoricalIndexedMerkleTreeError::KeyAlreadyExists(key));
        }
        if low_leaf_candidates.len() > 1 {
            return Err(HistoricalIndexedMerkleTreeError::TooManyCandidates);
        }
        let (low_leaf_index, _) = low_leaf_candidates[0];
        Ok(low_leaf_index as u64)
    }

    pub async fn index(&self, leaves: &[V], key: U256) -> HIMTResult<Option<u64>> {
        let leaf_candidates = leaves
            .into_iter()
            .enumerate()
            .filter(|(_, leaf)| leaf.key == key)
            .collect::<Vec<_>>();
        if leaf_candidates.is_empty() {
            return Ok(None);
        }
        if leaf_candidates.len() > 1 {
            return Err(HistoricalIndexedMerkleTreeError::TooManyCandidates);
        }
        let (leaf_index, _) = leaf_candidates[0];
        Ok(Some(leaf_index as u64))
    }

    pub async fn key_by_root(&self, root: HashOut<V>, index: u64) -> HIMTResult<U256> {
        let key = self.0.get_leaf_by_root(root, index).await?.key;
        Ok(key)
    }
}
