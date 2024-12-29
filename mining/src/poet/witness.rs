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
    circuits::validity::validity_pis::ValidityPublicInputs,
    common::{
        salt::Salt,
        signature::key_set::KeySet,
        trees::{account_tree::AccountTree, block_hash_tree::BlockHashMerkleProof},
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
    pub latest_validity_pis: ValidityPublicInputs,
    pub deposit_validity_pis: ValidityPublicInputs,
    pub deposit_block_merkle_proof: BlockHashMerkleProof,
    pub withdrawal_validity_pis: ValidityPublicInputs,
    pub withdrawal_block_merkle_proof: BlockHashMerkleProof,
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
            latest_validity_pis: ValidityPublicInputs::genesis(),
            deposit_validity_pis: ValidityPublicInputs::genesis(),
            deposit_block_merkle_proof: BlockHashMerkleProof::dummy(32),
            withdrawal_validity_pis: ValidityPublicInputs::genesis(),
            withdrawal_block_merkle_proof: BlockHashMerkleProof::dummy(32),
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
        let deposit_block_number = self.deposit_validity_pis.public_state.block_number;
        let withdrawal_block_number = self.withdrawal_validity_pis.public_state.block_number;

        withdrawal_block_number - deposit_block_number
    }
}

/// A proof of elapsed time from deposit_source to withdrawal_destination
#[derive(Debug, Clone)]
pub struct PoetProof {
    pub poet_witness: PoetWitness,
}

// deposit_source -> intermediates[0]
fn prove_deposit(witness: &PoetWitness) -> anyhow::Result<()> {
    println!("Proving deposit...");
    witness.deposit_block_merkle_proof.verify(
        &witness.deposit_validity_pis.public_state.block_hash,
        witness.deposit_validity_pis.public_state.block_number as u64,
        witness.latest_validity_pis.public_state.block_tree_root,
    )?;

    Ok(())
}

// intermediates[n-1] -> withdrawal_destination
fn prove_withdrawal(witness: &PoetWitness, withdrawal_destination: Address) -> anyhow::Result<()> {
    println!("Proving withdraw...");
    witness
        .withdrawal_block_merkle_proof
        .verify(
            &witness.withdrawal_validity_pis.public_state.block_hash,
            witness.withdrawal_validity_pis.public_state.block_number as u64,
            witness.latest_validity_pis.public_state.block_tree_root,
        )
        .map_err(|e| anyhow::anyhow!("Failed to verify withdrawal block merkle proof: {:?}", e))?;

    let generic_withdrawal_destination = witness.withdrawal_transfer.recipient;
    anyhow::ensure!(
        generic_withdrawal_destination == withdrawal_destination,
        "The withdrawal destination is incorrect: {} != {}",
        generic_withdrawal_destination,
        withdrawal_destination
    );

    witness
        .account_membership_proof_just_before_withdrawal
        .verify(
            witness.intermediate,
            witness
                .withdrawal_validity_pis
                .public_state
                .prev_account_tree_root,
        )
        .map_err(|e| anyhow::anyhow!("Failed to verify account membership proof: {:?}", e))?;

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

    prove_deposit(&witness)?;
    prove_withdrawal(&witness, witness.withdrawal_destination)?;
    prove_not_to_transfer(&witness)?;

    Ok(PoetProof {
        poet_witness: witness,
    })
}

fn get_last_sent_tx_block_number(witness: &PoetWitness) -> u32 {
    let account_leaf = &witness.account_membership_proof_just_before_withdrawal.leaf;

    account_leaf.value as u32
}

fn prove_not_to_transfer(witness: &PoetWitness) -> anyhow::Result<()> {
    println!("Proving to stay...");
    let last_sent_tx_block_number = get_last_sent_tx_block_number(&witness);

    anyhow::ensure!(
        last_sent_tx_block_number < witness.deposit_validity_pis.public_state.block_number,
        "No transfers were made between the deposit and withdrawal: last sent tx block number {} should be less than deposit block {}",
        last_sent_tx_block_number, witness.deposit_validity_pis.public_state.block_number
    );

    Ok(())
}

// TODO
fn is_mining_target(token_index: u32, amount: U256) -> U256 {
    let target_amount = U256::from(100);
    if token_index == 0 && amount == target_amount {
        return target_amount;
    }

    U256::default()
}

fn extract_withdrawal_transfer(
    transfer: &GenericTransfer,
    is_included: bool,
    meta: MetaData,
) -> Option<Withdrawal> {
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

fn filter_withdrawals_from_history(history: &[HistoryEntry]) -> anyhow::Result<Vec<Withdrawal>> {
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
                    .into_iter()
                    .filter_map(|transfer| {
                        extract_withdrawal_transfer(transfer, *is_included, meta.clone())
                    })
                    .collect();
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
                // TODO: Check deposit nullifier from contract
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
    let withdrawals = filter_withdrawals_from_history(&history)?;
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

    // let last_seen_block_number: u32 = prev_account_info.block_number;

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

    let latest_block_number = client.validity_prover.get_block_number().await?;
    let deposit_block_merkle_proof = client
        .validity_prover
        .get_block_merkle_proof(latest_block_number, processed_deposit_block)
        .await?;
    let withdrawal_block_merkle_proof = client
        .validity_prover
        .get_block_merkle_proof(latest_block_number, processed_withdrawal_block)
        .await?;
    let latest_validity_pis = client
        .validity_prover
        .get_validity_pis(latest_block_number)
        .await?
        .unwrap();
    let deposit_validity_pis = client
        .validity_prover
        .get_validity_pis(processed_deposit_block)
        .await?
        .unwrap();
    let withdrawal_validity_pis = client
        .validity_prover
        .get_validity_pis(processed_withdrawal_block)
        .await?
        .unwrap();

    let account_root_hash = prev_account_info.root_hash;
    assert_eq!(
        account_root_hash,
        withdrawal_validity_pis.public_state.prev_account_tree_root
    );

    let witness = PoetWitness {
        deposit_source: Address::default(),
        intermediate: intermediate_account.pubkey,
        withdrawal_destination: withdrawal_transfer.recipient,
        latest_validity_pis,
        deposit_validity_pis,
        deposit_block_merkle_proof,
        withdrawal_validity_pis,
        withdrawal_block_merkle_proof,
        withdrawal_transfer: withdrawal_transfer.clone(),
        account_membership_proof_just_before_withdrawal,
    };

    Ok(witness)
}
