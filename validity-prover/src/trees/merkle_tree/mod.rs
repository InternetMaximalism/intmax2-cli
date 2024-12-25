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
    use intmax2_zkp::utils::trees::merkle_tree::u64_le_bits;

    use crate::trees::{merkle_tree::mock_merkle_tree::MockMerkleTree, setup_test};

    use super::sql_merkle_tree::SqlMerkleTree;
    use crate::trees::merkle_tree::MerkleTreeClient;

    type V = u32;

    #[tokio::test]
    async fn test_merkle_tree() -> anyhow::Result<()> {
        let database_url = setup_test();

        let height = 10;
        let tree = MockMerkleTree::<V>::new(height);

        let timestamp0 = 0;
        for i in 0..5 {
            tree.update_leaf(timestamp0, i, i as u32).await?;
        }
        let timestamp1 = 1;
        for i in 5..10 {
            tree.update_leaf(timestamp1, i, i as u32).await?;
        }
        tree.update_leaf(timestamp1, 3, 9).await?;

        let leaves0_m = tree.get_leaves(timestamp0).await?;
        let leaves1_m = tree.get_leaves(timestamp1).await?;
        let root0_m = tree.get_root(timestamp0).await?;
        let root1_m = tree.get_root(timestamp1).await?;
        let index = 6;
        let proof1_m = tree.prove(timestamp1, index).await?;
        let last_timestamp_m = tree.get_last_timestamp().await?;

        let timestamp = 0;
        let tree = SqlMerkleTree::<V>::new(&database_url, 0, height);
        tree.reset().await?;

        // let index_bits = u64_le_bits(6, height);
        // proof2.verify(&6, index_bits, root2)?;

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
