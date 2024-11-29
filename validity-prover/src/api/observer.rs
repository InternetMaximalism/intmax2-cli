use intmax2_client_sdk::external_api::contract::rollup_contract::{
    FullBlockWithMeta, RollupContract,
};
use intmax2_interfaces::api::validity_prover::interface::DepositInfo;
use intmax2_zkp::{common::witness::full_block::FullBlock, ethereum_types::bytes32::Bytes32};
use sqlx::{postgres::PgPoolOptions, PgPool, Result as SqlxResult};

use super::error::ObserverError;

pub struct Observer {
    rollup_contract: RollupContract,
    pool: PgPool,
}

impl Observer {
    pub async fn new(rollup_contract: RollupContract, database_url: &str) -> SqlxResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
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

            sqlx::query!(
                "INSERT INTO full_blocks (block_number, eth_block_number, eth_tx_index, full_block) 
                 VALUES ($1, $2, $3, $4)",
                0i32, // genesis block number
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

    pub async fn sync_eth_block_number(&self) -> SqlxResult<Option<u64>> {
        let result = sqlx::query!("SELECT sync_eth_block_number FROM sync_state WHERE id = 1")
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.and_then(|r| r.sync_eth_block_number.map(|n| n as u64)))
    }

    pub async fn get_next_block_number(&self) -> SqlxResult<u32> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM full_blocks")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.count.unwrap_or(0) as u32)
    }

    pub async fn get_next_deposit_index(&self) -> SqlxResult<u32> {
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
            Some(r) => serde_json::from_value(r.full_block)
                .map_err(|e| ObserverError::DeserializationError(e.to_string()))?,
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

    pub async fn get_deposit_info(&self, deposit_hash: Bytes32) -> SqlxResult<Option<DepositInfo>> {
        let event = sqlx::query!(
            r#"
            SELECT deposit_index, eth_block_number, eth_tx_index 
            FROM deposit_leaf_events 
            WHERE deposit_hash = $1
            "#,
            deposit_hash.as_bytes()
        )
        .fetch_optional(&self.pool)
        .await?;

        let event = match event {
            Some(e) => e,
            None => return Ok(None),
        };

        let block = sqlx::query!(
            r#"
            SELECT full_block, block_number
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

    // 残りのメソッドも同様にDB操作に変換します...
}
