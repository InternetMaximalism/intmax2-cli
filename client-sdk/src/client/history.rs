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
    ethereum_types::{address::Address, u256::U256, u32limb_trait::U32LimbTrait},
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
        is_included: bool,
        is_rejected: bool,
        timestamp: u64, // timestamp of the block where the deposit was saved to db
    },
    Receive {
        amount: U256,
        token_index: u32,
        from: U256,
        is_included: bool,
        is_rejected: bool,
        timestamp: u64, // timestamp of the block where the receive was saved to db
    },
    Send {
        transfers: Vec<GenericTransfer>,
        is_included: bool,
        is_rejected: bool,
        timestamp: u64, // timestamp of the block where the send was saved to db
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
                    is_included: true,
                    is_rejected: false,
                    timestamp: meta.timestamp,
                });
            } else {
                history.push(HistoryEntry::Deposit {
                    token_type: decrypted.token_type,
                    token_address: decrypted.token_address,
                    token_id: decrypted.token_id,
                    token_index,
                    amount: decrypted.amount,
                    is_included: false,
                    is_rejected: true,
                    timestamp: meta.timestamp,
                });
            }
        } else {
            history.push(HistoryEntry::Deposit {
                token_type: decrypted.token_type,
                token_address: decrypted.token_address,
                token_id: decrypted.token_id,
                token_index,
                amount: decrypted.amount,
                is_included: false,
                is_rejected: false,
                timestamp: meta.timestamp,
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
                    is_included: true,
                    is_rejected: false,
                    timestamp: meta.timestamp,
                });
            } else {
                history.push(HistoryEntry::Receive {
                    amount: decrypted.transfer.amount,
                    token_index: decrypted.transfer.token_index,
                    from: decrypted.sender,
                    is_included: false,
                    is_rejected: true,
                    timestamp: meta.timestamp,
                });
            }
        } else {
            history.push(HistoryEntry::Receive {
                amount: decrypted.transfer.amount,
                token_index: decrypted.transfer.token_index,
                from: decrypted.sender,
                is_included: false,
                is_rejected: false,
                timestamp: meta.timestamp,
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
                    is_included: true,
                    is_rejected: false,
                    timestamp: meta.timestamp,
                });
            } else {
                history.push(HistoryEntry::Send {
                    transfers,
                    is_included: false,
                    is_rejected: true,
                    timestamp: meta.timestamp,
                });
            }
        } else {
            history.push(HistoryEntry::Send {
                transfers,
                is_included: false,
                is_rejected: false,
                timestamp: meta.timestamp,
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
