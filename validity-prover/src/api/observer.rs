use intmax2_client_sdk::external_api::contract::rollup_contract::{
    DepositLeafInserted, FullBlockWithMeta, RollupContract,
};
use intmax2_interfaces::api::validity_prover::interface::DepositInfo;
use intmax2_zkp::{
    common::witness::full_block::FullBlock,
    ethereum_types::{bytes32::Bytes32, u32limb_trait::U32LimbTrait},
};
use sqlx::{postgres::PgPoolOptions, PgPool};

use super::error::ObserverError;

pub struct Observer {
    rollup_contract: RollupContract,
    pool: PgPool,
}

impl Observer {
    pub async fn new(
        rollup_contract: RollupContract,
        database_url: &str,
        database_max_connections: u32,
        database_timeout: u64,
    ) -> Result<Self, ObserverError> {
        let pool = PgPoolOptions::new()
            .max_connections(database_max_connections)
            .idle_timeout(std::time::Duration::from_secs(database_timeout))
            .connect(database_url)
            .await?;

        // Initialize with genesis block if table is empty
        let count = sqlx::query!("SELECT COUNT(*) as count FROM full_blocks")
            .fetch_one(&pool)
            .await?
            .count
            .unwrap_or(0);

        if count == 0 {
            let genesis = FullBlockWithMeta {
                full_block: FullBlock::genesis(),
                eth_block_number: 0,
                eth_tx_index: 0,
            };

            // Initialize sync_state
            sqlx::query!(
                "INSERT INTO sync_state (id, sync_eth_block_number) VALUES (1, $1)",
                rollup_contract.deployed_block_number as i64
            )
            .execute(&pool)
            .await?;

            // Insert genesis block
            sqlx::query!(
                "INSERT INTO full_blocks (block_number, eth_block_number, eth_tx_index, full_block) 
                 VALUES ($1, $2, $3, $4)",
                0i32,
                genesis.eth_block_number as i64,
                genesis.eth_tx_index as i64,
                serde_json::to_value(&genesis.full_block).unwrap()
            )
            .execute(&pool)
            .await?;
        }

        Ok(Observer {
            rollup_contract,
            pool,
        })
    }

    pub async fn sync_eth_block_number(&self) -> Result<Option<u64>, ObserverError> {
        let result = sqlx::query!("SELECT sync_eth_block_number FROM sync_state WHERE id = 1")
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.and_then(|r| r.sync_eth_block_number.map(|n| n as u64)))
    }

    pub async fn get_next_block_number(&self) -> Result<u32, ObserverError> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM full_blocks")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0) as u32)
    }

    pub async fn get_next_deposit_index(&self) -> Result<u32, ObserverError> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM deposit_leaf_events")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0) as u32)
    }

    pub async fn get_full_block(&self, block_number: u32) -> Result<FullBlock, ObserverError> {
        let record = sqlx::query!(
            "SELECT full_block FROM full_blocks WHERE block_number = $1",
            block_number as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        let full_block: FullBlock = match record {
            Some(r) => serde_json::from_value(r.full_block)?,
            None => return Err(ObserverError::BlockNotFound(block_number)),
        };

        if full_block.block.block_number != block_number {
            return Err(ObserverError::BlockNumberMismatch(
                full_block.block.block_number,
                block_number,
            ));
        }

        Ok(full_block)
    }

    pub async fn get_deposit_info(
        &self,
        deposit_hash: Bytes32,
    ) -> Result<Option<DepositInfo>, ObserverError> {
        let event = sqlx::query!(
            r#"
            SELECT deposit_index, eth_block_number, eth_tx_index 
            FROM deposit_leaf_events 
            WHERE deposit_hash = $1
            "#,
            deposit_hash.to_bytes_be()
        )
        .fetch_optional(&self.pool)
        .await?;

        let event = match event {
            Some(e) => e,
            None => return Ok(None),
        };

        let block = sqlx::query!(
            r#"
            SELECT block_number
            FROM full_blocks 
            WHERE (eth_block_number, eth_tx_index) > ($1, $2)
            ORDER BY eth_block_number, eth_tx_index
            LIMIT 1
            "#,
            event.eth_block_number,
            event.eth_tx_index
        )
        .fetch_optional(&self.pool)
        .await?;

        match block {
            Some(b) => Ok(Some(DepositInfo {
                deposit_hash,
                block_number: b.block_number as u32,
                deposit_index: event.deposit_index as u32,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_full_blocks_from(
        &self,
        from_block_number: u32,
    ) -> Result<Vec<FullBlock>, ObserverError> {
        let records = sqlx::query!(
            "SELECT full_block FROM full_blocks WHERE block_number >= $1 ORDER BY block_number",
            from_block_number as i32
        )
        .fetch_all(&self.pool)
        .await?;

        let blocks = records
            .into_iter()
            .map(|r| serde_json::from_value(r.full_block))
            .collect::<Result<Vec<FullBlock>, _>>()?;

        Ok(blocks)
    }

    pub async fn get_full_block_with_meta(
        &self,
        block_number: u32,
    ) -> Result<Option<FullBlockWithMeta>, ObserverError> {
        let record = sqlx::query!(
            "SELECT eth_block_number, eth_tx_index, full_block 
             FROM full_blocks 
             WHERE block_number = $1",
            block_number as i32
        )
        .fetch_optional(&self.pool)
        .await?;

        match record {
            Some(r) => {
                let full_block: FullBlock = serde_json::from_value(r.full_block)?;

                Ok(Some(FullBlockWithMeta {
                    full_block,
                    eth_block_number: r.eth_block_number as u64,
                    eth_tx_index: r.eth_tx_index as u64,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_deposits_between_blocks(
        &self,
        block_number: u32,
    ) -> Result<Vec<DepositLeafInserted>, ObserverError> {
        let current_block = self.get_full_block_with_meta(block_number).await?;
        let prev_block = self
            .get_full_block_with_meta(block_number.saturating_sub(1))
            .await?;

        let (prev_block, current_block) = match (prev_block, current_block) {
            (Some(p), Some(c)) => (p, c),
            _ => return Ok(Vec::new()),
        };

        let deposits = sqlx::query!(
            r#"
            SELECT deposit_index, deposit_hash, eth_block_number, eth_tx_index
            FROM deposit_leaf_events
            WHERE (eth_block_number, eth_tx_index) > ($1, $2)
            AND (eth_block_number, eth_tx_index) <= ($3, $4)
            ORDER BY deposit_index
            "#,
            prev_block.eth_block_number as i64,
            prev_block.eth_tx_index as i64,
            current_block.eth_block_number as i64,
            current_block.eth_tx_index as i64,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(deposits
            .into_iter()
            .map(|d| DepositLeafInserted {
                deposit_index: d.deposit_index as u32,
                deposit_hash: Bytes32::from_bytes_be(&d.deposit_hash),
                eth_block_number: d.eth_block_number as u64,
                eth_tx_index: d.eth_tx_index as u64,
            })
            .collect())
    }

    pub async fn sync(&self) -> Result<(), ObserverError> {
        let sync_eth_block_number = self.sync_eth_block_number().await?;
        log::info!("Syncing from eth block number: {:?}", sync_eth_block_number);

        // Get full blocks and validate
        let full_blocks = self
            .rollup_contract
            .get_full_block_with_meta(sync_eth_block_number)
            .await
            .map_err(|e| ObserverError::FullBlockSyncError(e.to_string()))?;

        let next_block_number = self.get_next_block_number().await?;

        if let Some(first) = full_blocks.first() {
            if first.full_block.block.block_number != next_block_number {
                return Err(ObserverError::FullBlockSyncError(format!(
                    "First block mismatch: {} != {}",
                    first.full_block.block.block_number, next_block_number
                )));
            }
        }

        // Get deposit leaf events and validate
        let deposit_leaf_events = self
            .rollup_contract
            .get_deposit_leaf_inserted_events(sync_eth_block_number)
            .await
            .map_err(|e| ObserverError::FullBlockSyncError(e.to_string()))?;

        let next_deposit_index = self.get_next_deposit_index().await?;

        if let Some(first) = deposit_leaf_events.first() {
            if first.deposit_index != next_deposit_index {
                return Err(ObserverError::FullBlockSyncError(format!(
                    "First deposit index mismatch: {} != {}",
                    first.deposit_index, next_deposit_index
                )));
            }
        }

        // Calculate new sync block number
        let new_sync_eth_block_number = {
            let last_full_block_eth_block_number = full_blocks.last().map(|fb| fb.eth_block_number);
            let last_deposit_event = deposit_leaf_events.last().map(|dle| dle.eth_block_number);
            let candidate = vec![last_full_block_eth_block_number, last_deposit_event]
                .into_iter()
                .flatten()
                .max();
            if candidate.is_some() {
                candidate.map(|x| x + 1) // next block
            } else {
                sync_eth_block_number
            }
        };

        // Begin transaction
        let mut tx = self.pool.begin().await?;

        // Insert full blocks
        for block in &full_blocks {
            sqlx::query!(
                "INSERT INTO full_blocks (block_number, eth_block_number, eth_tx_index, full_block) 
                 VALUES ($1, $2, $3, $4)",
                block.full_block.block.block_number as i32,
                block.eth_block_number as i64,
                block.eth_tx_index as i64,
                serde_json::to_value(&block.full_block).unwrap()
            )
            .execute(&mut *tx)
            .await?;
        }

        // Insert deposit events
        for event in &deposit_leaf_events {
            sqlx::query!(
                "INSERT INTO deposit_leaf_events (deposit_index, deposit_hash, eth_block_number, eth_tx_index) 
                 VALUES ($1, $2, $3, $4)",
                event.deposit_index as i32,
                event.deposit_hash.to_bytes_be(),
                event.eth_block_number as i64,
                event.eth_tx_index as i64
            )
            .execute(&mut *tx)
            .await?;
        }

        // Update sync state
        if let Some(new_block_number) = new_sync_eth_block_number {
            sqlx::query!(
                "UPDATE sync_state SET sync_eth_block_number = $1 WHERE id = 1",
                new_block_number as i64
            )
            .execute(&mut *tx)
            .await?;
        }

        // Commit transaction
        tx.commit().await?;

        if let Some(last_block) = full_blocks.last() {
            log::info!(
                "Observer synced to block number: {}, deposit index: {}",
                last_block.full_block.block.block_number,
                deposit_leaf_events
                    .last()
                    .map(|e| e.deposit_index)
                    .unwrap_or(0)
            );
        } else {
            log::info!(
                "Observer synced to deposit index: {}",
                deposit_leaf_events
                    .last()
                    .map(|e| e.deposit_index)
                    .unwrap_or(0)
            );
        }

        Ok(())
    }
}
