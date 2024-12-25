use intmax2_zkp::utils::{
    leafable::Leafable, trees::incremental_merkle_tree::IncrementalMerkleProof,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::trees::merkle_tree::{HashOut, MTResult, MerkleTreeClient};

#[derive(Debug, Clone)]
pub struct HistoricalIncrementalMerkleTree<
    V: Leafable + Serialize + DeserializeOwned,
    DB: MerkleTreeClient<V>,
> {
    merkle_tree: DB,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: Leafable + Serialize + DeserializeOwned, DB: MerkleTreeClient<V>>
    HistoricalIncrementalMerkleTree<V, DB>
{
    pub fn new(merkle_tree: DB) -> Self {
        HistoricalIncrementalMerkleTree {
            merkle_tree,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn height(&self) -> usize {
        self.merkle_tree.height()
    }

    pub async fn len(&self, timestamp: u64) -> MTResult<usize> {
        let len = self.merkle_tree.get_num_leaves(timestamp).await?;
        Ok(len)
    }

    pub async fn update(&self, timestamp: u64, index: u64, leaf: V) -> MTResult<()> {
        self.merkle_tree.update_leaf(timestamp, index, leaf).await?;
        Ok(())
    }

    pub async fn push(&self, timestamp: u64, leaf: V) -> MTResult<()> {
        let index = self.len(timestamp).await? as u64;
        self.merkle_tree.update_leaf(timestamp, index, leaf).await?;
        Ok(())
    }

    pub async fn get_root(&self, timestamp: u64) -> MTResult<HashOut<V>> {
        let root = self.merkle_tree.get_root(timestamp).await?;
        Ok(root)
    }

    pub async fn get_leaves(&self, timestamp: u64) -> MTResult<Vec<V>> {
        let leaves = self.merkle_tree.get_leaves(timestamp).await?;
        Ok(leaves)
    }

    pub async fn get_leaf(&self, timestamp: u64, index: u64) -> MTResult<V> {
        let leaf = self.merkle_tree.get_leaf(timestamp, index).await?;
        Ok(leaf)
    }

    pub async fn prove(&self, timestamp: u64, index: u64) -> MTResult<IncrementalMerkleProof<V>> {
        let proof = self.merkle_tree.prove(timestamp, index).await?;
        Ok(IncrementalMerkleProof(proof))
    }

    pub async fn get_last_timestamp(&self) -> MTResult<u64> {
        let timestamp = self.merkle_tree.get_last_timestamp().await?;
        Ok(timestamp)
    }
}

#[cfg(test)]
mod tests {
    use intmax2_zkp::utils::trees::incremental_merkle_tree::IncrementalMerkleTree;

    use crate::trees::{
        incremental_merkle_tree::HistoricalIncrementalMerkleTree,
        merkle_tree::{
            mock_merkle_tree::MockMerkleTree, sql_merkle_tree::SqlMerkleTree, MerkleTreeClient,
        },
    };

    #[tokio::test]
    async fn merkle_tree_with_leaves() -> anyhow::Result<()> {
        let height = 32;
        let database_url = crate::trees::setup_test();

        let tag = 1;

        type V = u32;

        let db = SqlMerkleTree::<V>::new(&database_url, tag, height);
        db.reset().await?;
        let db = MockMerkleTree::<V>::new(height);
        let db_tree = HistoricalIncrementalMerkleTree::new(db);

        let timestamp = db_tree.get_last_timestamp().await?;
        dbg!(timestamp);
        for i in 0..5 {
            db_tree.push(timestamp, i as u32).await?;
        }
        let root_db = db_tree.get_root(timestamp).await?;

        let mut tree = IncrementalMerkleTree::<V>::new(height);
        for i in 0..5 {
            tree.push(i);
        }
        let root = tree.get_root();
        assert_eq!(root_db, root);

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
