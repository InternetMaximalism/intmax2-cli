use intmax2_zkp::{
    common::trees::account_tree::AccountMerkleProof,
    constants::ACCOUNT_TREE_HEIGHT,
    ethereum_types::u256::U256,
    utils::trees::indexed_merkle_tree::{
        insertion::IndexedInsertionProof, leaf::IndexedMerkleLeaf, membership::MembershipProof,
        update::UpdateProof,
    },
};

use crate::trees::{
    error::HistoricalIndexedMerkleTreeError,
    indexed_merkle_tree::{HIMTResult, HistoricalIndexedMerkleTree},
    merkle_tree::HashOut,
    node::NodeDB,
};

type V = IndexedMerkleLeaf;
pub type HistoricalAccountTree<DB> = HistoricalIndexedMerkleTree<DB>;

impl<DB: NodeDB<V>> HistoricalAccountTree<DB> {
    pub fn node_db(&self) -> &DB {
        self.0.node_db()
    }

    pub async fn initialize(node_db: DB) -> HIMTResult<Self> {
        let tree = HistoricalIndexedMerkleTree::new(node_db, ACCOUNT_TREE_HEIGHT as u32).await?;
        if tree.len().await? == 1 {
            tree.insert(U256::dummy_pubkey(), 0).await?; // add default account
        }
        Ok(tree)
    }

    pub async fn prove_inclusion_by_root(
        &self,
        root: HashOut<V>,
        account_id: u64,
    ) -> HIMTResult<AccountMerkleProof> {
        let leaf = self.get_leaf_by_root(root, account_id).await?;
        let merkle_proof = self.prove_by_root(root, account_id).await?;
        Ok(AccountMerkleProof { merkle_proof, leaf })
    }

    pub async fn prove_membership_by_root(
        &self,
        root: HashOut<V>,
        key: U256,
    ) -> HIMTResult<MembershipProof> {
        let leaves = self.get_leaves_by_root(root).await?;
        if let Some(index) = self.index(&leaves, key).await? {
            // inclusion proof
            return Ok(MembershipProof {
                is_included: true,
                leaf_index: index,
                leaf: self.get_leaf_by_root(root, index).await?,
                leaf_proof: self.prove_by_root(root, index).await?,
            });
        } else {
            // exclusion proof
            let low_index = self.low_index(&leaves, key).await?;
            return Ok(MembershipProof {
                is_included: false,
                leaf_index: low_index,
                leaf: self.get_leaf_by_root(root, low_index).await?,
                leaf_proof: self.prove_by_root(root, low_index).await?,
            });
        }
    }

    pub async fn update(&self, leaves: &[V], key: U256, value: u64) -> HIMTResult<()> {
        let index = self
            .index(leaves, key)
            .await?
            .ok_or_else(|| HistoricalIndexedMerkleTreeError::KeyDoesNotExist(key))?;
        let mut leaf = self.get_current_leaf(index).await?;
        leaf.value = value;
        self.0.update(index, leaf).await?;
        Ok(())
    }

    pub async fn insert(&self, key: U256, value: u64) -> HIMTResult<()> {
        let leaves = self.0.get_current_leaves().await?;
        let index = self.len().await? as u64;
        let low_index = self.low_index(&leaves, key).await?;
        let prev_low_leaf = self.0.get_current_leaf(low_index).await?;
        let new_low_leaf = IndexedMerkleLeaf {
            next_index: index,
            next_key: key,
            ..prev_low_leaf
        };
        let leaf = IndexedMerkleLeaf {
            next_index: prev_low_leaf.next_index,
            key,
            next_key: prev_low_leaf.next_key,
            value,
        };
        self.0.update(low_index, new_low_leaf).await?;
        self.0.push(leaf).await?;
        Ok(())
    }

    pub async fn prove_and_insert(
        &self,
        key: U256,
        value: u64,
    ) -> HIMTResult<IndexedInsertionProof> {
        let leaves = self.0.get_current_leaves().await?;
        let index = self.len().await? as u64;
        let low_index = self.low_index(&leaves, key).await?;
        let prev_low_leaf = self.0.get_current_leaf(low_index).await?;
        let new_low_leaf = IndexedMerkleLeaf {
            next_index: index,
            next_key: key,
            ..prev_low_leaf
        };
        let leaf = IndexedMerkleLeaf {
            next_index: prev_low_leaf.next_index,
            key,
            next_key: prev_low_leaf.next_key,
            value,
        };

        let root = self.0.get_current_root().await?;
        let low_leaf_proof = self.0.prove_by_root(root, low_index).await?;
        self.0.update(low_index, new_low_leaf).await?;
        self.0.push(leaf).await?;

        let root = self.0.get_current_root().await?;
        let leaf_proof = self.0.prove_by_root(root, index).await?;
        Ok(IndexedInsertionProof {
            index,
            low_leaf_proof,
            leaf_proof,
            low_leaf_index: low_index,
            prev_low_leaf,
        })
    }

    pub async fn prove_and_update(&self, key: U256, new_value: u64) -> HIMTResult<UpdateProof> {
        let leaves = self.get_current_leaves().await?;
        let index = self
            .index(&leaves, key)
            .await?
            .ok_or_else(|| HistoricalIndexedMerkleTreeError::KeyDoesNotExist(key))?;
        let prev_leaf = self.get_current_leaf(index).await?;
        let new_leaf = IndexedMerkleLeaf {
            value: new_value,
            ..prev_leaf
        };
        self.0.update(index, new_leaf).await?;
        let root = self.0.get_current_root().await?;
        Ok(UpdateProof {
            leaf_proof: self.prove_by_root(root, index).await?,
            leaf_index: index,
            prev_leaf,
        })
    }
}

#[cfg(test)]
mod tests {
    use intmax2_zkp::utils::trees::indexed_merkle_tree::leaf::IndexedMerkleLeaf;

    use crate::trees::{
        account_tree::HistoricalAccountTree,
        node::{NodeDB as _, SqlNodeDB},
    };

    #[tokio::test]
    async fn test_account_tree() -> anyhow::Result<()> {
        let database_url = crate::trees::setup_test();

        let tag = 4;
        let node_db = SqlNodeDB::<IndexedMerkleLeaf>::new(&database_url, tag).await?;
        node_db.reset().await?;
        // let node_db = crate::node::MockNodeDB::new();

        let account_tree = HistoricalAccountTree::initialize(node_db).await?;

        for i in 2..5 {
            account_tree.insert(i.into(), i.into()).await?;
        }
        let old_root = account_tree.get_current_root().await?;
        let old_leaves = account_tree.get_current_leaves().await?;
        for i in 5..8 {
            account_tree.insert(i.into(), i.into()).await?;
        }
        let leaves = account_tree.get_leaves_by_root(old_root).await?;
        assert_eq!(leaves, old_leaves);

        let account_id = 3;
        let proof = account_tree
            .prove_inclusion_by_root(old_root, account_id)
            .await?;
        let result = proof.verify(old_root, account_id, (account_id as u32).into());
        assert!(result);

        Ok(())
    }
}
