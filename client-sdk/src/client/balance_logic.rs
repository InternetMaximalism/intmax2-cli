use intmax2_interfaces::{
    api::{
        balance_prover::interface::BalanceProverClientInterface,
        validity_prover::interface::ValidityProverClientInterface,
    },
    data::{
        common_tx_data::CommonTxData, deposit_data::DepositData, transfer_data::TransferData,
        tx_data::TxData,
    },
};
use intmax2_zkp::{
    circuits::balance::{
        balance_pis::BalancePublicInputs, balance_processor::get_prev_balance_pis,
    },
    common::{
        private_state::FullPrivateState,
        salt::Salt,
        signature::key_set::KeySet,
        transfer::Transfer,
        tx::Tx,
        witness::{
            deposit_witness::DepositWitness, private_transition_witness::PrivateTransitionWitness,
            receive_deposit_witness::ReceiveDepositWitness,
            receive_transfer_witness::ReceiveTransferWitness, spent_witness::SpentWitness,
            transfer_witness::TransferWitness, tx_witness::TxWitness,
        },
    },
    ethereum_types::{bytes32::Bytes32, u256::U256},
};
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};

use super::{
    error::ClientError,
    utils::{generate_salt, generate_transfer_tree},
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub async fn process_deposit<V: ValidityProverClientInterface, B: BalanceProverClientInterface>(
    validity_prover: &V,
    balance_prover: &B,
    key: KeySet,
    pubkey: U256,
    full_private_state: &mut FullPrivateState,
    new_salt: Salt,
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    receive_block_number: u32,
    deposit_data: &DepositData,
) -> Result<ProofWithPublicInputs<F, C, D>, ClientError> {
    // update balance proof up to the deposit block
    let before_balance_proof = update_no_send(
        validity_prover,
        balance_prover,
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
    let balance_proof = balance_prover
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
    balance_prover: &B,
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
    let before_balance_proof = update_no_send(
        validity_prover,
        balance_prover,
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
    let balance_proof = balance_prover
        .prove_receive_transfer(
            key,
            pubkey,
            &receive_transfer_witness,
            &Some(before_balance_proof),
        )
        .await?;

    Ok(balance_proof)
}

pub async fn update_send_by_sender<
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
>(
    validity_prover: &V,
    balance_prover: &B,
    key: KeySet,
    full_private_state: &mut FullPrivateState,
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    tx_block_number: u32,
    tx_data: &TxData<F, C, D>,
) -> Result<ProofWithPublicInputs<F, C, D>, ClientError> {
    // sync check
    if tx_block_number > validity_prover.get_block_number().await? {
        return Err(ClientError::InternalError(
            "Validity prover is not up to date".to_string(),
        ));
    }
    let prev_balance_pis = get_prev_balance_pis(key.pubkey, prev_balance_proof);
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
        tx: tx_data.common.tx.clone(),
        tx_index: tx_data.common.tx_index,
        tx_merkle_proof: tx_data.common.tx_merkle_proof.clone(),
    };
    let update_witness = validity_prover
        .get_update_witness(
            key.pubkey,
            tx_block_number,
            prev_balance_pis.public_state.block_number,
            true,
        )
        .await?;
    let last_block_number = update_witness.get_last_block_number();
    if last_block_number != tx_block_number {
        return Err(ClientError::InternalError(
            "last block number should be tx_block_number".to_string(),
        ));
    }
    let spent_proof =
        if tx_data.spent_witness.prev_private_state == full_private_state.to_private_state() {
            // We can use the original spent proof if prev_private_state matches
            tx_data.common.spent_proof.clone()
        } else {
            // We regenerate spent proof
            let spent_witness = generate_spent_witness(
                full_private_state,
                tx_data.spent_witness.tx.nonce,
                &tx_data.spent_witness.transfers,
            )
            .await?;
            balance_prover.prove_spent(key, &spent_witness).await?
        };

    // prove tx send
    let balance_proof = balance_prover
        .prove_send(
            key,
            key.pubkey,
            &tx_witness,
            &update_witness,
            &spent_proof,
            prev_balance_proof,
        )
        .await?;
    Ok(balance_proof)
}

/// Update balance proof to the tx specified by tx_block_number and common_tx_data by receiver side.
pub async fn update_send_by_receiver<
    V: ValidityProverClientInterface,
    B: BalanceProverClientInterface,
>(
    validity_prover: &V,
    balance_prover: &B,
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
    let last_block_number = update_witness.get_last_block_number();
    if last_block_number != tx_block_number {
        return Err(ClientError::InternalError(
            "last block number should be tx_block_number".to_string(),
        ));
    }

    // prove tx send
    let balance_proof = balance_prover
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

/// Update prev_balance_proof to block_number or do noting if already synced later than block_number.
/// Assumes that there are no send transactions between the block_number of prev_balance_proof and block_number.
async fn update_no_send<V: ValidityProverClientInterface, B: BalanceProverClientInterface>(
    validity_prover: &V,
    balance_prover: &B,
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
    let prev_block_number = prev_balance_pis.public_state.block_number;
    if block_number <= prev_block_number {
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
    if prev_block_number < last_block_number {
        return Err(ClientError::InternalError(
            "There is a sent tx after prev balance proof".to_string(),
        ));
    }
    let balance_proof = balance_prover
        .prove_update(key, pubkey, &update_witness, &prev_balance_proof)
        .await?;
    Ok(balance_proof)
}

pub async fn generate_spent_witness(
    full_private_state: &FullPrivateState,
    tx_nonce: u32,
    transfers: &[Transfer],
) -> Result<SpentWitness, ClientError> {
    let transfer_tree = generate_transfer_tree(&transfers);
    let tx = Tx {
        nonce: tx_nonce,
        transfer_tree_root: transfer_tree.get_root(),
    };
    let new_salt = generate_salt();
    let spent_witness = SpentWitness::new(
        &full_private_state.asset_tree,
        &full_private_state.to_private_state(),
        &transfer_tree.leaves(), // this is padded
        tx,
        new_salt,
    )
    .map_err(|e| {
        ClientError::WitnessGenerationError(format!("failed to generate spent witness: {}", e))
    })?;
    Ok(spent_witness)
}
