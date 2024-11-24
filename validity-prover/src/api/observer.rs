use std::sync::Arc;

use intmax2_client_sdk::external_api::contract::{
    interface::BlockchainError,
    rollup_contract::{DepositLeafInserted, FullBlockWithMeta, RollupContract},
};
use intmax2_interfaces::api::validity_prover::interface::DepositInfo;
use intmax2_zkp::{common::witness::full_block::FullBlock, ethereum_types::bytes32::Bytes32};
use tokio::sync::RwLock;

pub struct Observer {
    rollup_contract: RollupContract,

    // TODO: make these DB backed
    data: Arc<RwLock<Data>>,
}

#[derive(Debug, Default)]
struct Data {
    sync_eth_block_number: Option<u64>,
    full_blocks: Vec<FullBlockWithMeta>,
    deposit_leaf_events: Vec<DepositLeafInserted>,
}

#[derive(Debug, thiserror::Error)]
pub enum ObserverError {
    #[error("Blockchain error: {0}")]
    BlockchainError(#[from] BlockchainError),

    #[error("Block not found: {0}")]
    BlockNotFound(u32),
}

impl Observer {
    pub fn new(rollup_contract: RollupContract) -> Self {
        Observer {
            rollup_contract,
            data: Arc::new(RwLock::new(Data::default())),
        }
    }

    pub async fn sync_eth_block_number(&self) -> Option<u64> {
        self.data.read().await.sync_eth_block_number.clone()
    }

    pub async fn get_next_block_number(&self) -> u32 {
        self.data.read().await.full_blocks.len() as u32
    }

    pub async fn get_full_block(&self, block_number: u32) -> Result<FullBlock, ObserverError> {
        self.data
            .read()
            .await
            .full_blocks
            .get(block_number as usize)
            .cloned()
            .map(|fm| fm.full_block)
            .ok_or(ObserverError::BlockNotFound(block_number))
    }

    /// Get the DepositInfo corresponding to the given deposit_hash.
    pub async fn get_deposit_info(&self, deposit_hash: Bytes32) -> Option<DepositInfo> {
        let event = self
            .data
            .read()
            .await
            .deposit_leaf_events
            .iter()
            .find(|deposit| deposit.deposit_hash == deposit_hash)
            .cloned();
        let event = if let Some(e) = event {
            e
        } else {
            return None;
        };
        let is_after = |a: (u64, u64), b: (u64, u64)| a.0 > b.0 || (a.0 == b.0 && a.1 > b.1);
        let deposit_time = (event.eth_block_number, event.eth_tx_index);

        let block = self
            .data
            .read()
            .await
            .full_blocks
            .iter()
            .filter(|block| {
                let block_time = (block.eth_block_number, block.eth_tx_index);
                is_after(block_time, deposit_time)
            })
            .min_by_key(|block| (block.eth_block_number, block.eth_tx_index))
            .cloned();
        let block = if let Some(b) = block {
            b
        } else {
            return None;
        };
        Some(DepositInfo {
            deposit_hash,
            block_number: block.full_block.block.block_number,
            deposit_index: event.deposit_index,
        })
    }

    /// Get the FullBlocks that were newly added from the specified block number.
    pub async fn get_full_blocks_from(&self, from_block_number: u32) -> Vec<FullBlock> {
        self.data
            .read()
            .await
            .full_blocks
            .iter()
            .map(|full_block_with_meta| full_block_with_meta.full_block.clone())
            .filter(|full_block| full_block.block.block_number >= from_block_number)
            .collect()
    }

    pub async fn get_full_block_with_meta(&self, block_number: u32) -> Option<FullBlockWithMeta> {
        self.data
            .read()
            .await
            .full_blocks
            .iter()
            .find(|full_block_with_meta| {
                full_block_with_meta.full_block.block.block_number == block_number
            })
            .cloned()
    }

    /// Get the DepositLeafInserted events that were newly added between the previous block and the current block.
    pub async fn get_deposits_between_blocks(&self, block_number: u32) -> Vec<DepositLeafInserted> {
        // Find the target block and its previous block
        let current_block = self.get_full_block_with_meta(block_number).await;
        let prev_block = self
            .get_full_block_with_meta(block_number.saturating_sub(1))
            .await;

        // Early return if either block is not found
        let (prev_block, current_block) = match (prev_block, current_block) {
            (Some(p), Some(c)) => (p, c),
            _ => return Vec::new(),
        };

        // Helper function to compare temporal order of events
        let is_after = |a: (u64, u64), b: (u64, u64)| a.0 > b.0 || (a.0 == b.0 && a.1 > b.1);

        self.data
            .read()
            .await
            .deposit_leaf_events
            .iter()
            .filter(|deposit| {
                let deposit_time = (deposit.eth_block_number, deposit.eth_tx_index);
                let prev_time = (prev_block.eth_block_number, prev_block.eth_tx_index);
                let current_time = (current_block.eth_block_number, current_block.eth_tx_index);

                is_after(deposit_time, prev_time) && !is_after(deposit_time, current_time)
            })
            .cloned()
            .collect()
    }

    pub async fn sync(&self) -> Result<(), ObserverError> {
        let current_eth_block_number = self.rollup_contract.get_block_number().await?;
        let sync_eth_block_number = self.sync_eth_block_number().await;

        let full_blocks = self
            .rollup_contract
            .get_full_block_with_meta(sync_eth_block_number)
            .await?;
        let deposit_leaf_events = self
            .rollup_contract
            .get_deposit_leaf_inserted_events(sync_eth_block_number)
            .await?;

        self.data.write().await.full_blocks.extend(full_blocks);
        self.data
            .write()
            .await
            .deposit_leaf_events
            .extend(deposit_leaf_events);
        self.data
            .write()
            .await
            .sync_eth_block_number
            .replace(current_eth_block_number);
        Ok(())
    }
}
