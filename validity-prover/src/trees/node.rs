use crate::trees::{bit_path::BitPath, error::NodeDBError};
use async_trait::async_trait;
use hashbrown::HashMap;
use intmax2_zkp::utils::{leafable::Leafable, leafable_hasher::LeafableHasher};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::Arc;
use tokio::sync::RwLock;

pub type NodeDBResult<T> = Result<T, NodeDBError>;
type Hasher<V> = <V as Leafable>::LeafableHasher;
type HashOut<V> = <Hasher<V> as LeafableHasher>::HashOut;

#[derive(Clone, Debug)]
pub struct Node<V: Leafable> {
    pub left_hash: HashOut<V>,
    pub right_hash: HashOut<V>,
}

#[async_trait(?Send)]
pub trait NodeDB<V: Leafable + Serialize + DeserializeOwned>: std::fmt::Debug + Clone {
    async fn insert_node(&self, parent_hash: HashOut<V>, node: Node<V>) -> NodeDBResult<()>;
    async fn get_node(&self, parent_hash: HashOut<V>) -> NodeDBResult<Option<Node<V>>>;
    async fn insert_current_node_hash(
        &self,
        bit_path: BitPath,
        hash: HashOut<V>,
    ) -> NodeDBResult<()>;
    async fn get_current_node_hash(&self, bit_path: BitPath) -> NodeDBResult<Option<HashOut<V>>>;

    async fn insert_leaf_hash(&self, position: u64, leaf_hash: HashOut<V>) -> NodeDBResult<()>;
    async fn insert_leaf(&self, leaf: V) -> NodeDBResult<()>;
    async fn num_leaf_hashes(&self) -> NodeDBResult<u32>;
    async fn get_leaf_hash(&self, position: u64) -> NodeDBResult<Option<HashOut<V>>>;
    async fn get_leaf_by_hash(&self, hash: HashOut<V>) -> NodeDBResult<Option<V>>;
    async fn get_all_leaf_hashes(&self) -> NodeDBResult<Vec<(u64, HashOut<V>)>>;
    async fn reset(&self) -> NodeDBResult<()>;
}

#[derive(Clone, Debug)]
pub struct MockNodeDB<V: Leafable> {
    current_node_hashes: Arc<RwLock<HashMap<BitPath, HashOut<V>>>>,
    nodes: Arc<RwLock<HashMap<HashOut<V>, Node<V>>>>,
    leaf_hashes: Arc<RwLock<HashMap<u64, HashOut<V>>>>,
    leaves: Arc<RwLock<HashMap<HashOut<V>, V>>>,
}

impl<V: Leafable> MockNodeDB<V> {
    pub fn new() -> Self {
        MockNodeDB {
            current_node_hashes: Arc::new(RwLock::new(HashMap::new())),
            nodes: Arc::new(RwLock::new(HashMap::new())),
            leaf_hashes: Arc::new(RwLock::new(HashMap::new())),
            leaves: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait(?Send)]
impl<V: Leafable + Serialize + DeserializeOwned> NodeDB<V> for MockNodeDB<V> {
    async fn insert_node(&self, parent_hash: HashOut<V>, node: Node<V>) -> NodeDBResult<()> {
        self.nodes.write().await.insert(parent_hash, node);
        Ok(())
    }

    async fn get_node(&self, parent_hash: HashOut<V>) -> NodeDBResult<Option<Node<V>>> {
        Ok(self.nodes.read().await.get(&parent_hash).cloned())
    }

    async fn insert_current_node_hash(
        &self,
        bit_path: BitPath,
        hash: HashOut<V>,
    ) -> NodeDBResult<()> {
        self.current_node_hashes
            .write()
            .await
            .insert(bit_path, hash);
        Ok(())
    }

    async fn get_current_node_hash(&self, bit_path: BitPath) -> NodeDBResult<Option<HashOut<V>>> {
        Ok(self
            .current_node_hashes
            .read()
            .await
            .get(&bit_path)
            .cloned())
    }

    async fn insert_leaf_hash(&self, position: u64, leaf_hash: HashOut<V>) -> NodeDBResult<()> {
        self.leaf_hashes.write().await.insert(position, leaf_hash);
        Ok(())
    }

    async fn insert_leaf(&self, leaf: V) -> NodeDBResult<()> {
        self.leaves.write().await.insert(leaf.hash(), leaf);
        Ok(())
    }

    async fn num_leaf_hashes(&self) -> NodeDBResult<u32> {
        Ok(self.leaf_hashes.read().await.len() as u32)
    }

    async fn get_leaf_hash(&self, position: u64) -> NodeDBResult<Option<HashOut<V>>> {
        Ok(self.leaf_hashes.read().await.get(&position).cloned())
    }

    async fn get_leaf_by_hash(&self, hash: HashOut<V>) -> NodeDBResult<Option<V>> {
        Ok(self.leaves.read().await.get(&hash).cloned())
    }

    async fn get_all_leaf_hashes(&self) -> NodeDBResult<Vec<(u64, HashOut<V>)>> {
        let mut leaves: Vec<(u64, HashOut<V>)> = self
            .leaf_hashes
            .read()
            .await
            .iter()
            .map(|(position, leaf_hash)| (*position, *leaf_hash))
            .collect();
        leaves.sort_by_key(|(position, _)| *position);
        Ok(leaves)
    }

    async fn reset(&self) -> NodeDBResult<()> {
        self.nodes.write().await.clear();
        self.leaf_hashes.write().await.clear();
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SqlNodeDB<V: Leafable + Serialize + DeserializeOwned> {
    tag: u32, // tag is used to distinguish between different trees in the same database
    pool: Pool<Postgres>,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: Leafable + Serialize + DeserializeOwned> SqlNodeDB<V> {
    pub async fn new(db_url: &str, tag: u32) -> NodeDBResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(db_url)
            .await?;
        Ok(SqlNodeDB {
            tag,
            pool,
            _phantom: std::marker::PhantomData,
        })
    }
}

#[async_trait(?Send)]
impl<V: Leafable + Serialize + DeserializeOwned> NodeDB<V> for SqlNodeDB<V> {
    async fn insert_node(&self, parent_hash: HashOut<V>, node: Node<V>) -> NodeDBResult<()> {
        let serialized_parent = bincode::serialize(&parent_hash)?;
        let serialized_left = bincode::serialize(&node.left_hash)?;
        let serialized_right = bincode::serialize(&node.right_hash)?;

        sqlx::query!(
            r#"
            INSERT INTO hash_nodes (tag, parent_hash, left_hash, right_hash)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (tag, parent_hash) DO NOTHING
            "#,
            self.tag as i32,
            serialized_parent as _,
            serialized_left as _,
            serialized_right as _
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_node(&self, parent_hash: HashOut<V>) -> NodeDBResult<Option<Node<V>>> {
        let serialized_hash = bincode::serialize(&parent_hash)?;

        let row = sqlx::query!(
            r#"
            SELECT left_hash, right_hash
            FROM hash_nodes
            WHERE parent_hash = $1 AND tag = $2
            "#,
            serialized_hash as _,
            self.tag as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let left_hash = bincode::deserialize(&row.left_hash)?;
                let right_hash = bincode::deserialize(&row.right_hash)?;

                Ok(Some(Node {
                    left_hash,
                    right_hash,
                }))
            }
            None => Ok(None),
        }
    }

    async fn insert_current_node_hash(
        &self,
        bit_path: BitPath,
        hash: HashOut<V>,
    ) -> NodeDBResult<()> {
        let serialized_bit_path = bincode::serialize(&bit_path)?;
        let serialized_hash = bincode::serialize(&hash)?;
        sqlx::query!(
            r#"
            INSERT INTO current_node_hashes (tag, bit_path, hash_value)
            VALUES ($1, $2, $3)
            ON CONFLICT (tag, bit_path) DO UPDATE 
            SET hash_value = EXCLUDED.hash_value  
            "#,
            self.tag as i32,
            serialized_bit_path as _,
            serialized_hash as _
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_current_node_hash(&self, bit_path: BitPath) -> NodeDBResult<Option<HashOut<V>>> {
        let serialized_bit_path = bincode::serialize(&bit_path)?;
        let row = sqlx::query!(
            r#"
            SELECT hash_value
            FROM current_node_hashes
            WHERE bit_path = $1 AND tag = $2
            "#,
            serialized_bit_path as _,
            self.tag as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let hash = bincode::deserialize(&row.hash_value)?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    async fn insert_leaf_hash(&self, position: u64, leaf_hash: HashOut<V>) -> NodeDBResult<()> {
        let serialized_hash = bincode::serialize(&leaf_hash)?;

        sqlx::query!(
            r#"
            INSERT INTO current_leaf_hashes (tag, position, leaf_hash)
            VALUES ($1, $2, $3)
            ON CONFLICT (tag, position) 
            DO UPDATE SET leaf_hash = EXCLUDED.leaf_hash
            "#,
            self.tag as i32,
            position as i64,
            serialized_hash as _
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_leaf(&self, leaf: V) -> NodeDBResult<()> {
        let hash = leaf.hash();
        let serialized_hash = bincode::serialize(&hash)?;
        let serialized_leaf = bincode::serialize(&leaf)?;

        sqlx::query!(
            r#"
            INSERT INTO leaves (tag, leaf_hash, leaf)
            VALUES ($1, $2, $3)
            ON CONFLICT (tag, leaf_hash) DO NOTHING
            "#,
            self.tag as i32,
            serialized_hash as _,
            serialized_leaf as _
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn num_leaf_hashes(&self) -> NodeDBResult<u32> {
        let row = sqlx::query!(
            r#"
            SELECT COUNT(*)
            FROM current_leaf_hashes
            WHERE tag = $1
            "#,
            self.tag as i32
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn get_leaf_hash(&self, position: u64) -> NodeDBResult<Option<HashOut<V>>> {
        let row = sqlx::query!(
            r#"
            SELECT leaf_hash
            FROM current_leaf_hashes
            WHERE position = $1 AND tag = $2
            "#,
            position as i64,
            self.tag as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let hash = bincode::deserialize(&row.leaf_hash)?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    async fn get_leaf_by_hash(&self, hash: HashOut<V>) -> NodeDBResult<Option<V>> {
        let serialized_hash = bincode::serialize(&hash)?;

        let row = sqlx::query!(
            r#"
            SELECT leaf
            FROM leaves
            WHERE leaf_hash = $1 AND tag = $2
            "#,
            serialized_hash as _,
            self.tag as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let leaf = bincode::deserialize(&row.leaf)?;
                Ok(Some(leaf))
            }
            None => Ok(None),
        }
    }

    async fn get_all_leaf_hashes(&self) -> NodeDBResult<Vec<(u64, HashOut<V>)>> {
        let time = std::time::Instant::now();
        let rows = sqlx::query!(
            r#"
            SELECT position, leaf_hash
            FROM current_leaf_hashes 
            WHERE tag = $1
            ORDER BY position
            "#,
            self.tag as i32
        )
        .fetch_all(&self.pool)
        .await?;

        let mut leaf_hashes = Vec::new();
        for row in rows {
            let hash = bincode::deserialize(&row.leaf_hash)?;
            leaf_hashes.push((row.position as u64, hash));
        }
        leaf_hashes.sort_by_key(|(position, _)| *position);
        tracing::info!("get_all_leaf_hashes took {:?}", time.elapsed());
        Ok(leaf_hashes)
    }

    async fn reset(&self) -> NodeDBResult<()> {
        sqlx::query!("DELETE FROM hash_nodes WHERE tag = $1", self.tag as i32)
            .execute(&self.pool)
            .await?;

        sqlx::query!(
            "DELETE FROM current_node_hashes WHERE tag = $1",
            self.tag as i32
        )
        .execute(&self.pool)
        .await?;

        sqlx::query!(
            "DELETE FROM current_leaf_hashes WHERE tag = $1",
            self.tag as i32
        )
        .execute(&self.pool)
        .await?;

        sqlx::query!("DELETE FROM leaves WHERE tag = $1", self.tag as i32)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
