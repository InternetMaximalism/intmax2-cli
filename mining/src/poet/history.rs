use crate::common::history::{DepositEntry, SendEntry};
use intmax2_client_sdk::client::history::GenericTransfer;
use intmax2_interfaces::data::meta_data::MetaData;
use intmax2_zkp::{
    common::salt::Salt,
    ethereum_types::{address::Address, u256::U256},
};

#[derive(Debug, Clone)]
pub struct Withdrawal {
    pub recipient: Address,
    pub token_index: u32,
    pub amount: U256,
    pub salt: Salt,
    pub meta: MetaData,
}

fn extract_withdrawal_transfer(
    transfer: &GenericTransfer,
    is_included: bool,
    meta: MetaData,
) -> Option<Withdrawal> {
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
        if target_amount != U256::default() && is_included {
            return Some(Withdrawal {
                recipient,
                token_index,
                amount,
                salt: Salt::default(), // TODO
                meta: meta.clone(),
            });
        }
    }

    None
}

pub fn filter_withdrawals_from_history(
    tx_history: &[SendEntry],
) -> anyhow::Result<Vec<Withdrawal>> {
    let processed_withdrawals = tx_history
        .into_iter()
        .map(|transition| {
            let SendEntry {
                transfers,
                is_included,
                meta,
                ..
            } = transition;

            transfers
                .iter()
                .filter_map(|transfer| {
                    extract_withdrawal_transfer(transfer, *is_included, meta.clone())
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();

    Ok(processed_withdrawals)
}

fn extract_deposit(
    transition: DepositEntry,
    withdrawal: &Withdrawal,
    from_block: u32,
    to_block: u32,
) -> Option<u32> {
    let DepositEntry {
        token_index,
        amount,
        is_included,
        meta,
        ..
    } = transition;

    // TODO: Check deposit nullifier from contract
    println!(
        "Deposit: Token index: {:?}, Amount: {:?}, Included: {:?}, Block number: {:?}",
        token_index, amount, is_included, meta.block_number
    );
    if token_index == Some(withdrawal.token_index)
        && amount == withdrawal.amount
        && is_included
        && meta.block_number > Some(from_block)
        && meta.block_number <= Some(to_block)
    {
        return meta.block_number; // TODO: deposit index
    }

    None
}

/// Select the most recent deposit transaction that meets the specified conditions.
pub fn select_most_recent_deposit_from_history(
    deposit_history: &[DepositEntry],
    withdrawal: &Withdrawal,
    from_block: u32,
    to_block: u32,
) -> Option<u32> {
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
