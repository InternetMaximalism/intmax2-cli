use intmax2_client_sdk::client::{
    client::Client,
    error::ClientError,
    history::{extract_generic_transfers, GenericTransfer},
    strategy::{deposit::fetch_deposit_info, transfer::fetch_transfer_info, tx::fetch_tx_info},
};
use intmax2_interfaces::{
    api::{
        balance_prover::interface::BalanceProverClientInterface,
        block_builder::interface::BlockBuilderClientInterface,
        store_vault_server::interface::StoreVaultClientInterface,
        validity_prover::interface::ValidityProverClientInterface,
        withdrawal_server::interface::WithdrawalServerClientInterface,
    },
    data::{deposit_data::TokenType, meta_data::MetaData},
};
use intmax2_zkp::{
    common::{salt::Salt, signature::key_set::KeySet},
    ethereum_types::{address::Address, bytes32::Bytes32, u256::U256},
};
use serde::{Deserialize, Serialize};
// use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};

// type F = GoldilocksField;
// type C = PoseidonGoldilocksConfig;
// const D: usize = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositEntry {
    pub is_included: bool,
    pub is_rejected: bool,
    pub pubkey_salt_hash: Bytes32,
    pub token_type: TokenType,
    pub token_address: Address, // H160
    pub token_id: U256,
    pub token_index: Option<u32>,
    pub amount: U256,
    pub salt: Salt,
    pub block_number: u32,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiveEntry {
    pub amount: U256,
    pub token_index: u32,
    pub from: U256,
    pub is_included: bool,
    pub is_rejected: bool,
    pub meta: MetaData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendEntry {
    pub transfers: Vec<GenericTransfer>,
    pub is_included: bool,
    pub is_rejected: bool,
    pub meta: MetaData,
}

// #[derive(Clone, Debug, Serialize, Deserialize)]
// #[serde(rename_all = "camelCase")]
// pub enum HistoryEntry {
//     Deposit(DepositEntry),
//     Receive(ReceiveEntry),
//     Send(SendEntry),
// }

pub async fn fetch_deposit_history<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    key: KeySet,
    processed_deposit_uuids: Vec<String>,
) -> Result<Vec<DepositEntry>, ClientError> {
    let mut history = vec![];
    let all_deposit_info = fetch_deposit_info(
        &client.store_vault_server,
        &client.validity_prover,
        &client.liquidity_contract,
        key,
        0, // set to 0 to get all deposits
        &processed_deposit_uuids,
        client.config.deposit_timeout,
    )
    .await?;
    for (meta, settled) in all_deposit_info.settled {
        if let Some(block_number) = meta.block_number {
            history.push(DepositEntry {
                is_included: processed_deposit_uuids.contains(&meta.uuid),
                is_rejected: false,
                pubkey_salt_hash: settled.pubkey_salt_hash,
                token_type: settled.token_type,
                token_address: settled.token_address,
                token_id: settled.token_id,
                token_index: settled.token_index,
                amount: settled.amount,
                salt: settled.deposit_salt,
                block_number,
                timestamp: meta.timestamp,
            });
        }
    }
    for (meta, pending) in all_deposit_info.pending {
        if let Some(block_number) = meta.block_number {
            history.push(DepositEntry {
                is_included: false,
                is_rejected: false,
                pubkey_salt_hash: pending.pubkey_salt_hash,
                token_type: pending.token_type,
                token_address: pending.token_address,
                token_id: pending.token_id,
                token_index: pending.token_index,
                amount: pending.amount,
                salt: pending.deposit_salt,
                block_number,
                timestamp: meta.timestamp,
            });
        }
    }
    for (meta, timeout) in all_deposit_info.timeout {
        if let Some(block_number) = meta.block_number {
            history.push(DepositEntry {
                is_included: false,
                is_rejected: true,
                pubkey_salt_hash: timeout.pubkey_salt_hash,
                token_type: timeout.token_type,
                token_address: timeout.token_address,
                token_id: timeout.token_id,
                token_index: timeout.token_index,
                amount: timeout.amount,
                salt: timeout.deposit_salt,
                block_number,
                timestamp: meta.timestamp,
            });
        }
    }

    Ok(history)
}

pub async fn fetch_transfer_history<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    key: KeySet,
    processed_transfer_uuids: Vec<String>,
) -> Result<Vec<ReceiveEntry>, ClientError> {
    let mut history = vec![];
    let all_transfers_info = fetch_transfer_info(
        &client.store_vault_server,
        &client.validity_prover,
        key,
        0, // set to 0 to get all transfers
        &processed_transfer_uuids,
        client.config.tx_timeout,
    )
    .await?;
    for (meta, settled) in all_transfers_info.settled {
        let transfer = settled.transfer;
        history.push(ReceiveEntry {
            amount: transfer.amount,
            token_index: transfer.token_index,
            from: transfer.recipient.data,
            is_included: processed_transfer_uuids.contains(&meta.uuid),
            is_rejected: false,
            meta: meta.clone(),
        });
    }
    for (meta, pending) in all_transfers_info.pending {
        let transfer = pending.transfer;
        history.push(ReceiveEntry {
            amount: transfer.amount,
            token_index: transfer.token_index,
            from: transfer.recipient.data,
            is_included: false,
            is_rejected: false,
            meta: meta.clone(),
        });
    }
    for (meta, timeout) in all_transfers_info.timeout {
        let transfer = timeout.transfer;
        history.push(ReceiveEntry {
            amount: transfer.amount,
            token_index: transfer.token_index,
            from: transfer.recipient.data,
            is_included: false,
            is_rejected: true,
            meta: meta.clone(),
        });
    }

    Ok(history)
}

pub async fn fetch_tx_history<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    key: KeySet,
    processed_tx_uuids: Vec<String>,
) -> Result<Vec<SendEntry>, ClientError> {
    let mut history = vec![];
    let all_tx_info = fetch_tx_info(
        &client.store_vault_server,
        &client.validity_prover,
        key,
        0, // set to 0 to get all txs
        &processed_tx_uuids,
        client.config.tx_timeout,
    )
    .await?;
    for (meta, settled) in all_tx_info.settled {
        history.push(SendEntry {
            transfers: extract_generic_transfers(settled),
            is_included: processed_tx_uuids.contains(&meta.uuid),
            is_rejected: false,
            meta,
        });
    }
    for (meta, pending) in all_tx_info.pending {
        history.push(SendEntry {
            transfers: extract_generic_transfers(pending),
            is_included: false,
            is_rejected: false,
            meta,
        });
    }
    for (meta, timeout) in all_tx_info.timeout {
        history.push(SendEntry {
            transfers: extract_generic_transfers(timeout),
            is_included: false,
            is_rejected: true,
            meta,
        });
    }

    Ok(history)
}

// pub async fn fetch_history<
//     BB: BlockBuilderClientInterface,
//     S: StoreVaultClientInterface,
//     V: ValidityProverClientInterface,
//     B: BalanceProverClientInterface,
//     W: WithdrawalServerClientInterface,
// >(
//     client: &Client<BB, S, V, B, W>,
//     key: KeySet,
// ) -> Result<Vec<HistoryEntry>, ClientError> {
//     let mut history = Vec::new();

//     let user_data = client.get_user_data(key).await?;

//     fetch_deposit_history(&mut history, client, key, user_data.processed_deposit_uuids).await?;

//     fetch_transfer_history(
//         &mut history,
//         client,
//         key,
//         user_data.processed_transfer_uuids,
//     )
//     .await?;

//     fetch_tx_history(&mut history, client, key, user_data.processed_tx_uuids).await?;

//     // sort history
//     history.sort_by_key(|entry| match entry {
//         HistoryEntry::Deposit(DepositEntry { meta, .. }) => meta.timestamp,
//         HistoryEntry::Receive(ReceiveEntry { meta, .. }) => meta.timestamp,
//         HistoryEntry::Send(SendEntry { meta, .. }) => meta.timestamp,
//     });

//     Ok(history)
// }
