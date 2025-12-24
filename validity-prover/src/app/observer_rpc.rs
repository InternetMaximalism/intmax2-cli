use super::{
    check_point_store::{ChainType, CheckPoint, CheckPointStore, EventType},
    error::ObserverError,
    leader_election::LeaderElection,
    observer_api::ObserverApi,
    observer_common::{ObserverConfig, SyncEvent},
    rate_manager::RateManager,
    validity_prover::{ACCOUNT_DB_TAG, BLOCK_DB_TAG, DEPOSIT_DB_TAG},
};
use crate::{
    app::observer_common::{initialize_observer_db, sync_event_key},
    trees::{
        deposit_hash::DepositHash,
        merkle_tree::{
            sql_incremental_merkle_tree::SqlIncrementalMerkleTree,
            sql_indexed_merkle_tree::SqlIndexedMerkleTree, IncrementalMerkleTreeClient,
            IndexedMerkleTreeClient,
        },
    },
    EnvVar,
};
use alloy::{eips::BlockNumberOrTag, providers::Provider};
use intmax2_client_sdk::external_api::contract::{
    liquidity_contract::LiquidityContract, rollup_contract::RollupContract,
};
use intmax2_zkp::{
    constants::{ACCOUNT_TREE_HEIGHT, BLOCK_HASH_TREE_HEIGHT, DEPOSIT_TREE_HEIGHT},
    ethereum_types::{bytes32::Bytes32, u32limb_trait::U32LimbTrait as _},
    utils::leafable::Leafable as _,
};
use log::warn;
use server_common::db::{DbPool, DbPoolConfig};
use tracing::{debug, info, instrument};

#[derive(Clone)]
pub struct RPCObserver {
    pub config: ObserverConfig,
    pub rollup_contract: RollupContract,
    pub liquidity_contract: LiquidityContract,
    pub observer_api: ObserverApi,
    pub check_point_store: CheckPointStore,
    pub leader_election: LeaderElection,
    pub rate_manager: RateManager,
    pub pool: DbPool,
}

impl RPCObserver {
    pub async fn new(
        env: &EnvVar,
        observer_api: ObserverApi,
        leader_election: LeaderElection,
        rate_manager: RateManager,
    ) -> Result<Self, ObserverError> {
        let config = ObserverConfig::from_env(env);
        tracing::info!("Observer config: {:?}", config);
        let pool = DbPool::from_config(&DbPoolConfig {
            max_connections: env.database_max_connections,
            idle_timeout: env.database_timeout,
            url: env.database_url.to_string(),
        })
        .await?;
        let check_point_store = CheckPointStore::new(pool.clone());
        initialize_observer_db(pool.clone()).await?;

        Ok(RPCObserver {
            config,
            rollup_contract: observer_api.rollup_contract.clone(),
            liquidity_contract: observer_api.liquidity_contract.clone(),
            observer_api,
            check_point_store,
            leader_election,
            rate_manager,
            pool,
        })
    }

    fn default_eth_block_number(&self, event_type: EventType) -> u64 {
        match event_type.to_chain_type() {
            ChainType::L1 => self.config.liquidity_contract_deployed_block_number,
            ChainType::L2 => self.config.rollup_contract_deployed_block_number,
        }
    }

    async fn get_current_eth_block_number(
        &self,
        event_type: EventType,
    ) -> Result<u64, ObserverError> {
        let current_eth_block_number = match event_type.to_chain_type() {
            ChainType::L1 => self.liquidity_contract.provider.get_block_number().await?,
            ChainType::L2 => self.rollup_contract.provider.get_block_number().await?,
        };
        Ok(current_eth_block_number)
    }

    async fn get_eth_block_hash(
        &self,
        event_type: EventType,
        block_number: u64,
    ) -> Result<alloy::primitives::B256, ObserverError> {
        let block = match event_type.to_chain_type() {
            ChainType::L1 => {
                self.liquidity_contract
                    .provider
                    .get_block_by_number(BlockNumberOrTag::Number(block_number))
                    .await?
            }
            ChainType::L2 => {
                self.rollup_contract
                    .provider
                    .get_block_by_number(BlockNumberOrTag::Number(block_number))
                    .await?
            }
        };
        let block = block.ok_or_else(|| {
            ObserverError::EventFetchError(format!(
                "Block not found for block number {block_number}"
            ))
        })?;
        Ok(block.header.hash)
    }

    async fn rewind_events(
        &self,
        event_type: EventType,
        from_eth_block_number: u64,
    ) -> Result<(), ObserverError> {
        match event_type {
            EventType::Deposited => {
                sqlx::query!(
                    "DELETE FROM deposited_events WHERE eth_block_number >= $1",
                    from_eth_block_number as i64
                )
                .execute(&self.pool)
                .await?;
            }
            EventType::DepositLeafInserted => {
                sqlx::query!(
                    "DELETE FROM deposit_leaf_events WHERE eth_block_number >= $1",
                    from_eth_block_number as i64
                )
                .execute(&self.pool)
                .await?;
            }
            EventType::BlockPosted => {
                sqlx::query!(
                    "DELETE FROM full_blocks WHERE eth_block_number >= $1",
                    from_eth_block_number as i64
                )
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn rewind_validity_state(&self, from_eth_block_number: u64) -> Result<(), ObserverError> {
        let min_block_number = sqlx::query_scalar!(
            "SELECT MIN(block_number) FROM full_blocks WHERE eth_block_number >= $1",
            from_eth_block_number as i64
        )
        .fetch_one(&self.pool)
        .await?;
        let Some(min_block_number) = min_block_number else {
            return Ok(());
        };
        let min_block_number = min_block_number as u64;
        let reset_block_number = min_block_number.max(1);
        warn!(
            "Reorg detected, rewinding validity state from block {min_block_number}, resetting merkle trees from {reset_block_number}",
        );
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "DELETE FROM validity_state WHERE block_number >= $1",
            min_block_number as i32
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "DELETE FROM tx_tree_roots WHERE block_number >= $1",
            min_block_number as i32
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "DELETE FROM validity_proofs WHERE block_number >= $1",
            min_block_number as i32
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        let pool = self.pool.raw_pool().clone();
        let account_tree =
            SqlIndexedMerkleTree::new(pool.clone(), ACCOUNT_DB_TAG, ACCOUNT_TREE_HEIGHT);
        let block_tree = SqlIncrementalMerkleTree::<Bytes32>::new(
            pool.clone(),
            BLOCK_DB_TAG,
            BLOCK_HASH_TREE_HEIGHT,
        );
        let deposit_hash_tree =
            SqlIncrementalMerkleTree::<DepositHash>::new(pool, DEPOSIT_DB_TAG, DEPOSIT_TREE_HEIGHT);
        account_tree.reset(reset_block_number).await?;
        block_tree.reset(reset_block_number).await?;
        deposit_hash_tree.reset(reset_block_number).await?;
        Ok(())
    }

    async fn ensure_checkpoint_consistent(
        &self,
        event_type: EventType,
        checkpoint: Option<CheckPoint>,
    ) -> Result<Option<CheckPoint>, ObserverError> {
        let Some(checkpoint) = checkpoint else {
            return Ok(None);
        };
        let Some(expected_hash) = checkpoint.block_hash else {
            return Ok(Some(checkpoint));
        };
        let current_hash = self
            .get_eth_block_hash(event_type, checkpoint.eth_block_number)
            .await?;
        if current_hash == expected_hash {
            return Ok(Some(checkpoint));
        }

        let rewind_block_number = checkpoint
            .eth_block_number
            .saturating_sub(1)
            .max(self.default_eth_block_number(event_type));
        warn!(
            "Checkpoint mismatch detected for {event_type}, rewinding from block {rewind_block_number}",
        );
        if event_type.to_chain_type() == ChainType::L2 {
            self.rewind_validity_state(rewind_block_number).await?;
        }
        // Rewind one extra block so the previous block is reprocessed.
        self.rewind_events(event_type, rewind_block_number).await?;
        let rewind_hash = self
            .get_eth_block_hash(event_type, rewind_block_number)
            .await?;
        let rewound = CheckPoint {
            eth_block_number: rewind_block_number,
            block_hash: Some(rewind_hash),
        };
        self.check_point_store
            .set_check_point(event_type, rewound.eth_block_number, rewound.block_hash)
            .await?;
        Ok(Some(rewound))
    }

    #[instrument(skip(self))]
    async fn fetch_and_write_deposit_leaf_inserted_events_legacy(
        &self,
        expected_next_event_id: u64,
        from_eth_block_number: u64,
        to_eth_block_number: u64,
    ) -> Result<u64, ObserverError> {
        let events = self
            .rollup_contract
            .get_deposit_leaf_inserted_events(from_eth_block_number, to_eth_block_number)
            .await
            .map_err(|e| ObserverError::EventFetchError(e.to_string()))?;
        let events = events
            .into_iter()
            .skip_while(|e| e.deposit_index < expected_next_event_id as u32)
            .collect::<Vec<_>>();
        if events.is_empty() {
            return Ok(expected_next_event_id);
        }
        let first = events.first().unwrap();
        if first.deposit_index != expected_next_event_id as u32 {
            return Err(ObserverError::EventGapDetected {
                event_type: EventType::DepositLeafInserted,
                expected_next_event_id,
                got_event_id: first.deposit_index as u64,
            });
        }

        // sequence check
        {
            let mut next_event_id = expected_next_event_id;
            for event in &events {
                if event.deposit_index as u64 != next_event_id {
                    return Err(ObserverError::EventFetchError(format!(
                        "Event sequence error. Deposited: Expected: {}, Got: {}",
                        next_event_id, event.deposit_index
                    )));
                }
                next_event_id += 1;
            }
        }

        let deposit_leaf_with_block_numbers = self
            .rollup_contract
            .get_deposit_leaf_inserted_with_block_number_events(
                from_eth_block_number,
                to_eth_block_number,
            )
            .await
            .map_err(|e| ObserverError::EventFetchError(e.to_string()))?;
        let next_block_number_by_deposit_index = deposit_leaf_with_block_numbers
            .into_iter()
            .map(|event| (event.deposit_index, event.next_block_number))
            .collect::<std::collections::HashMap<_, _>>();

        let mut tx = self.pool.begin().await?;
        for event in &events {
            let next_block_number = next_block_number_by_deposit_index
                .get(&event.deposit_index)
                .copied()
                .map(|v| v as i32);
            sqlx::query!(
            "INSERT INTO deposit_leaf_events (deposit_index, deposit_hash, eth_block_number, eth_tx_index, next_block_number) 
            VALUES ($1, $2, $3, $4, $5)",
            event.deposit_index as i32,
            event.deposit_hash.to_bytes_be(),
            event.eth_block_number as i64,
            event.eth_tx_index as i64,
            next_block_number
            )
            .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let next_event_id = events.last().unwrap().deposit_index as u64 + 1;
        Ok(next_event_id)
    }

    #[instrument(skip(self))]
    async fn fetch_and_write_deposit_leaf_with_block_number_events(
        &self,
        expected_next_event_id: u64,
        from_eth_block_number: u64,
        to_eth_block_number: u64,
    ) -> Result<u64, ObserverError> {
        let events = self
            .rollup_contract
            .get_deposit_leaf_inserted_with_block_number_events(
                from_eth_block_number,
                to_eth_block_number,
            )
            .await
            .map_err(|e| ObserverError::EventFetchError(e.to_string()))?;
        let events = events
            .into_iter()
            .skip_while(|e| e.deposit_index < expected_next_event_id as u32)
            .collect::<Vec<_>>();
        if events.is_empty() {
            return Ok(expected_next_event_id);
        }
        let first = events.first().unwrap();
        if first.deposit_index != expected_next_event_id as u32 {
            return Err(ObserverError::EventGapDetected {
                event_type: EventType::DepositLeafInserted,
                expected_next_event_id,
                got_event_id: first.deposit_index as u64,
            });
        }

        // sequence check
        {
            let mut next_event_id = expected_next_event_id;
            for event in &events {
                if event.deposit_index as u64 != next_event_id {
                    return Err(ObserverError::EventFetchError(format!(
                        "Event sequence error. Deposited: Expected: {}, Got: {}",
                        next_event_id, event.deposit_index
                    )));
                }
                next_event_id += 1;
            }
        }

        let mut tx = self.pool.begin().await?;
        for event in &events {
            sqlx::query!(
            "INSERT INTO deposit_leaf_events (deposit_index, deposit_hash, eth_block_number, eth_tx_index, next_block_number) 
            VALUES ($1, $2, $3, $4, $5)",
            event.deposit_index as i32,
            event.deposit_hash.to_bytes_be(),
            event.eth_block_number as i64,
            event.eth_tx_index as i64,
            event.next_block_number as i32
            )
            .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let next_event_id = events.last().unwrap().deposit_index as u64 + 1;
        Ok(next_event_id)
    }

    #[instrument(skip(self))]
    async fn fetch_and_write_deposit_leaf_inserted_events(
        &self,
        expected_next_event_id: u64,
        from_eth_block_number: u64,
        to_eth_block_number: u64,
    ) -> Result<u64, ObserverError> {
        let switch_block = self.config.rollup_contract_event_upgrade_block_number;
        let Some(switch_block) = switch_block else {
            return self
                .fetch_and_write_deposit_leaf_inserted_events_legacy(
                    expected_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await;
        };

        if to_eth_block_number < switch_block {
            return self
                .fetch_and_write_deposit_leaf_inserted_events_legacy(
                    expected_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await;
        }
        if from_eth_block_number >= switch_block {
            return self
                .fetch_and_write_deposit_leaf_with_block_number_events(
                    expected_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await;
        }

        let legacy_to = switch_block.saturating_sub(1);
        let next_event_id = if from_eth_block_number <= legacy_to {
            self.fetch_and_write_deposit_leaf_inserted_events_legacy(
                expected_next_event_id,
                from_eth_block_number,
                legacy_to,
            )
            .await?
        } else {
            expected_next_event_id
        };
        self.fetch_and_write_deposit_leaf_with_block_number_events(
            next_event_id,
            switch_block,
            to_eth_block_number,
        )
        .await
    }

    #[instrument(skip(self))]
    async fn fetch_and_write_deposited_events(
        &self,
        expected_next_event_id: u64,
        from_eth_block_number: u64,
        to_eth_block_number: u64,
    ) -> Result<u64, ObserverError> {
        let events = self
            .liquidity_contract
            .get_deposited_events(from_eth_block_number, to_eth_block_number)
            .await
            .map_err(|e| ObserverError::EventFetchError(e.to_string()))?;
        let events = events
            .into_iter()
            .skip_while(|e| e.deposit_id < expected_next_event_id)
            .collect::<Vec<_>>();
        if events.is_empty() {
            return Ok(expected_next_event_id);
        }
        let first = events.first().unwrap();
        if first.deposit_id != expected_next_event_id {
            return Err(ObserverError::EventGapDetected {
                event_type: EventType::Deposited,
                expected_next_event_id,
                got_event_id: first.deposit_id,
            });
        }

        // sequence check
        {
            let mut next_event_id = expected_next_event_id;
            for event in &events {
                if event.deposit_id != next_event_id {
                    return Err(ObserverError::EventFetchError(format!(
                        "Event sequence error. Deposited: Expected: {}, Got: {}",
                        next_event_id, event.deposit_id
                    )));
                }
                next_event_id += 1;
            }
        }

        let mut tx = self.pool.begin().await?;
        for event in &events {
            let deposit_hash = event.to_deposit().hash();
            sqlx::query!(
                "INSERT INTO deposited_events (deposit_id, depositor, pubkey_salt_hash, token_index, amount, is_eligible, deposited_at, deposit_hash, tx_hash, eth_block_number, eth_tx_index) 
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                event.deposit_id as i64,
                event.depositor.to_hex(),
                event.pubkey_salt_hash.to_hex(),
                event.token_index as i64,
                event.amount.to_hex(),
                event.is_eligible,
                event.deposited_at as i64,
                deposit_hash.to_hex(),
                event.tx_hash.to_hex(),
                event.eth_block_number as i64,
                event.eth_tx_index as i64
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        let next_event_id = events.last().unwrap().deposit_id + 1;
        Ok(next_event_id)
    }

    #[instrument(skip(self))]
    async fn fetch_and_write_block_posted_events_legacy(
        &self,
        expected_next_event_id: u64,
        from_eth_block_number: u64,
        to_eth_block_number: u64,
    ) -> Result<u64, ObserverError> {
        let events = self
            .rollup_contract
            .get_blocks_posted_event(from_eth_block_number, to_eth_block_number)
            .await
            .map_err(|e| ObserverError::EventFetchError(e.to_string()))?;
        let events = events
            .into_iter()
            .skip_while(|b| b.block_number < expected_next_event_id as u32)
            .collect::<Vec<_>>();
        if events.is_empty() {
            return Ok(expected_next_event_id);
        }
        let first = events.first().unwrap();
        if first.block_number != expected_next_event_id as u32 {
            return Err(ObserverError::EventGapDetected {
                event_type: EventType::BlockPosted,
                expected_next_event_id,
                got_event_id: first.block_number as u64,
            });
        }

        // sequence check
        {
            let mut next_event_id = expected_next_event_id;
            for event in &events {
                if event.block_number as u64 != next_event_id {
                    return Err(ObserverError::EventFetchError(format!(
                        "Event sequence error. Block posted: Expected: {}, Got: {}",
                        next_event_id, event.block_number
                    )));
                }
                next_event_id += 1;
            }
        }

        // fetch full block
        let full_block_with_meta = self
            .rollup_contract
            .get_full_block_with_meta(&events)
            .await?;
        let mut tx = self.pool.begin().await?;
        for event in &full_block_with_meta {
            sqlx::query!(
                "INSERT INTO full_blocks (block_number, eth_block_number, eth_tx_index, full_block) 
                 VALUES ($1, $2, $3, $4)",
                event.full_block.block.block_number as i32,
                event.eth_block_number as i64,
                event.eth_tx_index as i64,
                bincode::serialize(&event.full_block).unwrap()
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        let next_event_id = events.last().unwrap().block_number + 1;
        Ok(next_event_id as u64)
    }

    #[instrument(skip(self))]
    async fn fetch_and_write_full_block_posted_events(
        &self,
        expected_next_event_id: u64,
        from_eth_block_number: u64,
        to_eth_block_number: u64,
    ) -> Result<u64, ObserverError> {
        let events = self
            .rollup_contract
            .get_full_block_posted_events(from_eth_block_number, to_eth_block_number)
            .await
            .map_err(|e| ObserverError::EventFetchError(e.to_string()))?;
        let events = events
            .into_iter()
            .skip_while(|b| b.full_block.block.block_number < expected_next_event_id as u32)
            .collect::<Vec<_>>();
        if events.is_empty() {
            return Ok(expected_next_event_id);
        }
        let first = events.first().unwrap();
        if first.full_block.block.block_number != expected_next_event_id as u32 {
            return Err(ObserverError::EventGapDetected {
                event_type: EventType::BlockPosted,
                expected_next_event_id,
                got_event_id: first.full_block.block.block_number as u64,
            });
        }

        // sequence check
        {
            let mut next_event_id = expected_next_event_id;
            for event in &events {
                if event.full_block.block.block_number as u64 != next_event_id {
                    return Err(ObserverError::EventFetchError(format!(
                        "Event sequence error. Block posted: Expected: {}, Got: {}",
                        next_event_id, event.full_block.block.block_number
                    )));
                }
                next_event_id += 1;
            }
        }

        let mut tx = self.pool.begin().await?;
        for event in &events {
            sqlx::query!(
                "INSERT INTO full_blocks (block_number, eth_block_number, eth_tx_index, full_block) 
                 VALUES ($1, $2, $3, $4)",
                event.full_block.block.block_number as i32,
                event.eth_block_number as i64,
                event.eth_tx_index as i64,
                bincode::serialize(&event.full_block).unwrap()
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        let next_event_id = events.last().unwrap().full_block.block.block_number + 1;
        Ok(next_event_id as u64)
    }

    #[instrument(skip(self))]
    async fn fetch_and_write_block_posted_events(
        &self,
        expected_next_event_id: u64,
        from_eth_block_number: u64,
        to_eth_block_number: u64,
    ) -> Result<u64, ObserverError> {
        let switch_block = self.config.rollup_contract_event_upgrade_block_number;
        let Some(switch_block) = switch_block else {
            return self
                .fetch_and_write_block_posted_events_legacy(
                    expected_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await;
        };

        if to_eth_block_number < switch_block {
            return self
                .fetch_and_write_block_posted_events_legacy(
                    expected_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await;
        }
        if from_eth_block_number >= switch_block {
            return self
                .fetch_and_write_full_block_posted_events(
                    expected_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await;
        }

        let legacy_to = switch_block.saturating_sub(1);
        let next_event_id = if from_eth_block_number <= legacy_to {
            self.fetch_and_write_block_posted_events_legacy(
                expected_next_event_id,
                from_eth_block_number,
                legacy_to,
            )
            .await?
        } else {
            expected_next_event_id
        };
        self.fetch_and_write_full_block_posted_events(
            next_event_id,
            switch_block,
            to_eth_block_number,
        )
        .await
    }

    #[instrument(skip(self))]
    async fn reset_check_point(
        &self,
        event_type: EventType,
        local_last_eth_block_number: Option<u64>,
        reason: &str,
    ) -> Result<(), ObserverError> {
        let reset_eth_block_number =
            local_last_eth_block_number.unwrap_or(self.default_eth_block_number(event_type));
        let reset_block_hash = self
            .get_eth_block_hash(event_type, reset_eth_block_number)
            .await?;
        warn!(
            "Reset checkpoint. Event type: {event_type}, Local last eth block number: {local_last_eth_block_number:?}, Reset eth block number: {reset_eth_block_number}, Reason: {reason}"
        );
        self.check_point_store
            .set_check_point(event_type, reset_eth_block_number, Some(reset_block_hash))
            .await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn sync_and_save_checkpoint(
        &self,
        event_type: EventType,
        onchain_next_event_id: u64,
        local_next_event_id: u64,
    ) -> Result<u64, ObserverError> {
        self.leader_election.wait_for_leadership().await?;
        let checkpoint = self.check_point_store.get_check_point(event_type).await?;
        let checkpoint = self
            .ensure_checkpoint_consistent(event_type, checkpoint)
            .await?;
        let local_last_eth_block_number = self
            .observer_api
            .get_local_last_eth_block_number(event_type)
            .await?;
        let checkpoint_eth_block_number = checkpoint.map(|checkpoint| checkpoint.eth_block_number);
        let from_eth_block_number = checkpoint_eth_block_number
            .max(local_last_eth_block_number)
            .unwrap_or(self.default_eth_block_number(event_type));
        tracing::info!(
            "checkpoint eth block number: {:?}, local last eth block number: {:?}, from eth block number: {:?}",
            checkpoint_eth_block_number,
            local_last_eth_block_number,
            from_eth_block_number
        );
        let current_eth_block_number = self.get_current_eth_block_number(event_type).await?;
        if from_eth_block_number > current_eth_block_number {
            // This should never happen unless checkpoint is corrupted, so we need to reset the checkpoint
            let reason = format!(
                "from_eth_block_number : {from_eth_block_number} > current_eth_block_number: {current_eth_block_number}"
            );
            self.reset_check_point(event_type, local_last_eth_block_number, &reason)
                .await?;
            return Ok(local_next_event_id);
        }
        let to_eth_block_number = current_eth_block_number
            .min(from_eth_block_number + self.config.observer_event_block_interval - 1);
        // This is asserted because we already checked that from_eth_block_number <= current_eth_block_number
        assert!(
            to_eth_block_number >= from_eth_block_number,
            "to_eth_block_number should be greater than or equal to from_eth_block_number"
        );
        generate_error_for_test()?;
        let next_event_id = match event_type {
            EventType::DepositLeafInserted => {
                self.fetch_and_write_deposit_leaf_inserted_events(
                    local_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await
            }
            EventType::Deposited => {
                self.fetch_and_write_deposited_events(
                    local_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await
            }
            EventType::BlockPosted => {
                self.fetch_and_write_block_posted_events(
                    local_next_event_id,
                    from_eth_block_number,
                    to_eth_block_number,
                )
                .await
            }
        };
        match next_event_id {
            Ok(next_event_id) => {
                if to_eth_block_number == current_eth_block_number
                    && next_event_id == local_next_event_id
                    && onchain_next_event_id > local_next_event_id
                {
                    // This means we have synced all events but the onchain event is not synced yet
                    let reason = format!(
                        "Sync all events but onchain event is not synced yet. Local next event id: {local_next_event_id}, Onchain next event id: {onchain_next_event_id}, From eth block number: {from_eth_block_number}, To eth block number: {to_eth_block_number}"
                    );
                    self.reset_check_point(event_type, local_last_eth_block_number, &reason)
                        .await?;
                    return Ok(next_event_id);
                }
                info!(
                    "Sync success. Local next event id: {}, synced next event id: {}, From eth block number: {}, To eth block number: {}",
                    local_next_event_id, next_event_id, from_eth_block_number, to_eth_block_number
                    );
                let block_hash = self
                    .get_eth_block_hash(event_type, to_eth_block_number)
                    .await?;
                self.check_point_store
                    .set_check_point(event_type, to_eth_block_number, Some(block_hash))
                    .await?;
                Ok(next_event_id)
            }
            Err(ObserverError::EventGapDetected {
                event_type: _event_type,
                expected_next_event_id,
                got_event_id,
            }) => {
                assert_eq!(event_type, _event_type, "Event type mismatch");
                if checkpoint_eth_block_number.is_none() {
                    // This never happens except for RPC issues
                    let reason = format!(
                        "Checkpoint eth block number is None But event gap detected. Expected next event id: {expected_next_event_id}, Got event id: {got_event_id}, From eth block number: {from_eth_block_number}, To eth block number: {to_eth_block_number}"
                    );
                    self.reset_check_point(event_type, local_last_eth_block_number, &reason)
                        .await?;
                    return Ok(local_next_event_id);
                }
                // If event gap detected, we need to reset the checkpoint
                let reason = format!(
                    "Event gap detected. Expected next event id: {expected_next_event_id}, Got event id: {got_event_id}, From eth block number: {from_eth_block_number}, To eth block number: {to_eth_block_number}"
                );
                self.reset_check_point(event_type, local_last_eth_block_number, &reason)
                    .await?;
                Ok(local_next_event_id)
            }
            Err(e) => {
                // Return other errors as is. Handle them in the upper function with other errors
                return Err(e);
            }
        }
    }
}

#[async_trait::async_trait(?Send)]
impl SyncEvent for RPCObserver {
    fn name(&self) -> String {
        "RPCObserver".to_string()
    }

    fn config(&self) -> ObserverConfig {
        self.config.clone()
    }

    fn rate_manager(&self) -> &RateManager {
        &self.rate_manager
    }

    #[instrument(skip(self))]
    async fn sync_events(&self, event_type: EventType) -> Result<(), ObserverError> {
        // determine whether to sync or not
        let mut local_next_event_id = self
            .observer_api
            .get_local_next_event_id(event_type)
            .await?;
        let onchain_next_event_id = self
            .observer_api
            .get_onchain_next_event_id(event_type)
            .await?;
        if local_next_event_id >= onchain_next_event_id {
            debug!(
                "No new events to sync. Local: {}, Onchain: {}",
                local_next_event_id, onchain_next_event_id
            );
            return Ok(());
        }
        info!(
            "Syncing events. Local next event id: {}, Onchain next event id: {}",
            local_next_event_id, onchain_next_event_id
        );
        // continue to sync until local_next_event_id >= onchain_next_event_id with max_query_times
        for _ in 0..self.config.observer_max_query_times {
            self.rate_manager()
                .emit_heartbeat(&sync_event_key(event_type))
                .await?;
            local_next_event_id = self
                .sync_and_save_checkpoint(event_type, onchain_next_event_id, local_next_event_id)
                .await?;
            if local_next_event_id >= onchain_next_event_id {
                break;
            }
        }
        info!(
            "Synced events. Local next event id: {}, Onchain next event id: {}",
            local_next_event_id, onchain_next_event_id
        );

        Ok(())
    }
}

// This function is used to generate an error for triggering RPC error for testing purposes.
pub fn generate_error_for_test() -> Result<(), ObserverError> {
    let error_timestamps = match std::env::var("ERROR_TIMESTAMPS") {
        Ok(val) => val,
        Err(_) => return Ok(()), // No error if env var not set
    };

    let now = chrono::Utc::now().timestamp() as u64;

    // Parse comma-separated list of timestamps
    let timestamps: Vec<u64> = error_timestamps
        .split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect();

    // Check if any timestamp is within 100 seconds of current time
    for timestamp in timestamps {
        if timestamp > now.saturating_sub(100) && timestamp < now.saturating_add(100) {
            return Err(ObserverError::EnvError(format!(
                "Error triggered by ERROR_TIMESTAMPS at time {now}"
            )));
        }
    }

    Ok(())
}
