use std::sync::Arc;

use ethers::{
    contract::abigen,
    core::k256::ecdsa::SigningKey,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::Wallet,
    types::H256,
};
use intmax2_zkp::{
    common::witness::full_block::FullBlock,
    ethereum_types::{address::Address, bytes32::Bytes32, u32limb_trait::U32LimbTrait as _},
};

use crate::external_api::{contract::utils::get_latest_block_number, utils::retry::with_retry};

use super::{
    data_decoder::decode_post_block_calldata,
    interface::BlockchainError,
    utils::{get_client, get_client_with_signer, get_transaction},
};

const EVENT_BLOCK_RANGE: u64 = 10000;

abigen!(Rollup, "abi/Rollup.json",);

#[derive(Clone, Debug)]
pub struct DepositLeafInserted {
    pub deposit_index: u32,
    pub deposit_hash: Bytes32,
    pub block_number: u64,
}

#[derive(Clone, Debug)]
pub struct BlockPosted {
    pub prev_block_hash: Bytes32,
    pub block_builder: Address,
    pub block_number: u32,
    pub deposit_tree_root: Bytes32,
    pub signature_hash: Bytes32,
    pub tx_hash: H256,
}

#[derive(Debug, Clone)]
pub struct RollupContract {
    pub rpc_url: String,
    pub chain_id: u64,
    pub contract_address: ethers::types::Address,
    pub deployed_block_number: u64,
}

impl RollupContract {
    pub fn new(
        rpc_url: String,
        chain_id: u64,
        contract_address: &str,
        deployed_block_number: u64,
    ) -> Self {
        Self {
            rpc_url,
            chain_id,
            contract_address: contract_address.parse().unwrap(),
            deployed_block_number,
        }
    }

    pub async fn get_contract(&self) -> Result<rollup::Rollup<Provider<Http>>, BlockchainError> {
        let client = get_client(&self.rpc_url).await?;
        let contract = Rollup::new(self.contract_address, client);
        Ok(contract)
    }

    pub async fn get_contract_with_signer(
        &self,
        private_key: H256,
    ) -> Result<rollup::Rollup<SignerMiddleware<Provider<Http>, Wallet<SigningKey>>>, BlockchainError>
    {
        let client = get_client_with_signer(&self.rpc_url, self.chain_id, private_key).await?;
        let contract = Rollup::new(self.contract_address, Arc::new(client));
        Ok(contract)
    }

    pub async fn get_deposit_leaf_inserted_event(
        &self,
        from_block: Option<u64>,
    ) -> Result<Vec<DepositLeafInserted>, BlockchainError> {
        log::info!("get_deposit_leaf_inserted_event");
        let mut events = Vec::new();
        let mut from_block = from_block.unwrap_or(self.deployed_block_number);
        loop {
            log::info!("get_deposit_leaf_inserted_event: from_block={}", from_block);
            let contract = self.get_contract().await?;
            let new_events = with_retry(|| async {
                contract
                    .deposit_leaf_inserted_filter()
                    .address(self.contract_address.into())
                    .from_block(from_block)
                    .to_block(from_block + EVENT_BLOCK_RANGE - 1)
                    .query_with_meta()
                    .await
            })
            .await
            .map_err(|_| {
                BlockchainError::NetworkError(
                    "failed to get deposit leaf inserted event".to_string(),
                )
            })?;
            events.extend(new_events);
            let latest_block_number = get_latest_block_number(&self.rpc_url).await?;
            from_block += EVENT_BLOCK_RANGE;
            if from_block > latest_block_number {
                break;
            }
        }
        let mut deposit_leaf_inserted_events = Vec::new();
        for (event, meta) in events {
            deposit_leaf_inserted_events.push(DepositLeafInserted {
                deposit_index: event.deposit_index,
                deposit_hash: Bytes32::from_bytes_be(&event.deposit_hash),
                block_number: meta.block_number.as_u64(),
            });
        }
        deposit_leaf_inserted_events.sort_by_key(|event| event.deposit_index);
        Ok(deposit_leaf_inserted_events)
    }

    async fn get_blocks_posted_event(
        &self,
        from_block: Option<u64>,
    ) -> Result<Vec<BlockPosted>, BlockchainError> {
        log::info!("get_blocks_posted_event");
        let mut events = Vec::new();
        let mut from_block = from_block.unwrap_or(self.deployed_block_number);
        loop {
            log::info!("get_blocks_posted_event: from_block={}", from_block);
            let contract = self.get_contract().await?;
            let new_events = with_retry(|| async {
                contract
                    .block_posted_filter()
                    .address(self.contract_address.into())
                    .from_block(from_block)
                    .to_block(from_block + EVENT_BLOCK_RANGE - 1)
                    .query_with_meta()
                    .await
            })
            .await
            .map_err(|_| {
                BlockchainError::NetworkError("failed to get blocks posted event".to_string())
            })?;
            events.extend(new_events);
            let latest_block_number = get_latest_block_number(&self.rpc_url).await?;
            from_block += EVENT_BLOCK_RANGE;
            if from_block > latest_block_number {
                break;
            }
        }
        let mut blocks_posted_events = Vec::new();
        for (event, meta) in events {
            blocks_posted_events.push(BlockPosted {
                prev_block_hash: Bytes32::from_bytes_be(&event.prev_block_hash),
                block_builder: Address::from_bytes_be(&event.block_builder.as_bytes()),
                block_number: event.block_number.as_u32(),
                deposit_tree_root: Bytes32::from_bytes_be(&event.deposit_tree_root),
                signature_hash: Bytes32::from_bytes_be(&event.signature_hash),
                tx_hash: meta.transaction_hash,
            });
        }
        blocks_posted_events.sort_by_key(|event| event.block_number);
        Ok(blocks_posted_events)
    }

    pub async fn get_full_blocks(
        &self,
        from_block: Option<u64>,
    ) -> Result<Vec<FullBlock>, BlockchainError> {
        let blocks_posted_events = self.get_blocks_posted_event(from_block).await?;
        let mut full_blocks = Vec::new();
        for event in blocks_posted_events {
            let tx = get_transaction(&self.rpc_url, event.tx_hash).await?.ok_or(
                BlockchainError::InternalError("failed to get transaction".to_string()),
            )?;
            let contract = self.get_contract().await?;
            let functions = contract.abi().functions();
            let full_block = decode_post_block_calldata(
                functions,
                event.prev_block_hash,
                event.deposit_tree_root,
                event.block_number,
                &tx.input.to_vec(),
            )
            .map_err(|e| {
                BlockchainError::DecodeCallDataError(format!(
                    "failed to decode post block calldata: {}",
                    e
                ))
            })?;
            full_blocks.push(full_block);
        }
        Ok(full_blocks)
    }
}
