use crate::common::history::{DepositEntry, SendEntry};
use intmax2_client_sdk::client::history::GenericTransfer;
use intmax2_interfaces::data::meta_data::MetaData;
use intmax2_zkp::{
    common::{salt::Salt, trees::tx_tree::TxMerkleProof, tx::Tx},
    constants::TX_TREE_HEIGHT,
    ethereum_types::{address::Address, bytes32::Bytes32, u256::U256},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedDeposit {
    pub sender: Address,
    // pub recipient: U256,
    pub token_index: u32,
    pub amount: U256,
    pub salt: Salt,
    pub block_number: u32,
    pub block_timestamp: u64, // UNIX timestamp seconds when the deposit was received
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessedWithdrawal {
    // pub sender: U256,
    pub recipient: Address,
    pub token_index: u32,
    pub amount: U256,
    pub salt: Salt,
    pub tx: Tx,
    pub tx_index: u32,
    pub tx_merkle_proof: TxMerkleProof,
    pub tx_tree_root: Bytes32,
    pub block_number: u32,
    pub block_timestamp: u64, // UNIX timestamp seconds when the withdrawal was processed
}

impl Default for ProcessedWithdrawal {
    fn default() -> Self {
        Self {
            // sender: U256::default(),
            recipient: Address::default(),
            token_index: 0,
            amount: U256::default(),
            salt: Salt::default(),
            tx: Tx::default(),
            tx_index: 0,
            tx_merkle_proof: TxMerkleProof::dummy(TX_TREE_HEIGHT),
            tx_tree_root: Bytes32::default(),
            block_number: 0,
            block_timestamp: 0,
        }
    }
}

fn extract_withdrawal_transfer(
    transfer: &GenericTransfer,
    tx: Tx,
    tx_index: u32,
    tx_merkle_proof: TxMerkleProof,
    tx_tree_root: Bytes32,
    is_included: bool,
    meta: MetaData,
) -> Option<ProcessedWithdrawal> {
    println!(
        "Withdrawal: Included: {:?}, Block number: {:?}",
        is_included, meta.block_number
    );
    if let GenericTransfer::Withdrawal {
        recipient,
        token_index,
        amount,
        ..
    } = transfer.clone()
    {
        let target_amount = is_mining_target(token_index, amount);
        if meta.block_number.is_some() && target_amount != U256::default() && is_included {
            return Some(ProcessedWithdrawal {
                recipient,
                token_index,
                amount,
                salt: Salt::default(), // TODO
                tx,
                tx_index,
                tx_merkle_proof,
                tx_tree_root,
                block_number: meta.block_number.unwrap(),
                block_timestamp: meta.timestamp, // TODO
            });
        }
    }

    None
}

pub fn filter_withdrawals_from_history(
    tx_history: &[SendEntry],
) -> anyhow::Result<Vec<ProcessedWithdrawal>> {
    let processed_withdrawals = tx_history
        .into_iter()
        .map(|transition| {
            let SendEntry {
                transfers,
                tx,
                tx_index,
                tx_merkle_proof,
                tx_tree_root,
                is_included,
                meta,
                ..
            } = transition;

            transfers
                .iter()
                .filter_map(|transfer| {
                    extract_withdrawal_transfer(
                        transfer,
                        *tx,
                        *tx_index,
                        tx_merkle_proof.clone(),
                        *tx_tree_root,
                        *is_included,
                        meta.clone(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();

    Ok(processed_withdrawals)
}

fn extract_deposit(
    transition: DepositEntry,
    withdrawal: &ProcessedWithdrawal,
    from_block: u32,
    to_block: u32,
) -> Option<DepositEntry> {
    let DepositEntry {
        is_included,
        token_index,
        amount,
        block_number,
        ..
    } = transition;

    // TODO: Check deposit nullifier from contract
    println!(
        "Deposit: Token index: {:?}, Amount: {:?}, Included: {:?}, Block number: {:?}",
        token_index, amount, is_included, block_number
    );
    if token_index == Some(withdrawal.token_index)
        && amount == withdrawal.amount
        && is_included
        && block_number > from_block
        && block_number <= to_block
    {
        return Some(transition); // TODO: deposit index
    }

    None
}

/// Select the most recent deposit transaction that meets the specified conditions.
pub fn select_most_recent_deposit_from_history(
    deposit_history: &[DepositEntry],
    withdrawal: &ProcessedWithdrawal,
    from_block: u32,
    to_block: u32,
) -> Option<DepositEntry> {
    let processed_deposits = deposit_history
        .iter()
        .cloned()
        .filter_map(|transition| extract_deposit(transition, withdrawal, from_block, to_block))
        .collect::<Vec<_>>();

    processed_deposits.first().cloned()
}

// TODO
fn is_mining_target(token_index: u32, amount: U256) -> U256 {
    let target_amount = U256::from(100);
    if token_index == 0 && amount == target_amount {
        return target_amount;
    }

    U256::default()
}
