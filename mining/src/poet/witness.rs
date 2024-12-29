use crate::{
    common::history::{fetch_deposit_history, fetch_tx_history, DepositEntry, SendEntry},
    poet::{
        client::get_client,
        history::{filter_withdrawals_from_history, select_most_recent_deposit_from_history},
        validation::{fetch_validation_data, ValidationData},
    },
};
use intmax2_client_sdk::client::client::Client;
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
    common::{salt::Salt, signature::key_set::KeySet, trees::account_tree::AccountTree},
    ethereum_types::{address::Address, u256::U256},
    utils::trees::indexed_merkle_tree::membership::MembershipProof,
};

use super::history::Withdrawal;

const MIN_ELAPSED_TIME: u32 = 1;

/// A proof of elapsed time from deposit_source to withdrawal_destination
#[derive(Debug, Clone)]
pub struct PoetProof {
    pub poet_witness: PoetWitness,
}

#[derive(Debug, Clone)]
pub struct PoetWitness {
    pub deposit_source: Address,
    pub intermediate: U256,
    pub withdrawal_destination: Address,
    pub proof_data: ValidationData,
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
            proof_data: ValidationData::default(),
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
    /// Generate a proof of the flow of funds from deposit_source to withdrawal_destination
    /// and the elapsed time between the two transactions
    pub async fn generate(intermediate_account: KeySet) -> anyhow::Result<Self> {
        println!("Generating proof of elapsed time...");
        let client = get_client()?;

        // Fetch account data and history
        let (deposit_history, tx_history) =
            fetch_account_data(&client, intermediate_account).await?;

        // prove_withdrawal(&witness, witness.withdrawal_destination)?;
        let withdrawals = filter_withdrawals_from_history(&tx_history)?;
        let withdrawal_transfer = withdrawals.first().unwrap();
        println!("Withdrawal: {:?}", withdrawal_transfer);

        let processed_withdrawal_block = withdrawal_transfer
            .meta
            .block_number
            .ok_or(anyhow::anyhow!("The withdrawal block number is missing"))?;

        let from_block = calculate_from_block(&client, &tx_history, processed_withdrawal_block);
        let to_block = processed_withdrawal_block - MIN_ELAPSED_TIME;
        let processed_deposit_block = select_most_recent_deposit_from_history(
            &deposit_history,
            withdrawal_transfer,
            from_block,
            to_block,
        )
        .ok_or(anyhow::anyhow!(
            "No deposits were made between the withdrawal block and the specified block"
        ))?;

        let proof_data =
            fetch_validation_data(&client, processed_deposit_block, processed_withdrawal_block)
                .await?;

        let prev_account_info = client
            .validity_prover
            .get_account_info_by_block_number(
                processed_withdrawal_block - 1,
                intermediate_account.pubkey,
            )
            .await?;

        // Verify the account root hash
        let account_root_hash = prev_account_info.root_hash;
        assert_eq!(
            account_root_hash,
            proof_data
                .withdrawal_validity_pis
                .public_state
                .prev_account_tree_root
        );

        Ok(Self {
            deposit_source: Address::default(),
            intermediate: intermediate_account.pubkey,
            withdrawal_destination: withdrawal_transfer.recipient,
            proof_data,
            withdrawal_transfer: withdrawal_transfer.clone(),
            account_membership_proof_just_before_withdrawal: prev_account_info.membership_proof,
        })
    }

    /// deposit_source -> intermediate
    fn prove_deposit(&self) -> anyhow::Result<()> {
        println!("Proving deposit...");
        let ValidationData {
            deposit_block_merkle_proof,
            deposit_validity_pis,
            latest_validity_pis,
            ..
        } = &self.proof_data;

        deposit_block_merkle_proof.verify(
            &deposit_validity_pis.public_state.block_hash,
            deposit_validity_pis.public_state.block_number as u64,
            latest_validity_pis.public_state.block_tree_root,
        )?;

        Ok(())
    }

    /// intermediate -> withdrawal_destination
    fn prove_withdrawal(&self, withdrawal_destination: Address) -> anyhow::Result<()> {
        println!("Proving withdraw...");
        let ValidationData {
            withdrawal_block_merkle_proof,
            withdrawal_validity_pis,
            latest_validity_pis,
            ..
        } = &self.proof_data;

        withdrawal_block_merkle_proof
            .verify(
                &withdrawal_validity_pis.public_state.block_hash,
                withdrawal_validity_pis.public_state.block_number as u64,
                latest_validity_pis.public_state.block_tree_root,
            )
            .map_err(|e| {
                anyhow::anyhow!("Failed to verify withdrawal block merkle proof: {:?}", e)
            })?;

        let generic_withdrawal_destination = self.withdrawal_transfer.recipient;
        anyhow::ensure!(
            generic_withdrawal_destination == withdrawal_destination,
            "The withdrawal destination is incorrect: {} != {}",
            generic_withdrawal_destination,
            withdrawal_destination
        );

        // let ethereum_private_key = "0x";
        // let ethereum_transaction_count: u64 = 0;
        // let actual_deposit_salt =
        //     generate_deterministic_salt(ethereum_private_key, ethereum_transaction_count);
        // let deposit_hash = deposit_nullifier_hash;

        self.account_membership_proof_just_before_withdrawal
            .verify(
                self.intermediate,
                withdrawal_validity_pis.public_state.prev_account_tree_root,
            )
            .map_err(|e| anyhow::anyhow!("Failed to verify account membership proof: {:?}", e))?;

        Ok(())
    }

    fn prove_not_to_transfer(&self) -> anyhow::Result<()> {
        println!("Proving not to transfer...");
        let deposit_validity_pis = &self.proof_data.deposit_validity_pis;

        let last_sent_tx_block_number = self.get_last_sent_tx_block_number();

        anyhow::ensure!(
            last_sent_tx_block_number < deposit_validity_pis.public_state.block_number,
            "No transfers were made between the deposit and withdrawal: last sent tx block number {} should be less than deposit block {}",
            last_sent_tx_block_number, deposit_validity_pis.public_state.block_number
        );

        Ok(())
    }

    /// Prove the elapsed time from deposit_source to withdrawal_destination
    pub fn prove_elapsed_time(self) -> anyhow::Result<PoetProof> {
        println!("Proving elapsed time...");
        assert_ne!(
            self.deposit_source, self.withdrawal_destination,
            "The deposit address and the withdrawal address should be different"
        );
        let elapsed_time = self.get_elapsed_time();
        assert!(
            elapsed_time >= MIN_ELAPSED_TIME,
            "Elapsed time is too short: elapsed block interval {} should be greater than or equal to {}",
            elapsed_time,
            MIN_ELAPSED_TIME
        );

        self.prove_deposit()?;
        self.prove_withdrawal(self.withdrawal_destination)?;
        self.prove_not_to_transfer()?;

        Ok(PoetProof { poet_witness: self })
    }

    pub fn get_elapsed_time(&self) -> u32 {
        let ValidationData {
            deposit_validity_pis,
            withdrawal_validity_pis,
            ..
        } = &self.proof_data;

        let deposit_block_number = deposit_validity_pis.public_state.block_number;
        let withdrawal_block_number = withdrawal_validity_pis.public_state.block_number;

        withdrawal_block_number - deposit_block_number
    }

    pub fn get_last_sent_tx_block_number(&self) -> u32 {
        let account_leaf = &self.account_membership_proof_just_before_withdrawal.leaf;

        account_leaf.value as u32
    }
}

async fn fetch_account_data<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    intermediate_account: KeySet,
) -> anyhow::Result<(Vec<DepositEntry>, Vec<SendEntry>)> {
    let user_data = client.get_user_data(intermediate_account).await?;
    // let history = fetch_history(&client, intermediate_account).await?;
    let deposit_history = fetch_deposit_history(
        &client,
        intermediate_account,
        user_data.processed_deposit_uuids,
    )
    .await?;
    let tx_history =
        fetch_tx_history(&client, intermediate_account, user_data.processed_tx_uuids).await?;

    Ok((deposit_history, tx_history))
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
    _tx_history: &[SendEntry],
    _withdrawal_block: u32,
) -> u32 {
    let from_block = 0;

    from_block
}
