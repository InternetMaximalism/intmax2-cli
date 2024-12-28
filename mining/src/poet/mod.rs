use crate::poet::client::get_client;
use anyhow::Context;
// use ethers::types::Address;
use intmax2_client_sdk::client::history::{fetch_history, GenericTransfer, HistoryEntry};
use intmax2_interfaces::data::{deposit_data::TokenType, meta_data::MetaData, user_data::UserData};
use intmax2_zkp::{
    common::{
        block::Block, signature::key_set::KeySet, transfer::Transfer,
        trees::account_tree::AccountTree,
    },
    ethereum_types::{address::Address, u256::U256, u32limb_trait::U32LimbTrait},
};

pub mod client;

const MIN_ELAPSED_TIME: u32 = 5;

#[derive(Debug, Clone)]
pub struct PoetWitness {
    pub deposit_source: Address,
    pub intermediate: U256,
    pub withdrawal_destination: Address,
    pub deposit_block: Block,
    pub withdrawal_block: Block,
    pub withdrawal_transfer: Transfer,
    pub account_tree_just_before_withdrawal: AccountTree,
}

impl PoetWitness {
    pub fn generate() -> Self {
        Self {
            deposit_source: Address::default(),
            intermediate: U256::default(),
            withdrawal_destination: Address::default(),
            deposit_block: Block::default(),
            withdrawal_block: Block::default(),
            withdrawal_transfer: Transfer::default(),
            account_tree_just_before_withdrawal: AccountTree::new(32),
        }
    }

    pub fn get_elapsed_time(&self) -> u32 {
        let deposit_block_number = self.deposit_block.block_number;
        let withdrawal_block_number = self.withdrawal_block.block_number;

        withdrawal_block_number - deposit_block_number
    }
}

#[derive(Debug, Clone)]
pub struct PoetProof {}

// // deposit_source -> intermediates[0]
// pub fn prove_deposit(_witness: &PoetWitness) {
//     println!("Proving deposit...");
// }

// intermediates[n-1] -> withdrawal_destination
pub fn prove_withdrawal(
    witness: &PoetWitness,
    withdrawal_destination: Address,
) -> anyhow::Result<()> {
    println!("Proving withdraw...");
    let generic_withdrawal_destination = witness
        .withdrawal_transfer
        .recipient
        .to_address()
        .with_context(|| "Failed to convert the withdrawal destination to an address")?;
    assert_eq!(
        generic_withdrawal_destination.to_bytes_be(),
        withdrawal_destination.as_fixed_bytes(),
        "The withdrawal destination is incorrect"
    );
    // let tx_inclusion_witness: TxInclusionValue = todo!();

    Ok(())
}

pub fn get_leaf_by_pubkey(account_tree: &AccountTree, pubkey: U256) -> Option<u32> {
    let account_id = account_tree.index(pubkey);
    if account_id.is_none() {
        return None;
    }

    let account_tree_leaf_just_before_withdrawal = account_tree.get_leaf(account_id.unwrap());
    let last_sent_tx_block_number = account_tree_leaf_just_before_withdrawal.value;

    Some(last_sent_tx_block_number as u32)
}

pub fn prove_to_stay(witness: &PoetWitness) {
    println!("Proving to stay...");
    let account_tree_root_just_before_withdrawal =
        witness.account_tree_just_before_withdrawal.get_root();
    let last_sent_tx_block_number = get_leaf_by_pubkey(
        &witness.account_tree_just_before_withdrawal,
        witness.intermediate,
    );

    assert!(
        last_sent_tx_block_number.is_none()
            || last_sent_tx_block_number == Some(witness.deposit_block.block_number),
        "No transfers were made between the deposit and withdrawal"
    );
}

// TODO
pub fn is_mining_target(token_index: u32, amount: U256) -> U256 {
    let target_amount = U256::from_hex("0x100").unwrap();
    if token_index == 0 && amount == target_amount {
        return target_amount;
    }

    U256::default()
}

struct Withdrawal {
    recipient: Address,
    token_index: u32,
    amount: U256,
    meta: MetaData,
}

pub async fn select_withdrawal_from_user_data(
    history: &[HistoryEntry],
) -> anyhow::Result<Vec<Withdrawal>> {
    let processed_withdrawals = history
        .into_iter()
        .map(|transition| {
            if let HistoryEntry::Send {
                transfers,
                is_included,
                meta,
                ..
            } = transition
            {
                return transfers
                    .clone()
                    .into_iter()
                    .filter_map(|transfer| {
                        if let GenericTransfer::Withdrawal {
                            recipient,
                            token_index,
                            amount,
                            ..
                        } = transfer
                        {
                            let target_amount = is_mining_target(token_index, amount);
                            if target_amount != 0 && *is_included {
                                return Some(Withdrawal {
                                    recipient,
                                    token_index,
                                    amount,
                                    meta: meta.clone(),
                                });
                            }
                        }

                        None
                    })
                    .collect::<Vec<Withdrawal>>();
            }

            vec![]
        })
        .flatten()
        .collect::<Vec<_>>();

    return Ok(processed_withdrawals);
}

pub async fn select_deposit_from_user_data(
    history: &[HistoryEntry],
    key: KeySet,
    deposit_token_type: TokenType,
    deposit_token_index: u32,
    deposit_amount: U256,
    from_block: u32,
    to_block: u32,
) -> anyhow::Result<()> {
    let client = get_client()?;
    let history = fetch_history(&client, key).await?;

    let processed_deposits = history
        .iter()
        .filter(|transition| {
            if let HistoryEntry::Deposit {
                token_type,
                token_index,
                amount,
                is_included,
                meta,
                ..
            } = transition
            {
                if *token_type == deposit_token_type
                    && *token_index == Some(deposit_token_index)
                    && *amount == deposit_amount
                    && !is_included
                    && meta.block_number > Some(from_block)
                    && meta.block_number <= Some(to_block)
                {
                    return true;
                }
            }

            false
        })
        .collect::<Vec<_>>();
    println!("Processed deposits: {:?}", processed_deposits);

    Ok(())
}

// deposit_source -> intermediates[0]
//                -> intermediates[1]
//                -> ...
//                -> intermediates[n-1]
//                -> withdrawal_destination
pub async fn prove_elapsed_time() -> anyhow::Result<PoetProof> {
    println!("Proving elapsed time...");
    let witness = PoetWitness::generate();
    assert_ne!(
        witness.deposit_source, witness.withdrawal_destination,
        "The deposit address and the withdrawal address should be different"
    );
    assert!(
        witness.get_elapsed_time() >= MIN_ELAPSED_TIME,
        "Elapsed time is too short"
    );

    let key = KeySet::dummy();
    let deposit_amount = U256::from_hex("0x100").unwrap();

    let client = get_client()?;
    let history = fetch_history(&client, key).await?;
    prove_withdrawal(&witness, witness.withdrawal_destination)?;
    select_withdrawal_from_user_data(&history).await?;
    select_deposit_from_user_data(
        &history,
        key,
        TokenType::NATIVE,
        0,
        deposit_amount,
        from_block,
        to_block,
    )
    .await?;

    Ok(PoetProof {})
}
