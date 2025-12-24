use crate::external_api::contract::{
    convert::{convert_address_to_intmax, convert_bytes32_to_tx_hash, convert_tx_hash_to_bytes32},
    data_decoder::decode_post_block_calldata,
    utils::{get_batch_transaction, get_batch_transaction_receipt},
};
use alloy::{
    consensus::Transaction,
    network::TransactionBuilder,
    primitives::{Address, Bytes, B256, U256},
    rpc::types::TransactionReceipt,
    sol,
};
use intmax2_zkp::{
    common::{
        block::Block,
        signature_content::{
            block_sign_payload::BlockSignPayload,
            flatten::{FlatG1, FlatG2},
            utils::get_pubkey_hash,
            SignatureContent,
        },
        witness::full_block::FullBlock,
    },
    constants::NUM_SENDERS_IN_BLOCK,
    ethereum_types::{
        account_id::AccountIdPacked, address::Address as ZkpAddress, bytes16::Bytes16,
        bytes32::Bytes32, u256::U256 as ZkpU256, u32limb_trait::U32LimbTrait as _,
    },
};
use std::time::Instant;

use super::{
    convert::{
        convert_b128_to_byte16, convert_b256_to_bytes32, convert_bytes16_to_b128,
        convert_bytes32_to_b256, convert_u256_to_alloy, convert_u256_to_intmax,
    },
    error::BlockchainError,
    handlers::send_transaction_with_gas_bump,
    proxy_contract::ProxyContract,
    utils::{get_provider_with_signer, NormalProvider},
};

sol!(
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    Rollup,
    "abi/Rollup.json",
);

#[derive(Clone, Debug)]
pub struct DepositLeafInserted {
    pub deposit_index: u32,
    pub deposit_hash: Bytes32,

    // meta data
    pub eth_block_number: u64,
    pub eth_tx_index: u64,
}

#[derive(Clone, Debug)]
pub struct DepositLeafInsertedWithBlockNumber {
    pub deposit_index: u32,
    pub deposit_hash: Bytes32,
    pub next_block_number: u32,
    pub eth_block_number: u64,
    pub eth_tx_index: u64,
}

#[derive(Clone, Debug)]
pub struct BlockPosted {
    pub prev_block_hash: Bytes32,
    pub block_builder: ZkpAddress,
    pub timestamp: u64,
    pub block_number: u32,
    pub deposit_tree_root: Bytes32,
    pub signature_hash: Bytes32,

    // meta data
    pub tx_hash: Bytes32,
    pub eth_block_number: u64,
    pub eth_tx_index: u64,
}

#[derive(Clone, Debug)]
pub struct FullBlockWithMeta {
    pub full_block: FullBlock,
    pub eth_block_number: u64,
    pub eth_tx_index: u64,
}

#[derive(Clone, Debug)]
pub struct BlockPostDataEvent {
    pub is_registration_block: bool,
    pub tx_tree_root: Bytes32,
    pub expiry: u64,
    pub builder_address: ZkpAddress,
    pub builder_nonce: u32,
    pub sender_flags: Bytes16,
}

#[derive(Clone, Debug)]
pub struct FullBlockPostedEvent {
    pub block_number: u32,
    pub prev_block_hash: Bytes32,
    pub timestamp: u64,
    pub deposit_tree_root: Bytes32,
    pub block_data: BlockPostDataEvent,
    pub aggregated_public_key: FlatG1,
    pub aggregated_signature: FlatG2,
    pub message_point: FlatG2,
    pub sender_public_keys: Vec<ZkpU256>,
    pub public_keys_hash: Bytes32,
    pub sender_account_ids: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RollupContract {
    pub provider: NormalProvider,
    pub address: Address,
}

impl RollupContract {
    pub fn new(provider: NormalProvider, address: Address) -> Self {
        Self { provider, address }
    }

    pub async fn deploy(provider: NormalProvider, private_key: B256) -> anyhow::Result<Self> {
        let signer = get_provider_with_signer(&provider, private_key);
        let impl_contract = Rollup::deploy(signer).await?;
        let impl_address = *impl_contract.address();
        let proxy = ProxyContract::deploy(provider.clone(), private_key, impl_address, &[]).await?;
        Ok(Self {
            provider,
            address: proxy.address,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn initialize(
        &self,
        signer_private_key: B256,
        admin: Address,
        scroll_messenger_address: Address,
        liquidity_address: Address,
        contribution_address: Address,
        rate_limit_threshold_interval: U256,
        rate_limit_alpha: U256,
        rate_limit_k: U256,
    ) -> Result<B256, BlockchainError> {
        let signer = get_provider_with_signer(&self.provider, signer_private_key);
        let contract = Rollup::new(self.address, signer.clone());
        let tx_request = contract
            .initialize(
                admin,
                scroll_messenger_address,
                liquidity_address,
                contribution_address,
                rate_limit_threshold_interval,
                rate_limit_alpha,
                rate_limit_k,
            )
            .into_transaction_request();
        send_transaction_with_gas_bump(signer, tx_request, "initialize").await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn post_registration_block(
        &self,
        signer_private_key: B256,
        gas_limit: Option<u64>,
        msg_value: ZkpU256,
        tx_tree_root: Bytes32,
        expiry: u64,
        block_builder_nonce: u32,
        sender_flag: Bytes16,
        agg_pubkey: FlatG1,
        agg_signature: FlatG2,
        message_point: FlatG2,
        sender_public_keys: Vec<ZkpU256>,
    ) -> Result<B256, BlockchainError> {
        let signer = get_provider_with_signer(&self.provider, signer_private_key);
        let contract = Rollup::new(self.address, signer.clone());

        // Convert types to alloy types
        let tx_tree_root_bytes = convert_bytes32_to_b256(tx_tree_root);
        let sender_flag_bytes = convert_bytes16_to_b128(sender_flag);
        let agg_pubkey_bytes: [B256; 2] = [
            convert_u256_to_alloy(agg_pubkey.0[0]).into(),
            convert_u256_to_alloy(agg_pubkey.0[1]).into(),
        ];
        let agg_signature_bytes: [B256; 4] = [
            convert_u256_to_alloy(agg_signature.0[0]).into(),
            convert_u256_to_alloy(agg_signature.0[1]).into(),
            convert_u256_to_alloy(agg_signature.0[2]).into(),
            convert_u256_to_alloy(agg_signature.0[3]).into(),
        ];
        let message_point_bytes: [B256; 4] = [
            convert_u256_to_alloy(message_point.0[0]).into(),
            convert_u256_to_alloy(message_point.0[1]).into(),
            convert_u256_to_alloy(message_point.0[2]).into(),
            convert_u256_to_alloy(message_point.0[3]).into(),
        ];
        let sender_pubkeys: Vec<U256> = sender_public_keys
            .iter()
            .map(|pubkey| convert_u256_to_alloy(*pubkey))
            .collect();
        let msg_value = convert_u256_to_alloy(msg_value);
        let mut tx_request = contract
            .postRegistrationBlock(
                tx_tree_root_bytes,
                expiry,
                block_builder_nonce,
                sender_flag_bytes,
                agg_pubkey_bytes,
                agg_signature_bytes,
                message_point_bytes,
                sender_pubkeys,
            )
            .into_transaction_request();
        tx_request.set_value(msg_value);
        if let Some(gas_limit) = gas_limit {
            tx_request.set_gas_limit(gas_limit);
        }
        send_transaction_with_gas_bump(signer, tx_request, "post_registration_block").await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn post_non_registration_block(
        &self,
        signer_private_key: B256,
        gas_limit: Option<u64>,
        msg_value: ZkpU256,
        tx_tree_root: Bytes32,
        expiry: u64,
        block_builder_nonce: u32,
        sender_flag: Bytes16,
        agg_pubkey: FlatG1,
        agg_signature: FlatG2,
        message_point: FlatG2,
        public_keys_hash: Bytes32,
        account_ids: Vec<u8>,
    ) -> Result<B256, BlockchainError> {
        let signer = get_provider_with_signer(&self.provider, signer_private_key);
        let contract = Rollup::new(self.address, signer.clone());

        // Convert types to alloy types
        let tx_tree_root_bytes = convert_bytes32_to_b256(tx_tree_root);
        let sender_flag_bytes = convert_bytes16_to_b128(sender_flag);
        let agg_pubkey_bytes: [B256; 2] = [
            convert_u256_to_alloy(agg_pubkey.0[0]).into(),
            convert_u256_to_alloy(agg_pubkey.0[1]).into(),
        ];
        let agg_signature_bytes: [B256; 4] = [
            convert_u256_to_alloy(agg_signature.0[0]).into(),
            convert_u256_to_alloy(agg_signature.0[1]).into(),
            convert_u256_to_alloy(agg_signature.0[2]).into(),
            convert_u256_to_alloy(agg_signature.0[3]).into(),
        ];
        let message_point_bytes: [B256; 4] = [
            convert_u256_to_alloy(message_point.0[0]).into(),
            convert_u256_to_alloy(message_point.0[1]).into(),
            convert_u256_to_alloy(message_point.0[2]).into(),
            convert_u256_to_alloy(message_point.0[3]).into(),
        ];
        let public_keys_hash_bytes = convert_bytes32_to_b256(public_keys_hash);
        let account_ids_bytes = Bytes::from(account_ids);
        let msg_value = convert_u256_to_alloy(msg_value);

        let mut tx_request = contract
            .postNonRegistrationBlock(
                tx_tree_root_bytes,
                expiry,
                block_builder_nonce,
                sender_flag_bytes,
                agg_pubkey_bytes,
                agg_signature_bytes,
                message_point_bytes,
                public_keys_hash_bytes,
                account_ids_bytes,
            )
            .into_transaction_request();
        tx_request.set_value(msg_value);
        if let Some(gas_limit) = gas_limit {
            tx_request.set_gas_limit(gas_limit);
        }
        send_transaction_with_gas_bump(signer, tx_request, "post_non_registration_block").await
    }

    /// This is a backdoor method to simplify relaying deposits for testing purposes.
    /// It will be reverted in other environments.
    pub async fn process_deposits(
        &self,
        signer_private_key: B256,
        gas_limit: Option<u64>,
        last_processed_deposit_id: u32,
        deposit_hashes: &[Bytes32],
    ) -> Result<B256, BlockchainError> {
        let signer = get_provider_with_signer(&self.provider, signer_private_key);
        let contract = Rollup::new(self.address, signer.clone());
        let deposit_hashes_bytes: Vec<B256> = deposit_hashes
            .iter()
            .map(|e| convert_bytes32_to_b256(*e))
            .collect();
        let mut tx_request = contract
            .processDeposits(U256::from(last_processed_deposit_id), deposit_hashes_bytes)
            .into_transaction_request();
        if let Some(gas_limit) = gas_limit {
            tx_request.set_gas_limit(gas_limit);
        }
        send_transaction_with_gas_bump(signer, tx_request, "process_deposits").await
    }

    pub async fn get_latest_block_number(&self) -> Result<u32, BlockchainError> {
        let contract = Rollup::new(self.address, self.provider.clone());
        let latest_block_number = contract.getLatestBlockNumber().call().await?;
        Ok(latest_block_number)
    }

    pub async fn get_next_deposit_index(&self) -> Result<u32, BlockchainError> {
        let contract = Rollup::new(self.address, self.provider.clone());
        let next_deposit_index = contract.depositIndex().call().await?;
        Ok(next_deposit_index)
    }

    pub async fn get_l2_scroll_messenger(&self) -> Result<Address, BlockchainError> {
        let contract = Rollup::new(self.address, self.provider.clone());
        let l2_scroll_messenger = contract.l2ScrollMessenger().call().await?;
        Ok(l2_scroll_messenger)
    }

    pub async fn get_block_hash(&self, block_number: u32) -> Result<Bytes32, BlockchainError> {
        let contract = Rollup::new(self.address, self.provider.clone());
        let block_hash = contract.getBlockHash(block_number).call().await?;
        Ok(convert_b256_to_bytes32(block_hash))
    }

    pub async fn get_penalty(&self) -> Result<ZkpU256, BlockchainError> {
        let contract = Rollup::new(self.address, self.provider.clone());
        let penalty = contract.getPenalty().call().await?;
        Ok(convert_u256_to_intmax(penalty))
    }

    /// Returns the nonce for the block builder address
    pub async fn get_block_builder_nonce(
        &self,
        is_registration: bool,
        block_builder_address: Address,
    ) -> Result<u32, BlockchainError> {
        let contract = Rollup::new(self.address, self.provider.clone());
        let nonce = if is_registration {
            contract
                .builderRegistrationNonce(block_builder_address)
                .call()
                .await?
        } else {
            contract
                .builderNonRegistrationNonce(block_builder_address)
                .call()
                .await?
        };
        Ok(nonce)
    }
}

// Event related methods
impl RollupContract {
    pub async fn get_blocks_posted_event(
        &self,
        from_eth_block: u64,
        to_eth_block: u64,
    ) -> Result<Vec<BlockPosted>, BlockchainError> {
        log::info!("get_blocks_posted_event: from_block={from_eth_block}, to_block={to_eth_block}");
        let contract = Rollup::new(self.address, self.provider.clone());
        let events = contract
            .event_filter::<Rollup::BlockPosted>()
            .address(self.address)
            .from_block(from_eth_block)
            .to_block(to_eth_block)
            .query()
            .await?;
        let mut block_posited_events = Vec::new();
        for (event, meta) in events {
            block_posited_events.push(BlockPosted {
                prev_block_hash: convert_b256_to_bytes32(event.prevBlockHash),
                block_builder: convert_address_to_intmax(event.blockBuilder),
                timestamp: event.timestamp,
                block_number: event.blockNumber.to(),
                deposit_tree_root: convert_b256_to_bytes32(event.depositTreeRoot),
                signature_hash: convert_b256_to_bytes32(event.signatureHash),
                tx_hash: convert_tx_hash_to_bytes32(meta.transaction_hash.unwrap()),
                eth_block_number: meta.block_number.unwrap(),
                eth_tx_index: meta.transaction_index.unwrap(),
            });
        }
        block_posited_events.sort_by_key(|event| event.block_number);
        Ok(block_posited_events)
    }

    pub async fn get_full_block_with_meta(
        &self,
        block_posted_events: &[BlockPosted],
    ) -> Result<Vec<FullBlockWithMeta>, BlockchainError> {
        let tx_hashes = block_posted_events
            .iter()
            .map(|e| convert_bytes32_to_tx_hash(e.tx_hash))
            .collect::<Vec<_>>();
        let instant = Instant::now();
        let txs = get_batch_transaction(&self.provider, &tx_hashes).await?;
        log::info!(
            "get_batch_transaction: {:?} for {} txs",
            instant.elapsed(),
            tx_hashes.len()
        );
        let receipts = get_batch_transaction_receipt(&self.provider, &tx_hashes).await?;
        let mut full_blocks = Vec::new();
        for ((tx, receipt), event) in txs.iter().zip(receipts.iter()).zip(block_posted_events) {
            let full_block_event = Self::parse_full_block_posted(receipt)?
                .into_iter()
                .find(|e| e.block_number == event.block_number);
            let full_block = if let Some(full_block_event) = full_block_event {
                full_block_from_posted_event(&full_block_event)?
            } else {
                let input = tx.input();
                decode_post_block_calldata(
                    event.prev_block_hash,
                    event.deposit_tree_root,
                    event.timestamp,
                    event.block_number,
                    event.block_builder,
                    input,
                )
                .map_err(|e| {
                    BlockchainError::DecodeCallDataError(format!(
                        "failed to decode post block calldata: {e}"
                    ))
                })?
            };
            full_blocks.push(FullBlockWithMeta {
                full_block,
                eth_block_number: event.eth_block_number,
                eth_tx_index: event.eth_tx_index,
            });
        }

        // Sort by block number
        full_blocks.sort_by_key(|block| block.full_block.block.block_number);

        Ok(full_blocks)
    }

    pub async fn get_deposit_leaf_inserted_events(
        &self,
        from_eth_block: u64,
        to_eth_block_number: u64,
    ) -> Result<Vec<DepositLeafInserted>, BlockchainError> {
        log::info!(
            "get_deposit_leaf_inserted_event: from_eth_block={from_eth_block}, to_eth_block_number={to_eth_block_number}"
        );
        let contract = Rollup::new(self.address, self.provider.clone());
        let events = contract
            .event_filter::<Rollup::DepositLeafInserted>()
            .address(self.address)
            .from_block(from_eth_block)
            .to_block(to_eth_block_number)
            .query()
            .await?;
        let mut deposit_leaf_inserted_events = Vec::new();
        for (event, meta) in events {
            deposit_leaf_inserted_events.push(DepositLeafInserted {
                deposit_index: event.depositIndex,
                deposit_hash: convert_b256_to_bytes32(event.depositHash),
                eth_block_number: meta.block_number.unwrap(),
                eth_tx_index: meta.transaction_index.unwrap(),
            });
        }
        deposit_leaf_inserted_events.sort_by_key(|event| event.deposit_index);
        Ok(deposit_leaf_inserted_events)
    }

    pub async fn get_deposit_leaf_inserted_with_block_number_events(
        &self,
        from_eth_block: u64,
        to_eth_block_number: u64,
    ) -> Result<Vec<DepositLeafInsertedWithBlockNumber>, BlockchainError> {
        log::info!(
            "get_deposit_leaf_inserted_with_block_number_events: from_eth_block={from_eth_block}, to_eth_block_number={to_eth_block_number}"
        );
        let contract = Rollup::new(self.address, self.provider.clone());
        let events = contract
            .event_filter::<Rollup::DepositLeafInsertedWithBlockNumber>()
            .address(self.address)
            .from_block(from_eth_block)
            .to_block(to_eth_block_number)
            .query()
            .await?;
        let mut deposit_leaf_inserted_events = Vec::new();
        for (event, meta) in events {
            deposit_leaf_inserted_events.push(DepositLeafInsertedWithBlockNumber {
                deposit_index: event.depositIndex,
                deposit_hash: convert_b256_to_bytes32(event.depositHash),
                next_block_number: event.nextBlockNumber,
                eth_block_number: meta.block_number.unwrap(),
                eth_tx_index: meta.transaction_index.unwrap(),
            });
        }
        deposit_leaf_inserted_events.sort_by_key(|event| event.deposit_index);
        Ok(deposit_leaf_inserted_events)
    }

    fn full_block_posted_event_from_sol(
        event: &Rollup::FullBlockPosted,
    ) -> Result<FullBlockPostedEvent, BlockchainError> {
        let block_data = &event.blockData;
        let sender_public_keys = event
            .senderPublicKeys
            .iter()
            .cloned()
            .map(convert_u256_to_intmax)
            .collect();
        Ok(FullBlockPostedEvent {
            block_number: event.blockNumber,
            prev_block_hash: convert_b256_to_bytes32(event.prevBlockHash),
            timestamp: event.timestamp,
            deposit_tree_root: convert_b256_to_bytes32(event.depositTreeRoot),
            block_data: BlockPostDataEvent {
                is_registration_block: block_data.isRegistrationBlock,
                tx_tree_root: convert_b256_to_bytes32(block_data.txTreeRoot),
                expiry: block_data.expiry,
                builder_address: convert_address_to_intmax(block_data.builderAddress),
                builder_nonce: block_data.builderNonce,
                sender_flags: convert_b128_to_byte16(block_data.senderFlags),
            },
            aggregated_public_key: convert_to_flat_g1(event.aggregatedPublicKey)?,
            aggregated_signature: convert_to_flat_g2(event.aggregatedSignature)?,
            message_point: convert_to_flat_g2(event.messagePoint)?,
            sender_public_keys,
            public_keys_hash: convert_b256_to_bytes32(event.publicKeysHash),
            sender_account_ids: event.senderAccountIds.to_vec(),
        })
    }

    pub async fn get_full_block_posted_events(
        &self,
        from_eth_block: u64,
        to_eth_block_number: u64,
    ) -> Result<Vec<FullBlockWithMeta>, BlockchainError> {
        log::info!(
            "get_full_block_posted_events: from_eth_block={from_eth_block}, to_eth_block_number={to_eth_block_number}"
        );
        let contract = Rollup::new(self.address, self.provider.clone());
        let events = contract
            .event_filter::<Rollup::FullBlockPosted>()
            .address(self.address)
            .from_block(from_eth_block)
            .to_block(to_eth_block_number)
            .query()
            .await?;
        let mut full_blocks = Vec::new();
        for (event, meta) in events {
            let full_block_event = Self::full_block_posted_event_from_sol(&event)?;
            let full_block = full_block_from_posted_event(&full_block_event)?;
            full_blocks.push(FullBlockWithMeta {
                full_block,
                eth_block_number: meta.block_number.unwrap(),
                eth_tx_index: meta.transaction_index.unwrap(),
            });
        }
        full_blocks.sort_by_key(|block| block.full_block.block.block_number);
        Ok(full_blocks)
    }

    pub fn parse_full_block_posted(
        receipt: &TransactionReceipt,
    ) -> Result<Vec<FullBlockPostedEvent>, BlockchainError> {
        let mut events = Vec::new();
        for log in receipt.logs() {
            match log.log_decode::<Rollup::FullBlockPosted>() {
                Ok(event) => {
                    let full_block_event = Self::full_block_posted_event_from_sol(&event.inner)?;
                    events.push(full_block_event);
                }
                Err(_) => continue,
            }
        }
        Ok(events)
    }

    pub fn parse_deposit_leaf_inserted_with_block_number(
        &self,
        receipt: &TransactionReceipt,
    ) -> Result<Vec<DepositLeafInsertedWithBlockNumber>, BlockchainError> {
        let mut events = Vec::new();
        let eth_block_number = receipt.block_number.ok_or_else(|| {
            BlockchainError::ParseError("missing receipt block_number".to_string())
        })?;
        let eth_tx_index = receipt.transaction_index.ok_or_else(|| {
            BlockchainError::ParseError("missing receipt transaction_index".to_string())
        })?;
        for log in receipt.logs() {
            match log.log_decode::<Rollup::DepositLeafInsertedWithBlockNumber>() {
                Ok(event) => {
                    let inner = event.inner;
                    events.push(DepositLeafInsertedWithBlockNumber {
                        deposit_index: inner.depositIndex,
                        deposit_hash: convert_b256_to_bytes32(inner.depositHash),
                        next_block_number: inner.nextBlockNumber,
                        eth_block_number,
                        eth_tx_index,
                    });
                }
                Err(_) => continue,
            }
        }
        Ok(events)
    }
}

pub fn full_block_from_posted_event(
    event: &FullBlockPostedEvent,
) -> Result<FullBlock, BlockchainError> {
    let block_sign_payload = BlockSignPayload {
        is_registration_block: event.block_data.is_registration_block,
        tx_tree_root: event.block_data.tx_tree_root,
        expiry: event.block_data.expiry.into(),
        block_builder_address: event.block_data.builder_address,
        block_builder_nonce: event.block_data.builder_nonce,
    };
    let (signature, pubkeys, account_ids) = if event.block_data.is_registration_block {
        let pubkeys = event.sender_public_keys.clone();
        let signature = SignatureContent {
            block_sign_payload,
            sender_flag: event.block_data.sender_flags,
            agg_pubkey: event.aggregated_public_key.clone(),
            agg_signature: event.aggregated_signature.clone(),
            message_point: event.message_point.clone(),
            pubkey_hash: pad_pubkey_and_hash(&pubkeys),
            account_id_hash: Bytes32::default(),
        };
        (signature, Some(pubkeys), None)
    } else {
        let account_id_packed = AccountIdPacked::from_trimmed_bytes(&event.sender_account_ids)
            .map_err(|e| BlockchainError::ParseError(format!("invalid account ids: {e}")))?;
        let signature = SignatureContent {
            block_sign_payload,
            sender_flag: event.block_data.sender_flags,
            agg_pubkey: event.aggregated_public_key.clone(),
            agg_signature: event.aggregated_signature.clone(),
            message_point: event.message_point.clone(),
            pubkey_hash: event.public_keys_hash,
            account_id_hash: account_id_packed.hash(),
        };
        (signature, None, Some(event.sender_account_ids.clone()))
    };
    let block = Block {
        prev_block_hash: event.prev_block_hash,
        deposit_tree_root: event.deposit_tree_root,
        signature_hash: signature.hash(),
        timestamp: event.timestamp,
        block_number: event.block_number,
    };
    Ok(FullBlock {
        block,
        signature,
        pubkeys,
        account_ids,
    })
}

fn convert_to_flat_g1(data: [B256; 2]) -> Result<FlatG1, BlockchainError> {
    let flat_g1 = FlatG1([
        ZkpU256::from_bytes_be(&data[0].0)
            .map_err(|e| BlockchainError::ParseError(format!("invalid FlatG1[0]: {e}")))?,
        ZkpU256::from_bytes_be(&data[1].0)
            .map_err(|e| BlockchainError::ParseError(format!("invalid FlatG1[1]: {e}")))?,
    ]);
    Ok(flat_g1)
}

fn convert_to_flat_g2(data: [B256; 4]) -> Result<FlatG2, BlockchainError> {
    let flat_g2 = FlatG2([
        ZkpU256::from_bytes_be(&data[0].0)
            .map_err(|e| BlockchainError::ParseError(format!("invalid FlatG2[0]: {e}")))?,
        ZkpU256::from_bytes_be(&data[1].0)
            .map_err(|e| BlockchainError::ParseError(format!("invalid FlatG2[1]: {e}")))?,
        ZkpU256::from_bytes_be(&data[2].0)
            .map_err(|e| BlockchainError::ParseError(format!("invalid FlatG2[2]: {e}")))?,
        ZkpU256::from_bytes_be(&data[3].0)
            .map_err(|e| BlockchainError::ParseError(format!("invalid FlatG2[3]: {e}")))?,
    ]);
    Ok(flat_g2)
}

fn pad_pubkey_and_hash(pubkeys: &[ZkpU256]) -> Bytes32 {
    let mut pubkeys = pubkeys.to_vec();
    pubkeys.resize(NUM_SENDERS_IN_BLOCK, ZkpU256::dummy_pubkey());
    get_pubkey_hash(&pubkeys)
}
