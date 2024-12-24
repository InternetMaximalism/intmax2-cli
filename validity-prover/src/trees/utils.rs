use anyhow::ensure;
use intmax2_zkp::{
    common::{
        trees::{account_tree::AccountRegistrationProof, sender_tree::get_sender_leaves},
        witness::{
            block_witness::BlockWitness, full_block::FullBlock,
            validity_transition_witness::ValidityTransitionWitness,
            validity_witness::ValidityWitness,
        },
    },
    constants::{ACCOUNT_TREE_HEIGHT, NUM_SENDERS_IN_BLOCK},
    ethereum_types::{account_id_packed::AccountIdPacked, bytes32::Bytes32, u256::U256},
    utils::trees::indexed_merkle_tree::leaf::IndexedMerkleLeaf,
};

use crate::trees::{
    account_tree::HistoricalAccountTree, block_tree::HistoricalBlockHashTree, node::NodeDB,
};

pub async fn to_block_witness<ADB: NodeDB<IndexedMerkleLeaf>, BDB: NodeDB<Bytes32>>(
    full_block: &FullBlock,
    account_tree: &HistoricalAccountTree<ADB>,
    block_tree: &HistoricalBlockHashTree<BDB>,
) -> anyhow::Result<BlockWitness> {
    ensure!(
        full_block.block.block_number != 0,
        "genesis block is not allowed"
    );
    let is_registration_block = full_block.signature.is_registration_block;
    let (pubkeys, account_id_packed, account_merkle_proofs, account_membership_proofs) =
        if is_registration_block {
            let mut pubkeys = full_block.pubkeys.clone().ok_or(anyhow::anyhow!(
                "pubkeys is not given while it is registration block"
            ))?;
            pubkeys.resize(NUM_SENDERS_IN_BLOCK, U256::dummy_pubkey());
            let mut account_membership_proofs = Vec::new();
            for pubkey in pubkeys.iter() {
                let is_dummy = pubkey.is_dummy_pubkey();
                let leaves = account_tree.get_current_leaves().await?;
                ensure!(
                    account_tree.index(&leaves, *pubkey).await?.is_none() || is_dummy,
                    "account already exists"
                );
                let root = account_tree.get_current_root().await?;
                let proof = account_tree.prove_membership_by_root(root, *pubkey).await?;
                account_membership_proofs.push(proof);
            }
            (pubkeys, None, None, Some(account_membership_proofs))
        } else {
            let account_id_trimmed_bytes = full_block.account_ids.clone().ok_or(
                anyhow::anyhow!("account_ids is not given while it is non-registration block"),
            )?;
            let account_id_packed = AccountIdPacked::from_trimmed_bytes(&account_id_trimmed_bytes)
                .map_err(|e| anyhow::anyhow!("error while recovering packed account ids {}", e))?;
            let account_ids = account_id_packed.unpack();
            let mut account_merkle_proofs = Vec::new();
            let mut pubkeys = Vec::new();
            for account_id in account_ids {
                let root = account_tree.get_current_root().await?;
                let pubkey = account_tree.key_by_root(root, account_id).await?;
                let proof = account_tree
                    .prove_inclusion_by_root(root, account_id)
                    .await?;
                pubkeys.push(pubkey);
                account_merkle_proofs.push(proof);
            }
            (
                pubkeys,
                Some(account_id_packed),
                Some(account_merkle_proofs),
                None,
            )
        };
    let prev_account_tree_root = account_tree.get_current_root().await?;
    let prev_block_tree_root = block_tree.get_current_root().await?;
    let block_witness = BlockWitness {
        block: full_block.block.clone(),
        signature: full_block.signature.clone(),
        pubkeys: pubkeys.clone(),
        prev_account_tree_root,
        prev_block_tree_root,
        account_id_packed,
        account_merkle_proofs,
        account_membership_proofs,
    };
    Ok(block_witness)
}

pub async fn update_trees<ADB: NodeDB<IndexedMerkleLeaf>, BDB: NodeDB<Bytes32>>(
    block_witness: &BlockWitness,
    account_tree: &HistoricalAccountTree<ADB>,
    block_tree: &HistoricalBlockHashTree<BDB>,
) -> anyhow::Result<ValidityWitness> {
    let block_pis = block_witness.to_main_validation_pis().map_err(|e| {
        anyhow::anyhow!("failed to convert to main validation public inputs: {}", e)
    })?;
    ensure!(
        block_pis.block_number == block_tree.len().await? as u32,
        "block number mismatch"
    );

    // Update block tree
    let root = block_tree.get_current_root().await?;
    let block_merkle_proof = block_tree
        .prove_by_root(root, block_witness.block.block_number as u64)
        .await?;
    block_tree.push(block_witness.block.hash()).await?;

    // Update account tree
    let sender_leaves =
        get_sender_leaves(&block_witness.pubkeys, block_witness.signature.sender_flag);
    let account_registration_proofs = {
        if block_pis.is_valid && block_pis.is_registration_block {
            let mut account_registration_proofs = Vec::new();
            for sender_leaf in &sender_leaves {
                let last_block_number = if sender_leaf.did_return_sig {
                    block_pis.block_number
                } else {
                    0
                };
                let is_dummy_pubkey = sender_leaf.sender.is_dummy_pubkey();
                let proof = if is_dummy_pubkey {
                    AccountRegistrationProof::dummy(ACCOUNT_TREE_HEIGHT)
                } else {
                    account_tree
                        .prove_and_insert(sender_leaf.sender, last_block_number as u64)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("failed to prove and insert account_tree: {}", e)
                        })?
                };
                account_registration_proofs.push(proof);
            }
            Some(account_registration_proofs)
        } else {
            None
        }
    };

    let account_update_proofs = {
        if block_pis.is_valid && (!block_pis.is_registration_block) {
            let mut account_update_proofs = Vec::new();
            let block_number = block_pis.block_number;
            for sender_leaf in sender_leaves.iter() {
                let leaves = account_tree.get_current_leaves().await?;
                let account_id = account_tree
                    .index(&leaves, sender_leaf.sender)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("failed to get index from account_tree",))?;
                let prev_leaf = account_tree.get_current_leaf(account_id).await?;
                let prev_last_block_number = prev_leaf.value as u32;
                let last_block_number = if sender_leaf.did_return_sig {
                    block_number
                } else {
                    prev_last_block_number
                };
                let proof = account_tree
                    .prove_and_update(sender_leaf.sender, last_block_number as u64)
                    .await?;
                account_update_proofs.push(proof);
            }
            Some(account_update_proofs)
        } else {
            None
        }
    };

    let validity_transition_witness = ValidityTransitionWitness {
        sender_leaves,
        block_merkle_proof,
        account_registration_proofs,
        account_update_proofs,
    };
    Ok(ValidityWitness {
        validity_transition_witness,
        block_witness: block_witness.clone(),
    })
}
