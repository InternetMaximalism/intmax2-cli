use crate::{
    common::history::{fetch_deposit_history, fetch_tx_history, DepositEntry, SendEntry},
    poet::{
        client::get_client,
        history::{
            filter_withdrawals_from_history, select_most_recent_deposit_from_history,
            ReceivedDeposit,
        },
        nullifier::get_common_nullifier,
        validation::{fetch_validation_data, ValidationData},
    },
};
use ethers::{
    contract::LogMeta,
    types::{H160, H256},
};
use intmax2_client_sdk::{
    client::client::Client,
    external_api::{
        contract::{liquidity_contract::DepositedFilter, utils::get_latest_block_number},
        utils::retry::with_retry,
    },
};
use intmax2_interfaces::api::{
    balance_prover::interface::BalanceProverClientInterface,
    block_builder::interface::BlockBuilderClientInterface,
    store_vault_server::interface::StoreVaultClientInterface,
    validity_prover::interface::ValidityProverClientInterface,
    withdrawal_server::interface::WithdrawalServerClientInterface,
};
use intmax2_zkp::{
    circuits::balance::send::tx_inclusion_circuit::TxInclusionPublicInputs,
    common::{
        block::Block,
        generic_address::{self, GenericAddress},
        signature::key_set::KeySet,
        transfer::Transfer,
        withdrawal::{get_withdrawal_nullifier, Withdrawal},
        witness::{tx_witness::TxWitness, update_witness::UpdateWitness},
    },
    ethereum_types::{address::Address, u256::U256, u32limb_trait::U32LimbTrait},
    utils::{conversion::ToU64, trees::indexed_merkle_tree::membership::MembershipProof},
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use serde::{Deserialize, Serialize};

use super::{
    blockchain::{get_rpc_url, get_start_block_number},
    history::ProcessedWithdrawal,
};

// const MIN_ELAPSED_TIME: u32 = 60 * 60 * 48; // 2 days
const MIN_ELAPSED_TIME: u32 = 60; // 1 minutes

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

/// A proof of elapsed time from deposit_source to withdrawal_destination
#[derive(Debug, Clone)]
pub struct PoetProof {
    pub poet_witness: PoetValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoetValue {
    pub deposit_source: Address,
    pub intermediate: U256,
    pub withdrawal_destination: Address,
    pub proof_data: ValidationData,
    pub deposit_transfer: ReceivedDeposit,
    pub withdrawal_transfer: ProcessedWithdrawal,
    pub deposit_block: Block,
    pub withdrawal_block: Block,
    pub withdrawal_tx_witness: TxWitness,
    pub withdrawal_update_witness: UpdateWitness<F, C, D>,
    pub account_membership_proof_just_before_withdrawal: MembershipProof,
}

const EVENT_BLOCK_RANGE: u64 = 50000;

// pub fn get_pubkey_salt_hash(pubkey: U256, salt: Salt) -> Bytes32 {
//     let input = vec![pubkey.to_u64_vec(), salt.to_u64_vec()].concat();
//     let hash = PoseidonHashOut::hash_inputs_u64(&input);
//     hash.into()
// }

async fn fetch_deposited_event<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    pubkey_salt_hash: H256,
) -> anyhow::Result<(Vec<(DepositedFilter, LogMeta)>, Vec<H160>)> {
    let liquidity_contract = client.liquidity_contract.get_contract().await?;

    let rpc_url = get_rpc_url()?;
    let mut events = Vec::new();
    let mut from_block = get_start_block_number()
        .await
        .map_err(|_| anyhow::anyhow!("failed to get start block number"))?;
    loop {
        println!("get_deposited_event_by_sender: from_block={}", from_block);
        let new_events = with_retry(|| async {
            liquidity_contract
                .deposited_filter()
                .address(liquidity_contract.address().into())
                .topic3(pubkey_salt_hash)
                .from_block(from_block)
                .to_block(from_block + EVENT_BLOCK_RANGE)
                .query_with_meta()
                .await
        })
        .await
        .map_err(|_| anyhow::anyhow!("failed to get deposited event"))?;
        events.extend(new_events);
        let latest_block_number = get_latest_block_number(&rpc_url).await?;
        from_block += EVENT_BLOCK_RANGE;
        if from_block > latest_block_number {
            break;
        }
    }

    let mut senders = vec![];
    for (filter, _) in &events {
        let deposit_data = liquidity_contract
            .get_deposit_data(filter.deposit_id)
            .await?;
        let deposit_sender = deposit_data.sender;
        senders.push(deposit_sender);
    }

    Ok((events, senders))
}

impl PoetValue {
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

        // let processed_withdrawal_block_time = withdrawal_transfer.block_timestamp;
        let processed_withdrawal_block_number = withdrawal_transfer.block_number;

        let from_block =
            calculate_from_block(&client, &tx_history, processed_withdrawal_block_number);
        let to_block = processed_withdrawal_block_number - 1;
        let processed_deposit_transition = select_most_recent_deposit_from_history(
            &deposit_history,
            withdrawal_transfer,
            from_block,
            to_block,
        )
        .ok_or(anyhow::anyhow!(
            "No deposits were made between the withdrawal block and the specified block"
        ))?;

        let processed_deposit_block_number = processed_deposit_transition.block_number;
        let proof_data = fetch_validation_data(
            &client,
            processed_deposit_block_number,
            processed_withdrawal_block_number,
        )
        .await?;

        let recipient_salt_hash =
            H256::from_slice(&processed_deposit_transition.pubkey_salt_hash.to_bytes_be());
        let (deposited_events, senders) =
            fetch_deposited_event(&client, recipient_salt_hash).await?;
        if deposited_events.is_empty() {
            anyhow::bail!("No deposited events found for the recipient salt hash");
        }

        let (deposit_event, _) = deposited_events.first().unwrap();
        let deposit_sender = senders.first().unwrap();

        // let deposit_leaf = (
        //     Deposit {
        //         pubkey_salt_hash: processed_deposit_transition.pubkey_salt_hash,
        //         token_index: deposit_event.token_index,
        //         amount: processed_deposit_transition.amount,
        //     },
        //     deposit_sender,
        // );
        // let deposit_hash = deposit_leaf.0.hash();
        // assert_eq!(
        //     actual_deposit_hash,
        //     proof_data.deposit_validity_pis.public_state.deposit_nullifier_hash
        // );

        let prev_account_info = client
            .validity_prover
            .get_account_info_by_block_number(
                processed_withdrawal_block_number - 1,
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

        let deposit_source = Address::from_bytes_be(deposit_sender.as_bytes());
        let deposit_transfer = ReceivedDeposit {
            sender: deposit_source,
            // recipient: intermediate_account.pubkey,
            token_index: deposit_event.token_index,
            amount: processed_deposit_transition.amount,
            salt: processed_deposit_transition.salt,
            block_number: processed_deposit_transition.block_number,
            block_timestamp: processed_deposit_transition.timestamp, // TODO
        };
        let (deposit_block, withdrawal_block) = get_deposit_and_withdrawal_block(
            &client,
            processed_deposit_block_number,
            processed_withdrawal_block_number,
        )
        .await?;

        let latest_block_number = client.validity_prover.get_block_number().await?;
        let validity_pis = client
            .validity_prover
            .get_validity_pis(latest_block_number)
            .await?
            .ok_or(anyhow::anyhow!(format!(
                "validity public inputs not found for block number {}",
                latest_block_number
            )))?;
        let sender_leaves = client
            .validity_prover
            .get_sender_leaves(latest_block_number)
            .await?
            .ok_or(anyhow::anyhow!(format!(
                "sender leaves not found for block number {}",
                latest_block_number
            )))?;
        let withdrawal_tx_witness = TxWitness {
            validity_pis,
            sender_leaves,
            tx: withdrawal_transfer.tx.clone(),
            tx_index: withdrawal_transfer.tx_index,
            tx_merkle_proof: withdrawal_transfer.tx_merkle_proof.clone(),
        };
        let withdrawal_update_witness = client
            .validity_prover
            .get_update_witness(
                intermediate_account.pubkey,
                latest_block_number,
                withdrawal_block.block_number,
                true,
            )
            .await?;

        Ok(Self {
            deposit_source,
            intermediate: intermediate_account.pubkey,
            withdrawal_destination: withdrawal_transfer.recipient,
            proof_data,
            deposit_transfer,
            withdrawal_transfer: withdrawal_transfer.clone(),
            deposit_block,
            withdrawal_block,
            withdrawal_tx_witness,
            withdrawal_update_witness,
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
    fn prove_withdrawal(
        &self,
        withdrawal_destination: Address,
        single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
        withdrawal_tx_inclusion_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        println!("Proving withdraw...");
        let ValidationData {
            // withdrawal_block_merkle_proof,
            withdrawal_validity_pis,
            // latest_validity_pis,
            ..
        } = &self.proof_data;

        let withdrawal_tx_inclusion_pis = TxInclusionPublicInputs::from_u64_slice(
            &withdrawal_tx_inclusion_proof.public_inputs.to_u64_vec(),
        );

        let current_block_hash = withdrawal_tx_inclusion_pis.new_public_state.block_hash;
        let withdrawal_tx = &withdrawal_tx_inclusion_pis.tx;
        if current_block_hash == self.withdrawal_block.hash() {
            println!("Withdrawal transaction is included in the block");
        } else {
            panic!("Withdrawal transaction is not included in the block");
        }

        // withdrawal_block_merkle_proof
        //     .verify(
        //         &withdrawal_validity_pis.public_state.block_hash,
        //         withdrawal_validity_pis.public_state.block_number as u64,
        //         latest_validity_pis.public_state.block_tree_root,
        //     )
        //     .map_err(|e| {
        //         anyhow::anyhow!("Failed to verify withdrawal block merkle proof: {:?}", e)
        //     })?;

        let generic_withdrawal_destination = self.withdrawal_transfer.recipient;
        anyhow::ensure!(
            generic_withdrawal_destination == withdrawal_destination,
            "The withdrawal destination is incorrect: {} != {}",
            generic_withdrawal_destination,
            withdrawal_destination
        );

        let withdrawal_common_nullifier = get_common_nullifier(
            self.withdrawal_transfer.recipient,
            self.withdrawal_transfer.token_index,
            self.withdrawal_transfer.amount.clone(),
            self.withdrawal_transfer.salt.clone(),
        );
        let withdrawal =
            Withdrawal::from_u64_slice(&single_withdrawal_proof.public_inputs.to_u64_vec());
        println!("Withdrawal_transfer: {:?}", self.withdrawal_transfer);
        println!("Withdrawal: {:?}", withdrawal);
        anyhow::ensure!(
            withdrawal.nullifier == withdrawal_common_nullifier,
            "The withdrawal nullifier is incorrect: {} != {}",
            withdrawal.nullifier,
            withdrawal_common_nullifier
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
    pub fn prove_elapsed_time(
        self,
        single_withdrawal_proof: &ProofWithPublicInputs<F, C, D>,
        tx_inclusion_proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<PoetProof> {
        println!("Proving elapsed time...");
        assert_ne!(
            self.deposit_source, self.withdrawal_destination,
            "The deposit address and the withdrawal address should be different"
        );
        let elapsed_time = self.get_elapsed_time();
        println!("Elapsed time: {}", elapsed_time);
        assert!(
            elapsed_time >= MIN_ELAPSED_TIME,
            "Elapsed time is too short: elapsed block interval {} should be greater than or equal to {}",
            elapsed_time,
            MIN_ELAPSED_TIME
        );

        self.prove_deposit()?;
        self.prove_withdrawal(
            self.withdrawal_destination,
            &single_withdrawal_proof,
            &tx_inclusion_proof,
        )?;
        self.prove_not_to_transfer()?;

        Ok(PoetProof { poet_witness: self })
    }

    pub fn get_elapsed_time(&self) -> u32 {
        let deposit_block_time = self.deposit_block.block_time_since_genesis;
        let withdrawal_block_time = self.withdrawal_block.block_time_since_genesis;

        withdrawal_block_time - deposit_block_time
    }

    pub fn get_last_sent_tx_block_number(&self) -> u32 {
        if !self
            .account_membership_proof_just_before_withdrawal
            .is_included
        {
            return 0;
        }

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

async fn get_deposit_and_withdrawal_block<
    BB: BlockBuilderClientInterface,
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
    W: WithdrawalServerClientInterface,
>(
    client: &Client<BB, S, V, B, W>,
    processed_deposit_block_number: u32,
    processed_withdrawal_block_number: u32,
) -> anyhow::Result<(Block, Block)> {
    let rollup_deployed_block_number = client.rollup_contract.deployed_block_number;
    let (posted_events, _) = client
        .rollup_contract
        .get_blocks_posted_event(rollup_deployed_block_number)
        .await?;
    let deposit_block_with_meta = posted_events
        .iter()
        .find(|event| event.block_number == processed_deposit_block_number)
        .ok_or(anyhow::anyhow!(
            "Deposit block not found: block number {}",
            processed_deposit_block_number
        ))?;
    let deposit_block = Block {
        prev_block_hash: deposit_block_with_meta.prev_block_hash,
        deposit_tree_root: deposit_block_with_meta.deposit_tree_root,
        signature_hash: deposit_block_with_meta.signature_hash,
        block_number: deposit_block_with_meta.block_number,
        block_time_since_genesis: deposit_block_with_meta.block_time_since_genesis,
    };
    let withdrawal_block_with_meta = posted_events
        .iter()
        .find(|event| event.block_number == processed_withdrawal_block_number)
        .ok_or(anyhow::anyhow!(
            "Withdrawal block not found: block number {}",
            processed_withdrawal_block_number
        ))?;
    let withdrawal_block = Block {
        prev_block_hash: withdrawal_block_with_meta.prev_block_hash,
        deposit_tree_root: withdrawal_block_with_meta.deposit_tree_root,
        signature_hash: withdrawal_block_with_meta.signature_hash,
        block_number: withdrawal_block_with_meta.block_number,
        block_time_since_genesis: withdrawal_block_with_meta.block_time_since_genesis,
    };

    Ok((deposit_block, withdrawal_block))
}
