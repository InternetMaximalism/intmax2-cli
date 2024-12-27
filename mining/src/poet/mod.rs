use crate::poet::client::get_client;
use ethers::types::Address;
use intmax2_client_sdk::client::history::fetch_history;
use intmax2_interfaces::data::user_data::UserData;
use intmax2_zkp::{
    common::{block::Block, signature::key_set::KeySet, trees::account_tree::AccountTree},
    ethereum_types::u256::U256,
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

// deposit_source -> intermediates[0]
pub fn prove_deposit(_witness: &PoetWitness) {
    println!("Proving deposit...");
}

// intermediates[n-1] -> withdrawal_destination
pub fn prove_withdrawal(_witness: &PoetWitness) {
    println!("Proving withdraw...");
}

pub fn prove_to_stay(witness: &PoetWitness) {
    println!("Proving to stay...");
    let account_id_option = witness
        .account_tree_just_before_withdrawal
        .index(witness.intermediate);
    if account_id_option.is_none() {
        panic!("Account ID not found");
    }
    let account_id = account_id_option.unwrap();

    let account_tree_root_just_before_withdrawal =
        witness.account_tree_just_before_withdrawal.get_root();
    let account_tree_leaf_just_before_withdrawal = witness
        .account_tree_just_before_withdrawal
        .get_leaf(account_id);
    let last_sent_tx_block_number = account_tree_leaf_just_before_withdrawal.value;

    assert_eq!(
        last_sent_tx_block_number as u32, witness.deposit_block.block_number,
        "No transfers were made between the deposit and withdrawal"
    );
}

pub async fn select_deposit_from_user_data(
    user_data: UserData,
    key: KeySet,
    deposit_amount: U256,
) -> anyhow::Result<()> {
    let client = get_client()?;
    let processed_deposits = fetch_history(&client, key).await?;

    Ok(())
}

// deposit_source -> intermediates[0]
//                -> intermediates[1]
//                -> ...
//                -> intermediates[n-1]
//                -> withdrawal_destination
pub fn prove_elapsed_time() -> PoetProof {
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
    prove_deposit(&witness);
    prove_withdrawal(&witness);
    // select_deposit_from_user_data(UserData::default(), KeySet::dummy(), U256::default());

    PoetProof {}
}
