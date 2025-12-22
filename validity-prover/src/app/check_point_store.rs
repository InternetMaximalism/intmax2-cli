use std::fmt;

use alloy::primitives::B256;
use server_common::db::DbPool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Deposited,
    DepositLeafInserted,
    BlockPosted,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Deposited => write!(f, "Deposited"),
            EventType::DepositLeafInserted => write!(f, "DepositLeafInserted"),
            EventType::BlockPosted => write!(f, "BlockPosted"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainType {
    L1,
    L2,
}

impl EventType {
    pub fn to_chain_type(&self) -> ChainType {
        match self {
            EventType::Deposited => ChainType::L1,
            EventType::DepositLeafInserted => ChainType::L2,
            EventType::BlockPosted => ChainType::L2,
        }
    }
}

#[derive(Clone)]
pub struct CheckPointStore {
    pool: DbPool,
}

#[derive(Debug, Clone, Copy)]
pub struct CheckPoint {
    pub eth_block_number: u64,
    pub block_hash: Option<B256>,
}

impl CheckPointStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn get_check_point(
        &self,
        event_type: EventType,
    ) -> Result<Option<CheckPoint>, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT eth_block_number, block_hash FROM event_sync_eth_block WHERE event_type = $1
            "#,
            event_type.to_string()
        )
        .fetch_optional(&self.pool)
        .await?;
        let checkpoint = row.map(|row| {
            let block_hash = row.block_hash.as_deref().map(B256::from_slice);
            CheckPoint {
                eth_block_number: row.eth_block_number as u64,
                block_hash,
            }
        });
        Ok(checkpoint)
    }

    pub async fn set_check_point(
        &self,
        event_type: EventType,
        eth_block_number: u64,
        block_hash: Option<B256>,
    ) -> Result<(), sqlx::Error> {
        let block_hash = block_hash.map(|hash| hash.0.to_vec());
        sqlx::query!(
            r#"
            INSERT INTO event_sync_eth_block (event_type, eth_block_number, block_hash)
            VALUES ($1, $2, $3)
            ON CONFLICT (event_type) 
            DO UPDATE SET eth_block_number = EXCLUDED.eth_block_number,
                          block_hash = EXCLUDED.block_hash;
            "#,
            event_type.to_string(),
            eth_block_number as i64,
            block_hash
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
