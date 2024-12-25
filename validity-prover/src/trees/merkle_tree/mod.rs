use async_trait::async_trait;
use error::MerkleTreeError;
use intmax2_zkp::utils::{
    leafable::Leafable, leafable_hasher::LeafableHasher, trees::merkle_tree::MerkleProof,
};
use serde::{de::DeserializeOwned, Serialize};

pub mod error;
pub mod mock_merkle_tree;
pub mod sql_merkle_tree;

pub type Hasher<V> = <V as Leafable>::LeafableHasher;
pub type HashOut<V> = <Hasher<V> as LeafableHasher>::HashOut;
pub type MTResult<T> = std::result::Result<T, MerkleTreeError>;

#[async_trait(?Send)]
pub trait MerkleTreeClient<V: Leafable + Serialize + DeserializeOwned>:
    std::fmt::Debug + Clone
{
    fn height(&self) -> usize;
    async fn update_leaf(&self, timestamp: u64, position: u64, leaf: V) -> MTResult<()>;
    async fn get_root(&self, timestamp: u64) -> MTResult<HashOut<V>>;
    async fn get_leaf(&self, timestamp: u64, position: u64) -> MTResult<V>;
    async fn get_leaves(&self, timestamp: u64) -> MTResult<Vec<V>>;
    async fn get_num_leaves(&self, timestamp: u64) -> MTResult<usize>;
    async fn prove(&self, timestamp: u64, position: u64) -> MTResult<MerkleProof<V>>;
    async fn reset(&self) -> MTResult<()>;
    async fn get_last_timestamp(&self) -> MTResult<u64>;
}

#[cfg(test)]
mod tests {
    use crate::trees::{merkle_tree::mock_merkle_tree::MockMerkleTree, setup_test};

    use super::sql_merkle_tree::SqlMerkleTree;
    use crate::trees::merkle_tree::MerkleTreeClient;

    type V = u32;

    #[tokio::test]
    async fn test_merkle_tree() -> anyhow::Result<()> {
        let database_url = setup_test();

        let height = 10;
        let tree = MockMerkleTree::<V>::new(height);

        let timestamp = 0;
        for i in 0..5 {
            tree.update_leaf(timestamp, i, i as u32).await?;
        }
        let timestamp = 2;
        for i in 5..10 {
            tree.update_leaf(timestamp, i, i as u32).await?;
        }
        tree.update_leaf(timestamp, 3, 9).await?;

        let leaves0_m = tree.get_leaves(0).await?;
        let leaves2_m = tree.get_leaves(2).await?;
        let root0_m = tree.get_root(0).await?;
        let root2_m = tree.get_root(2).await?;
        let proof2_m = tree.prove(2, 6).await?;
        let last_timestamp_m = tree.get_last_timestamp().await?;

        let timestamp = 0;
        let tree = SqlMerkleTree::<V>::new(&database_url, 0, height);
        tree.reset().await?;

        for i in 0..5 {
            tree.update_leaf(timestamp, i, i as u32).await?;
        }
        let timestamp = 2;
        for i in 5..10 {
            tree.update_leaf(timestamp, i, i as u32).await?;
        }
        tree.update_leaf(timestamp, 3, 9).await?;

        let leaves0 = tree.get_leaves(0).await?;
        let leaves2 = tree.get_leaves(2).await?;
        let root0 = tree.get_root(0).await?;
        let root2 = tree.get_root(2).await?;
        let proof2 = tree.prove(2, 6).await?;
        let timestamp = tree.get_last_timestamp().await?;

        assert_eq!(leaves0, leaves0_m);
        assert_eq!(leaves2, leaves2_m);
        assert_eq!(root0_m, root0);
        assert_eq!(root2, root2_m);
        assert_eq!(proof2.siblings, proof2_m.siblings);
        assert_eq!(timestamp, last_timestamp_m);

        Ok(())
    }

    #[tokio::test]
    async fn test_speed_merkle_tree() -> anyhow::Result<()> {
        let height = 32;
        let n = 1 << 12;

        let database_url = setup_test();
        let tree = SqlMerkleTree::<V>::new(&database_url, 0, height);
        tree.reset().await?;

        let timestamp = 0;
        let time = std::time::Instant::now();
        for i in 0..n {
            tree.update_leaf(timestamp, i, i as u32).await?;
        }
        println!(
            "SqlMerkleTree: {} leaves, {} height, {} seconds",
            n,
            height,
            time.elapsed().as_secs_f64()
        );

        let time = std::time::Instant::now();
        let leaves = tree.get_leaves(timestamp).await?;
        println!(
            "time to get all {} leaves: {:?}",
            leaves.len(),
            time.elapsed()
        );

        Ok(())
    }
}
