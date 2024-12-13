use intmax2_interfaces::{
    api::{
        balance_prover::interface::BalanceProverClientInterface,
        validity_prover::interface::ValidityProverClientInterface,
    },
    data::{common_tx_data::CommonTxData, deposit_data::DepositData, transfer_data::TransferData},
};
use intmax2_zkp::{
    circuits::balance::{
        balance_pis::BalancePublicInputs, balance_processor::get_prev_balance_pis,
    },
    common::{
        private_state::FullPrivateState,
        salt::Salt,
        signature::key_set::KeySet,
        witness::{
            deposit_witness::DepositWitness, private_transition_witness::PrivateTransitionWitness,
            receive_deposit_witness::ReceiveDepositWitness,
            receive_transfer_witness::ReceiveTransferWitness, transfer_witness::TransferWitness,
            tx_witness::TxWitness,
        },
    },
    ethereum_types::{bytes32::Bytes32, u256::U256},
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};

use super::error::ClientError;

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub async fn process_deposit<V: ValidityProverClientInterface, B: BalanceProverClientInterface>(
    validity_prover: &V,
    balance_processor: &B,
    key: KeySet,
    pubkey: U256,
    full_private_state: &mut FullPrivateState,
    new_salt: Salt,
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    receive_block_number: u32,
    deposit_data: &DepositData,
) -> Result<ProofWithPublicInputs<F, C, D>, ClientError> {
    // update balance proof up to the deposit block
    let before_balance_proof = update_balance_proof(
        validity_prover,
        balance_processor,
        key,
        pubkey,
        prev_balance_proof,
        receive_block_number,
    )
    .await?;

    // Generate witness
    let deposit_info = validity_prover
        .get_deposit_info(deposit_data.deposit_hash().unwrap())
        .await?
        .ok_or(ClientError::InternalError(
            "Deposit index and block number not found".to_string(),
        ))?;
    if deposit_info.block_number > receive_block_number {
        return Err(ClientError::InternalError(
            "Deposit block number is greater than receive block number".to_string(),
        ));
    }
    let deposit_merkle_proof = validity_prover
        .get_deposit_merkle_proof(receive_block_number, deposit_info.deposit_index)
        .await?;
    let deposit_witness = DepositWitness {
        deposit_salt: deposit_data.deposit_salt,
        deposit_index: deposit_info.deposit_index as u32,
        deposit: deposit_data.deposit().unwrap(),
        deposit_merkle_proof,
    };
    let deposit = deposit_data.deposit().unwrap();
    let nullifier: Bytes32 = deposit.poseidon_hash().into();
    let private_transition_witness = PrivateTransitionWitness::new(
        full_private_state,
        deposit.token_index,
        deposit.amount,
        nullifier,
        new_salt,
    )
    .map_err(|e| ClientError::WitnessGenerationError(format!("PrivateTransitionWitness {}", e)))?;
    let receive_deposit_witness = ReceiveDepositWitness {
        deposit_witness,
        private_transition_witness,
    };

    // prove deposit
    let balance_proof = balance_processor
        .prove_receive_deposit(
            key,
            pubkey,
            &receive_deposit_witness,
            &Some(before_balance_proof),
        )
        .await?;

    Ok(balance_proof)
}

pub async fn process_transfer<V: ValidityProverClientInterface, B: BalanceProverClientInterface>(
    validity_prover: &V,
    balance_processor: &B,
    key: KeySet,
    pubkey: U256,
    full_private_state: &mut FullPrivateState,
    new_salt: Salt,
    sender_balance_proof: &ProofWithPublicInputs<F, C, D>, /* sender's balance proof after
                                                            * applying tx */
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>, /* receiver's prev balance
                                                                  * proof */
    receive_block_number: u32,
    transfer_data: &TransferData<F, C, D>,
) -> Result<ProofWithPublicInputs<F, C, D>, ClientError> {
    let sender_balance_pis = BalancePublicInputs::from_pis(&sender_balance_proof.public_inputs);
    if sender_balance_pis.public_state.block_number > receive_block_number {
        return Err(ClientError::InternalError(
            "Sender's block number is greater than receive block number".to_string(),
        ));
    }

    // update balance proof up to the deposit block
    let before_balance_proof = update_balance_proof(
        validity_prover,
        balance_processor,
        key,
        pubkey,
        prev_balance_proof,
        receive_block_number,
    )
    .await?;

    // Generate witness
    let transfer_witness = TransferWitness {
        tx: transfer_data.tx_data.tx.clone(),
        transfer: transfer_data.transfer.clone(),
        transfer_index: transfer_data.transfer_index,
        transfer_merkle_proof: transfer_data.transfer_merkle_proof.clone(),
    };
    let nullifier: Bytes32 = transfer_witness.transfer.commitment().into();
    let private_transition_witness = PrivateTransitionWitness::new(
        full_private_state,
        transfer_data.transfer.token_index,
        transfer_data.transfer.amount,
        nullifier,
        new_salt,
    )
    .map_err(|e| ClientError::WitnessGenerationError(format!("PrivateTransitionWitness {}", e)))?;
    let block_merkle_proof = validity_prover
        .get_block_merkle_proof(
            receive_block_number,
            sender_balance_pis.public_state.block_number,
        )
        .await?;
    let receive_transfer_witness = ReceiveTransferWitness {
        transfer_witness,
        private_transition_witness,
        sender_balance_proof: sender_balance_proof.clone(),
        block_merkle_proof,
    };

    // prove transfer
    let balance_proof = balance_processor
        .prove_receive_transfer(
            key,
            pubkey,
            &receive_transfer_witness,
            &Some(before_balance_proof),
        )
        .await?;

    Ok(balance_proof)
}

pub async fn process_common_tx<
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
>(
    validity_prover: &V,
    balance_processor: &B,
    key: KeySet,
    sender: U256,
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    tx_block_number: u32,
    common_tx_data: &CommonTxData<F, C, D>,
) -> Result<ProofWithPublicInputs<F, C, D>, ClientError> {
    // sync check
    if tx_block_number > validity_prover.get_block_number().await? {
        return Err(ClientError::InternalError(
            "Validity prover is not up to date".to_string(),
        ));
    }
    let prev_balance_pis = get_prev_balance_pis(sender, prev_balance_proof);
    if tx_block_number <= prev_balance_pis.public_state.block_number {
        return Err(ClientError::InternalError(
            "tx block number is not greater than prev balance proof".to_string(),
        ));
    }

    // get witness
    let validity_pis = validity_prover
        .get_validity_pis(tx_block_number)
        .await?
        .ok_or(ClientError::InternalError(format!(
            "validity public inputs not found for block number {}",
            tx_block_number
        )))?;

    let sender_leaves = validity_prover
        .get_sender_leaves(tx_block_number)
        .await?
        .ok_or(ClientError::InternalError(format!(
            "sender leaves not found for block number {}",
            tx_block_number
        )))?;

    let tx_witness = TxWitness {
        validity_pis,
        sender_leaves,
        tx: common_tx_data.tx.clone(),
        tx_index: common_tx_data.tx_index,
        tx_merkle_proof: common_tx_data.tx_merkle_proof.clone(),
    };
    let update_witness = validity_prover
        .get_update_witness(
            sender,
            tx_block_number,
            prev_balance_pis.public_state.block_number,
            true,
        )
        .await?;

    // prove tx send
    let balance_proof = balance_processor
        .prove_send(
            key,
            sender,
            &tx_witness,
            &update_witness,
            &common_tx_data.spent_proof,
            prev_balance_proof,
        )
        .await?;

    Ok(balance_proof)
}

// Inner function to update balance proof
async fn update_balance_proof<V: ValidityProverClientInterface, B: BalanceProverClientInterface>(
    validity_prover: &V,
    balance_processor: &B,
    key: KeySet,
    pubkey: U256,
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    block_number: u32,
) -> Result<ProofWithPublicInputs<F, C, D>, ClientError> {
    // sync check
    if block_number > validity_prover.get_block_number().await? {
        return Err(ClientError::InternalError(
            "Validity prover is not up to date".to_string(),
        ));
    }
    if block_number == 0 {
        return Err(ClientError::InternalError(
            "Block number should be greater than 0".to_string(),
        ));
    }

    let prev_balance_pis = get_prev_balance_pis(pubkey, prev_balance_proof);
    if block_number == prev_balance_pis.public_state.block_number {
        // no need to update balance proof
        return Ok(prev_balance_proof.clone().unwrap());
    }

    // get update witness
    let update_witness = validity_prover
        .get_update_witness(
            pubkey,
            block_number,
            prev_balance_pis.public_state.block_number,
            false,
        )
        .await?;
    let last_block_number = update_witness.get_last_block_number();
    if last_block_number > block_number {
        return Err(ClientError::InternalError(
            "There is a sent tx after prev balance proof".to_string(),
        ));
    }
    let balance_proof = balance_processor
        .prove_update(key, pubkey, &update_witness, &prev_balance_proof)
        .await?;
    Ok(balance_proof)
}
