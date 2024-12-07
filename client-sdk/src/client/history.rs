use std::fmt::Display;

use intmax2_interfaces::{
    api::{
        balance_prover::interface::BalanceProverClientInterface,
        block_builder::interface::BlockBuilderClientInterface,
        store_vault_server::interface::{DataType, StoreVaultClientInterface},
        validity_prover::interface::ValidityProverClientInterface,
        withdrawal_server::interface::WithdrawalServerClientInterface,
    },
    data::{
        deposit_data::{DepositData, TokenType},
        transfer_data::TransferData,
        tx_data::TxData,
    },
};
use intmax2_zkp::{
    common::signature::key_set::KeySet,
    ethereum_types::{address::Address, u256::U256},
};
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use serde::{Deserialize, Serialize};

use super::{client::Client, error::ClientError};

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
        is_rejected: bool,
        timestamp: Option<u64>, // timestamp of the block where the deposit was included
    },
    Receive {
        amount: U256,
        token_index: u32,
        from: U256,
        is_rejected: bool,
        timestamp: Option<u64>, // timestamp of the block where the receive was included
    },
    Send {
        transfers: Vec<GenericTransfer>,
        is_rejected: bool,
        timestamp: Option<u64>, // timestamp of the block where the send was included
    },
}

impl Display for HistoryEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryEntry::Deposit {
                token_type,
                token_address,
                token_id,
                token_index,
                amount,
                is_rejected,
                timestamp,
            } => {
                write!(
                    f,
                    "Deposit: token_type: {:?}, token_address: {:?}, token_id: {:?}, token_index: {:?}, amount: {:?}, is_rejected: {:?}, timestamp: {:?}",
                    token_type, token_address, token_id, token_index, amount, is_rejected, timestamp
                )
            }
            HistoryEntry::Receive {
                amount,
                token_index,
                from,
                is_rejected,
                timestamp,
            } => {
                write!(
                    f,
                    "Receive: amount: {:?}, token_index: {:?}, from: {:?}, is_rejected: {:?}, timestamp: {:?}",
                    amount, token_index, from, is_rejected, timestamp
                )
            }
            HistoryEntry::Send {
                transfers,
                is_rejected,
                timestamp,
            } => {
                write!(
                    f,
                    "Send: transfers: {:?}, is_rejected: {:?}, timestamp: {:?}",
                    transfers, is_rejected, timestamp
                )
            }
        }
    }
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
    let all_deposit_data = client
        .store_vault_server
        .get_data_all_after(DataType::Deposit, key.pubkey, 0)
        .await?;
    for (meta, data) in all_deposit_data {
        let decrypted = match DepositData::decrypt(&data, key) {
            Ok(decrypted) => decrypted,
            Err(e) => {
                log::warn!("Failed to decrypt deposit data: {:?}", e);
                continue;
            }
        };
        let token_index = client
            .liquidity_contract
            .get_token_index(
                decrypted.token_type,
                decrypted.token_address,
                decrypted.token_id,
            )
            .await?;
        if meta.timestamp <= user_data.deposit_lpt {
            if user_data.processed_deposit_uuids.contains(&meta.uuid) {
                history.push(HistoryEntry::Deposit {
                    token_type: decrypted.token_type,
                    token_address: decrypted.token_address,
                    token_id: decrypted.token_id,
                    token_index,
                    amount: decrypted.amount,
                    is_rejected: false,
                    timestamp: Some(meta.timestamp),
                });
            } else {
                history.push(HistoryEntry::Deposit {
                    token_type: decrypted.token_type,
                    token_address: decrypted.token_address,
                    token_id: decrypted.token_id,
                    token_index,
                    amount: decrypted.amount,
                    is_rejected: true,
                    timestamp: None,
                });
            }
        } else {
            history.push(HistoryEntry::Deposit {
                token_type: decrypted.token_type,
                token_address: decrypted.token_address,
                token_id: decrypted.token_id,
                token_index,
                amount: decrypted.amount,
                is_rejected: false,
                timestamp: None,
            });
        }
    }

    let all_transfer_data = client
        .store_vault_server
        .get_data_all_after(DataType::Transfer, key.pubkey, 0)
        .await?;
    for (meta, data) in all_transfer_data {
        let decrypted = match TransferData::<F, C, D>::decrypt(&data, key) {
            Ok(decrypted) => decrypted,
            Err(e) => {
                log::warn!("Failed to deserialize transfer data: {:?}", e);
                continue;
            }
        };
        if meta.timestamp <= user_data.transfer_lpt {
            if user_data.processed_transfer_uuids.contains(&meta.uuid) {
                history.push(HistoryEntry::Receive {
                    amount: decrypted.transfer.amount,
                    token_index: decrypted.transfer.token_index,
                    from: decrypted.sender,
                    is_rejected: false,
                    timestamp: Some(meta.timestamp),
                });
            } else {
                history.push(HistoryEntry::Receive {
                    amount: decrypted.transfer.amount,
                    token_index: decrypted.transfer.token_index,
                    from: decrypted.sender,
                    is_rejected: true,
                    timestamp: None,
                });
            }
        } else {
            history.push(HistoryEntry::Receive {
                amount: decrypted.transfer.amount,
                token_index: decrypted.transfer.token_index,
                from: decrypted.sender,
                is_rejected: false,
                timestamp: None,
            });
        }
    }

    let all_tx_data = client
        .store_vault_server
        .get_data_all_after(DataType::Tx, key.pubkey, 0)
        .await?;
    for (meta, data) in all_tx_data {
        let tx_data = match TxData::<F, C, D>::decrypt(&data, key) {
            Ok(tx_data) => tx_data,
            Err(e) => {
                log::warn!("Failed to deserialize tx data: {:?}", e);
                continue;
            }
        };
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
        if meta.timestamp <= user_data.tx_lpt {
            if user_data.processed_tx_uuids.contains(&meta.uuid) {
                history.push(HistoryEntry::Send {
                    transfers,
                    is_rejected: false,
                    timestamp: Some(meta.timestamp),
                });
            } else {
                history.push(HistoryEntry::Send {
                    transfers,
                    is_rejected: true,
                    timestamp: None,
                });
            }
        } else {
            history.push(HistoryEntry::Send {
                transfers,
                is_rejected: false,
                timestamp: None,
            });
        }
    }

    // sort history
    history.sort_by_key(|entry| match entry {
        HistoryEntry::Deposit { timestamp, .. } => *timestamp,
        HistoryEntry::Receive { timestamp, .. } => *timestamp,
        HistoryEntry::Send { timestamp, .. } => *timestamp,
    });

    Ok(history)
}
