use crate::poet::client::get_client;
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
        block::Block, salt::Salt, signature::key_set::KeySet, trees::account_tree::AccountTree,
    },
    ethereum_types::{address::Address, u256::U256},
    utils::trees::indexed_merkle_tree::membership::MembershipProof,
};

const MIN_ELAPSED_TIME: u32 = 1;

#[derive(Debug, Clone)]
pub struct Withdrawal {
    pub recipient: Address,
    pub token_index: u32,
    pub amount: U256,
    pub salt: Salt,
    pub meta: MetaData,
}

#[derive(Debug, Clone)]
pub struct PoetWitness {
    pub deposit_source: Address,
    pub intermediate: U256,
    pub withdrawal_destination: Address,
    pub deposit_block: Block,
    pub withdrawal_block: Block,
    pub withdrawal_transfer: Withdrawal,
    pub account_membership_proof_just_before_withdrawal: MembershipProof,
}

impl Default for PoetWitness {
    fn default() -> Self {
        let account_tree = AccountTree::new(32);
        let account_membership_proof_just_before_withdrawal =
            account_tree.prove_membership(U256::default());
        Self {
            deposit_source: Address::default(),
            intermediate: U256::default(),
            withdrawal_destination: Address::default(),
            deposit_block: Block::default(),
            withdrawal_block: Block::default(),
            withdrawal_transfer: Withdrawal {
                recipient: Address::default(),
                token_index: 0,
                amount: U256::default(),
                salt: Salt::default(),
                meta: MetaData {
                    uuid: "00000000-0000-0000-0000-000000000000".to_string(),
                    block_number: Some(0),
                    timestamp: 0,
                },
            },
            account_membership_proof_just_before_withdrawal,
        }
    }
}

impl PoetWitness {
    pub fn get_elapsed_time(&self) -> u32 {
        let deposit_block_number = self.deposit_block.block_number;
        let withdrawal_block_number = self.withdrawal_block.block_number;
        println!(
            "Deposit block number: {:?}, Withdrawal block number: {:?}",
            deposit_block_number, withdrawal_block_number
        );

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
    let generic_withdrawal_destination = witness.withdrawal_transfer.recipient;
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
    let elapsed_time = witness.get_elapsed_time();
    assert!(
        elapsed_time >= MIN_ELAPSED_TIME,
        "Elapsed time is too short: elapsed block interval {} should be greater than or equal to {}",
        elapsed_time,
        MIN_ELAPSED_TIME
    );

    prove_deposit(&witness);
    prove_withdrawal(&witness, witness.withdrawal_destination)?;
    prove_not_to_transfer(&witness);

    Ok(PoetProof {
        poet_witness: witness,
    })
}

fn get_last_sent_tx_block_number(witness: &PoetWitness) -> u32 {
    let account_leaf = &witness.account_membership_proof_just_before_withdrawal.leaf;

    account_leaf.value as u32
}

// TODO
pub fn prove_not_to_transfer(witness: &PoetWitness) {
    println!("Proving to stay...");
    let last_sent_tx_block_number = get_last_sent_tx_block_number(&witness);

    assert!(
        last_sent_tx_block_number < witness.deposit_block.block_number,
        "No transfers were made between the deposit and withdrawal: last sent tx block number {} should be less than deposit block {}",
        last_sent_tx_block_number, witness.deposit_block.block_number
    );
}

// TODO
fn is_mining_target(token_index: u32, amount: U256) -> U256 {
    let target_amount = U256::from(100);
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
                        salt: Salt::default(), // TODO
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
    println!("from_block: {:?}, to_block: {:?}", from_block, to_block);
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
                println!(
                    "Token index: {:?}, Amount: {:?}, Is included: {:?}, Block number: {:?}",
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
    _client: &Client<BB, S, V, B, W>,
    _history: &[HistoryEntry],
    _withdrawal_block: u32,
) -> u32 {
    let from_block = 0;

    from_block
}

/// Generate a proof of the flow of funds from deposit_source to withdrawal_destination
/// and the elapsed time between the two transactions
pub async fn generate_witness_of_elapsed_time(
    intermediate_account: KeySet,
) -> anyhow::Result<PoetWitness> {
    println!("Generating proof of elapsed time...");
    let client = get_client()?;
    let history = fetch_history(&client, intermediate_account).await?;

    // prove_withdrawal(&witness, witness.withdrawal_destination)?;
    let withdrawals = select_withdrawals_from_history(&history)?;
    let withdrawal_transfer = withdrawals.first().unwrap();
    println!("Withdrawal: {:?}", withdrawal_transfer);

    let processed_withdrawal_block = withdrawal_transfer
        .meta
        .block_number
        .ok_or(anyhow::anyhow!("The withdrawal block number is missing"))?;
    let to_block = processed_withdrawal_block - MIN_ELAPSED_TIME;

    let prev_account_info = client
        .validity_prover
        .get_account_info_by_block_number(
            processed_withdrawal_block - 1,
            intermediate_account.pubkey,
        )
        .await?;
    let account_membership_proof_just_before_withdrawal = prev_account_info.membership_proof;
    let account_root_hash = prev_account_info.root_hash;
    account_membership_proof_just_before_withdrawal
        .verify(intermediate_account.pubkey, account_root_hash)
        .map_err(|e| anyhow::anyhow!("Failed to verify account membership proof: {:?}", e))?;

    let last_seen_block_number: u32 = prev_account_info.block_number;
    println!("Last seen block number: {:?}", last_seen_block_number);

    let from_block = calculate_from_block(&client, &history, processed_withdrawal_block);
    let processed_deposit_block = select_most_recent_deposit_from_history(
        &history,
        withdrawal_transfer,
        from_block,
        to_block,
    )
    .ok_or(anyhow::anyhow!(
        "No deposits were made between the withdrawal block and the specified block"
    ))?;
    println!(
        "Processed deposit block number: {:?}",
        processed_deposit_block
    );

    let mut deposit_block = Block::default();
    deposit_block.block_number = processed_deposit_block;
    let mut withdrawal_block = Block::default();
    withdrawal_block.block_number = processed_withdrawal_block;

    let witness = PoetWitness {
        deposit_source: Address::default(),
        intermediate: intermediate_account.pubkey,
        withdrawal_destination: withdrawal_transfer.recipient,
        deposit_block,
        withdrawal_block,
        withdrawal_transfer: withdrawal_transfer.clone(),
        account_membership_proof_just_before_withdrawal,
    };

    Ok(witness)
}
