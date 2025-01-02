use intmax2_interfaces::{
    api::{
        balance_prover::interface::BalanceProverClientInterface,
        block_builder::interface::BlockBuilderClientInterface,
        store_vault_server::interface::StoreVaultClientInterface,
        validity_prover::interface::ValidityProverClientInterface,
        withdrawal_server::interface::WithdrawalServerClientInterface,
    },
    data::{deposit_data::TokenType, meta_data::MetaData, tx_data::TxData},
};
use intmax2_zkp::{
    common::signature::key_set::KeySet,
    ethereum_types::{address::Address, bytes32::Bytes32, u256::U256, u32limb_trait::U32LimbTrait},
};
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use serde::{Deserialize, Serialize};

use super::{
    client::Client,
    error::ClientError,
    strategy::{deposit::fetch_deposit_info, transfer::fetch_transfer_info, tx::fetch_tx_info},
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryEntry {
    Deposit {
        token_type: TokenType,
        token_address: Address,
        token_id: U256,
        token_index: Option<u32>,
        amount: U256,
        pubkey_salt_hash: Bytes32,
        is_included: bool,
        is_rejected: bool,
        meta: MetaData,
    },
    Receive {
        amount: U256,
        token_index: u32,
        from: U256,
        is_included: bool,
        is_rejected: bool,
        meta: MetaData,
    },
    Send {
        transfers: Vec<GenericTransfer>,
        is_included: bool,
        is_rejected: bool,
        meta: MetaData,
    },
}

/// Transfer without salt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GenericTransfer {
    Transfer {
        recipient: U256,
        token_index: u32,
        amount: U256,
    },
    Withdrawal {
        recipient: Address,
        token_index: u32,
        amount: U256,
    },
}

impl std::fmt::Display for GenericTransfer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericTransfer::Transfer {
                recipient,
                token_index,
                amount,
            } => write!(
                f,
                "Transfer(recipient: {}, token_index: {}, amount: {})",
                recipient.to_hex(),
                token_index,
                amount
            ),
            GenericTransfer::Withdrawal {
                recipient,
                token_index,
                amount,
            } => write!(
                f,
                "Withdrawal(recipient: {}, token_index: {}, amount: {})",
                recipient.to_hex(),
                token_index,
                amount
            ),
        }
    }
}

pub async fn fetch_history<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    key: KeySet,
) -> Result<Vec<HistoryEntry>, ClientError> {
    let user_data = client.get_user_data(key).await?;

    let mut history = Vec::new();

    // Deposits
    let all_deposit_info = fetch_deposit_info(
        &client.store_vault_server,
        &client.validity_prover,
        &client.liquidity_contract,
        key,
        0,   // set to 0 to get all deposits
        &[], // no processed deposit uuids to get all deposits
        client.config.deposit_timeout,
    )
    .await?;
    for (meta, settled) in all_deposit_info.settled {
        history.push(HistoryEntry::Deposit {
            token_type: settled.token_type,
            token_address: settled.token_address,
            token_id: settled.token_id,
            token_index: settled.token_index,
            amount: settled.amount,
            pubkey_salt_hash: settled.pubkey_salt_hash,
            is_included: user_data.processed_deposit_uuids.contains(&meta.uuid),
            is_rejected: false,
            meta,
        });
    }
    for (meta, pending) in all_deposit_info.pending {
        history.push(HistoryEntry::Deposit {
            token_type: pending.token_type,
            token_address: pending.token_address,
            token_id: pending.token_id,
            token_index: pending.token_index,
            amount: pending.amount,
            pubkey_salt_hash: pending.pubkey_salt_hash,
            is_included: false,
            is_rejected: false,
            meta,
        });
    }
    for (meta, timeout) in all_deposit_info.timeout {
        history.push(HistoryEntry::Deposit {
            token_type: timeout.token_type,
            token_address: timeout.token_address,
            token_id: timeout.token_id,
            token_index: timeout.token_index,
            amount: timeout.amount,
            pubkey_salt_hash: timeout.pubkey_salt_hash,
            is_included: false,
            is_rejected: true,
            meta,
        });
    }

    let all_transfers_info = fetch_transfer_info(
        &client.store_vault_server,
        &client.validity_prover,
        key,
        0,   // set to 0 to get all transfers
        &[], // no processed transfer uuids to get all transfers
        client.config.tx_timeout,
    )
    .await?;
    for (meta, settled) in all_transfers_info.settled {
        let transfer = settled.transfer;
        history.push(HistoryEntry::Receive {
            amount: transfer.amount,
            token_index: transfer.token_index,
            from: transfer.recipient.data,
            is_included: user_data.processed_transfer_uuids.contains(&meta.uuid),
            is_rejected: false,
            meta: meta.clone(),
        });
    }
    for (meta, pending) in all_transfers_info.pending {
        let transfer = pending.transfer;
        history.push(HistoryEntry::Receive {
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
        history.push(HistoryEntry::Receive {
            amount: transfer.amount,
            token_index: transfer.token_index,
            from: transfer.recipient.data,
            is_included: false,
            is_rejected: true,
            meta: meta.clone(),
        });
    }

    let all_tx_info = fetch_tx_info(
        &client.store_vault_server,
        &client.validity_prover,
        key,
        0,   // set to 0 to get all txs
        &[], // no processed tx uuids to get all txs
        client.config.tx_timeout,
    )
    .await?;
    for (meta, settled) in all_tx_info.settled {
        history.push(HistoryEntry::Send {
            transfers: extract_generic_transfers(settled),
            is_included: user_data.processed_tx_uuids.contains(&meta.uuid),
            is_rejected: false,
            meta,
        });
    }
    for (meta, pending) in all_tx_info.pending {
        history.push(HistoryEntry::Send {
            transfers: extract_generic_transfers(pending),
            is_included: false,
            is_rejected: false,
            meta,
        });
    }
    for (meta, timeout) in all_tx_info.timeout {
        history.push(HistoryEntry::Send {
            transfers: extract_generic_transfers(timeout),
            is_included: false,
            is_rejected: true,
            meta,
        });
    }

    // sort history
    history.sort_by_key(|entry| match entry {
        HistoryEntry::Deposit { meta, .. } => meta.timestamp,
        HistoryEntry::Receive { meta, .. } => meta.timestamp,
        HistoryEntry::Send { meta, .. } => meta.timestamp,
    });

    Ok(history)
}

fn extract_generic_transfers(tx_data: TxData<F, C, D>) -> Vec<GenericTransfer> {
    let mut transfers = Vec::new();
    for transfer in tx_data.spent_witness.transfers.iter() {
        let recipient = transfer.recipient;
        if !recipient.is_pubkey
            && recipient.data == U256::default()
            && transfer.amount == U256::default()
        {
            // dummy transfer
            continue;
        }
        if recipient.is_pubkey {
            transfers.push(GenericTransfer::Transfer {
                recipient: recipient.to_pubkey().unwrap(),
                token_index: transfer.token_index,
                amount: transfer.amount,
            });
        } else {
            transfers.push(GenericTransfer::Withdrawal {
                recipient: recipient.to_address().unwrap(),
                token_index: transfer.token_index,
                amount: transfer.amount,
            });
        }
    }
    transfers
}
