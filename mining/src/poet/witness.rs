use crate::poet::client::get_client;
use anyhow::Context;
// use ethers::types::Address;
use intmax2_client_sdk::client::{
    client::Client,
    history::{fetch_history, GenericTransfer, HistoryEntry},
};
use intmax2_interfaces::{
    api::{
        balance_prover::interface::BalanceProverClientInterface,
        block_builder::interface::BlockBuilderClientInterface,
        store_vault_server::interface::StoreVaultClientInterface,
        validity_prover::interface::ValidityProverClientInterface,
        withdrawal_server::interface::WithdrawalServerClientInterface,
    },
    data::meta_data::MetaData,
};
use intmax2_zkp::{
    common::{
        block::Block, signature::key_set::KeySet, transfer::Transfer,
        trees::account_tree::AccountTree,
    },
    ethereum_types::{address::Address, u256::U256, u32limb_trait::U32LimbTrait},
};

const MIN_ELAPSED_TIME: u32 = 5;

#[derive(Debug, Clone)]
pub struct Withdrawal {
    pub recipient: Address,
    pub token_index: u32,
    pub amount: U256,
    pub meta: MetaData,
}

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

impl Default for PoetWitness {
    fn default() -> Self {
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
}

impl PoetWitness {
    pub fn get_elapsed_time(&self) -> u32 {
        let deposit_block_number = self.deposit_block.block_number;
        let withdrawal_block_number = self.withdrawal_block.block_number;

        withdrawal_block_number - deposit_block_number
    }
}

/// A proof of elapsed time from deposit_source to withdrawal_destination
#[derive(Debug, Clone)]
pub struct PoetProof {
    pub poet_witness: PoetWitness,
}

// deposit_source -> intermediates[0]
fn prove_deposit(_witness: &PoetWitness) {
    println!("Proving deposit...");
}

// intermediates[n-1] -> withdrawal_destination
fn prove_withdrawal(witness: &PoetWitness, withdrawal_destination: Address) -> anyhow::Result<()> {
    println!("Proving withdraw...");
    let generic_withdrawal_destination = witness
        .withdrawal_transfer
        .recipient
        .to_address()
        .with_context(|| "Failed to convert the withdrawal destination to an address")?;
    assert_eq!(
        generic_withdrawal_destination, withdrawal_destination,
        "The withdrawal destination is incorrect"
    );
    // let tx_inclusion_witness: TxInclusionValue = todo!();

    Ok(())
}

/// Prove the elapsed time from deposit_source to withdrawal_destination
pub fn prove_elapsed_time(witness: PoetWitness) -> anyhow::Result<PoetProof> {
    println!("Proving elapsed time...");
    assert_ne!(
        witness.deposit_source, witness.withdrawal_destination,
        "The deposit address and the withdrawal address should be different"
    );
    assert!(
        witness.get_elapsed_time() >= MIN_ELAPSED_TIME,
        "Elapsed time is too short"
    );

    prove_deposit(&witness);
    prove_withdrawal(&witness, witness.withdrawal_destination)?;

    Ok(PoetProof {
        poet_witness: witness,
    })
}

fn get_leaf_by_pubkey(account_tree: &AccountTree, pubkey: U256) -> Option<u32> {
    let account_id = account_tree.index(pubkey);
    if account_id.is_none() {
        return None;
    }

    let account_tree_leaf_just_before_withdrawal = account_tree.get_leaf(account_id.unwrap());
    let last_sent_tx_block_number = account_tree_leaf_just_before_withdrawal.value;

    Some(last_sent_tx_block_number as u32)
}

// TODO
pub fn prove_not_to_transfer(witness: &PoetWitness) {
    println!("Proving to stay...");
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
fn is_mining_target(token_index: u32, amount: U256) -> U256 {
    let target_amount = U256::from_hex("0x100").unwrap();
    if token_index == 0 && amount == target_amount {
        return target_amount;
    }

    U256::default()
}

fn filter_mining_withdrawals(
    transfers: &[GenericTransfer],
    is_included: bool,
    meta: MetaData,
) -> Vec<Withdrawal> {
    transfers
        .into_iter()
        .filter_map(|transfer| {
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
                        meta: meta.clone(),
                    });
                }
            }

            None
        })
        .collect()
}

fn select_withdrawals_from_history(history: &[HistoryEntry]) -> anyhow::Result<Vec<Withdrawal>> {
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
                return filter_mining_withdrawals(&transfers, *is_included, meta.clone());
            }

            vec![]
        })
        .flatten()
        .collect::<Vec<_>>();

    Ok(processed_withdrawals)
}

/// Select the most recent deposit transaction that meets the specified conditions.
pub fn select_most_recent_deposit_from_history(
    history: &[HistoryEntry],
    withdrawal: &Withdrawal,
    from_block: u32,
    to_block: u32,
) -> Option<u32> {
    let processed_deposits = history
        .iter()
        .cloned()
        .filter_map(|transition| {
            if let HistoryEntry::Deposit {
                token_index,
                amount,
                is_included,
                meta,
                ..
            } = transition
            {
                if token_index == Some(withdrawal.token_index)
                    && amount == withdrawal.amount
                    && !is_included
                    && meta.block_number > Some(from_block)
                    && meta.block_number <= Some(to_block)
                {
                    return meta.block_number; // TODO: deposit index
                }
            }

            None
        })
        .collect::<Vec<_>>();

    processed_deposits.first().cloned()
}

/// The block number in which the first transaction was made prior to the "withdrawal_block"
fn calculate_from_block<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    history: &[HistoryEntry],
    withdrawal_block: u32,
) -> u32 {
    let from_block = todo!();

    from_block
}

/// Generate a proof of the flow of funds from deposit_source to withdrawal_destination
/// and the elapsed time between the two transactions
pub async fn generate_witness_of_elapsed_time() -> anyhow::Result<PoetWitness> {
    println!("Generating proof of elapsed time...");

    let intermediate_account = KeySet::dummy();
    let client = get_client()?;
    let history = fetch_history(&client, intermediate_account).await?;

    // prove_withdrawal(&witness, witness.withdrawal_destination)?;
    let withdrawals = select_withdrawals_from_history(&history)?;
    let withdrawal = withdrawals.first().unwrap();

    let withdrawal_block = withdrawal
        .meta
        .block_number
        .ok_or(anyhow::anyhow!("The withdrawal block number is missing"))?;
    let to_block = withdrawal_block - MIN_ELAPSED_TIME;

    let from_block = calculate_from_block(&client, &history, withdrawal_block);
    let processed_deposit_block = select_most_recent_deposit_from_history(
        &history, withdrawal, from_block, to_block,
    )
    .ok_or(anyhow::anyhow!(
        "No deposits were made between the withdrawal block and the specified block"
    ))?;
    println!(
        "Processed deposit block number: {:?}",
        processed_deposit_block
    );

    let witness = PoetWitness {
        deposit_source: Address::default(),
        intermediate: intermediate_account.pubkey,
        withdrawal_destination: withdrawal.recipient,
        deposit_block: Block::default(),
        withdrawal_block: Block::default(),
        withdrawal_transfer: Transfer::default(),
        account_tree_just_before_withdrawal: AccountTree::new(32),
    };

    Ok(witness)
}
