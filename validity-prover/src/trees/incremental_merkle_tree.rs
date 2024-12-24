use intmax2_zkp::utils::{
    leafable::Leafable, trees::incremental_merkle_tree::IncrementalMerkleProof,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::trees::{
    error::HistoricalMerkleTreeError,
    merkle_tree::{HMTResult, HashOut, HistoricalMerkleTree},
    node::NodeDB,
};

#[derive(Debug, Clone)]
pub struct HistoricalIncrementalMerkleTree<
    V: Leafable + Serialize + DeserializeOwned,
    DB: NodeDB<V>,
>(HistoricalMerkleTree<V, DB>);

impl<V: Leafable + Serialize + DeserializeOwned, DB: NodeDB<V>>
    HistoricalIncrementalMerkleTree<V, DB>
{
    pub async fn new(node_db: DB, height: u32) -> HMTResult<Self> {
        let merkle_tree = HistoricalMerkleTree::new(node_db, height).await?;
        merkle_tree.node_db().insert_leaf(V::empty_leaf()).await?;
        Ok(Self(merkle_tree))
    }

    pub fn height(&self) -> u32 {
        self.0.height()
    }

    fn node_db(&self) -> &DB {
        self.0.node_db()
    }

    pub async fn get_current_root(&self) -> HMTResult<HashOut<V>> {
        self.0.get_current_root().await
    }

    pub async fn len(&self) -> HMTResult<u32> {
        let len = self.node_db().num_leaf_hashes().await?;
        Ok(len)
    }

    pub async fn is_empty(&self) -> HMTResult<bool> {
        let len = self.len().await?;
        Ok(len == 0)
    }

    pub async fn update(&self, index: u64, leaf: V) -> HMTResult<()> {
        tracing::info!(
            "update leaf at index: {} with hash: {:?}",
            index,
            leaf.hash()
        );
        self.0.update_leaf(index, leaf.hash()).await?;
        self.node_db().insert_leaf(leaf).await?;
        Ok(())
    }

    pub async fn push(&self, leaf: V) -> HMTResult<()> {
        let index = self.len().await? as u64;
        self.0.update_leaf(index, leaf.hash()).await?;
        self.node_db().insert_leaf(leaf).await?;
        Ok(())
    }

    pub async fn get_leaf_by_root(&self, root: HashOut<V>, index: u64) -> HMTResult<V> {
        let leaf_hash = self.0.get_leaf_hash_by_root(root, index).await?;
        let leaf = self.node_db().get_leaf_by_hash(leaf_hash).await?.ok_or(
            HistoricalMerkleTreeError::LeafNotFoundError(format!("{:?}", leaf_hash)),
        )?;
        Ok(leaf)
    }

    /// Collect all leaves till the empty leaf is reached.
    pub async fn get_leaves_by_root(&self, root: HashOut<V>) -> HMTResult<Vec<V>> {
        let empty_leaf_hash = V::empty_leaf().hash();
        let mut index = 0;
        let mut leaves = vec![];
        loop {
            let leaf_hash = self.0.get_leaf_hash_by_root(root, index).await?;
            if leaf_hash == empty_leaf_hash {
                break;
            }
            let leaf = self.node_db().get_leaf_by_hash(leaf_hash).await?.ok_or(
                HistoricalMerkleTreeError::LeafNotFoundError(format!("{:?}", leaf_hash)),
            )?;
            leaves.push(leaf);
            index += 1;
        }
        Ok(leaves)
    }

    pub async fn get_current_leaf(&self, index: u64) -> HMTResult<V> {
        let leaf_hash = self.node_db().get_leaf_hash(index).await?;
        match leaf_hash {
            Some(leaf_hash) => {
                let leaf = self.node_db().get_leaf_by_hash(leaf_hash).await?.ok_or(
                    HistoricalMerkleTreeError::LeafNotFoundError(format!("{:?}", leaf_hash)),
                )?;
                Ok(leaf)
            }
            None => Err(HistoricalMerkleTreeError::LeafNotFoundError(format!(
                "{:?}",
                index
            ))),
        }
    }

    pub async fn get_current_leaves(&self) -> HMTResult<Vec<V>> {
        let leaf_hashes = self.node_db().get_all_leaf_hashes().await?;
        let mut leaves = vec![];
        for (_, leaf_hash) in leaf_hashes {
            let leaf = self.node_db().get_leaf_by_hash(leaf_hash).await?.ok_or(
                HistoricalMerkleTreeError::LeafNotFoundError(format!("{:?}", leaf_hash)),
            )?;
            leaves.push(leaf);
        }
        Ok(leaves)
    }

    pub async fn prove_by_root(
        &self,
        root: HashOut<V>,
        index: u64,
    ) -> HMTResult<IncrementalMerkleProof<V>> {
        let (proof, _) = self.0.prove_by_root(root, index).await?;
        Ok(IncrementalMerkleProof(proof))
    }
}

#[cfg(test)]
mod tests {
    use crate::trees::{
        incremental_merkle_tree::HistoricalIncrementalMerkleTree,
        node::{NodeDB as _, SqlNodeDB},
    };

    #[tokio::test]
    async fn merkle_tree_with_leaves() -> anyhow::Result<()> {
        let height = 32;
        let database_url = crate::trees::setup_test();

        let tag = 1;

        type V = u32;
        let node_db = SqlNodeDB::<V>::new(&database_url, tag).await?;
        node_db.reset().await?;
        let tree = HistoricalIncrementalMerkleTree::new(node_db, height).await?;

        for _ in 0..5 {
            let index = tree.len().await?;
            tree.push(index).await?;
        }
        let root = tree.get_current_root().await?;
        for _ in 0..5 {
            let index = tree.len().await?;
            tree.push(index).await?;
        }
        println!("start getting all current leaves");
        // let time = std::time::Instant::now();
        let leaves = tree.get_current_leaves().await?;
        dbg!(&leaves);

        let old_leaves = tree.get_leaves_by_root(root).await?;
        dbg!(&old_leaves);
        // println!(
        //     "Time to get all current {} leaves: {:?}",
        //     leaves.len(),
        //     time.elapsed()
        // );

        // for _ in 0..100 {
        //     let index = rng.gen_range(0..1 << height);
        //     let leaf = tree.get_leaf_by_root(root, index).await?;
        //     let proof = tree.prove_by_root(root, index).await?;
        //     proof.verify(&leaf, index, root).unwrap();
        // }

        // println!("start getting all leaves");
        // let time = std::time::Instant::now();
        // let leaves = tree.get_leaves_by_root(root).await?;
        // println!(
        //     "Time to get all {} leaves: {:?}",
        //     leaves.len(),
        //     time.elapsed()
        // );

        // println!("start getting all current leaves");
        // let time = std::time::Instant::now();
        // let leaves = tree.get_current_leaves().await?;
        // println!(
        //     "Time to get all current {} leaves: {:?}",
        //     leaves.len(),
        //     time.elapsed()
        // );

        Ok(())
    }
}
