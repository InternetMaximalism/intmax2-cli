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
    error::SyncError,
    utils::{generate_salt, generate_transfer_tree},
};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

pub async fn receive_deposit<V: ValidityProverClientInterface, B: BalanceProverClientInterface>(
    validity_prover: &V,
    balance_prover: &B,
    key: KeySet,
    full_private_state: &mut FullPrivateState,
    new_salt: Salt,
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    deposit_data: &DepositData,
) -> Result<ProofWithPublicInputs<F, C, D>, SyncError> {
    let prev_balance_pis = get_prev_balance_pis(key.pubkey, prev_balance_proof);
    let receive_block_number = prev_balance_pis.public_state.block_number;
    // Generate witness
    let deposit_info = validity_prover
        .get_deposit_info(deposit_data.deposit_hash().unwrap())
        .await?
        .ok_or(SyncError::DepositInfoNotFound(
            deposit_data.deposit_hash().unwrap(),
        ))?;
    if receive_block_number < deposit_info.block_number {
        return Err(SyncError::InternalError(
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
    .map_err(|e| SyncError::WitnessGenerationError(format!("PrivateTransitionWitness {}", e)))?;
    let receive_deposit_witness = ReceiveDepositWitness {
        deposit_witness,
        private_transition_witness,
    };

    // prove deposit
    let balance_proof = balance_prover
        .prove_receive_deposit(
            key,
            key.pubkey,
            &receive_deposit_witness,
            &prev_balance_proof,
        )
        .await?;

    Ok(balance_proof)
}

pub async fn receive_transfer<V: ValidityProverClientInterface, B: BalanceProverClientInterface>(
    validity_prover: &V,
    balance_prover: &B,
    key: KeySet,
    full_private_state: &mut FullPrivateState,
    new_salt: Salt,
    sender_balance_proof: &ProofWithPublicInputs<F, C, D>, /* sender's balance proof after
                                                            * applying tx */
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>, /* receiver's prev balance
                                                                  * proof */
    transfer_data: &TransferData<F, C, D>,
) -> Result<ProofWithPublicInputs<F, C, D>, SyncError> {
    let prev_balance_pis = get_prev_balance_pis(key.pubkey, prev_balance_proof);
    let receive_block_number = prev_balance_pis.public_state.block_number;
    let sender_balance_pis = BalancePublicInputs::from_pis(&sender_balance_proof.public_inputs);
    if receive_block_number < prev_balance_pis.public_state.block_number {
        return Err(SyncError::InternalError(
            "receive block number is not greater than prev balance proof".to_string(),
        ));
    }
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
    .map_err(|e| SyncError::WitnessGenerationError(format!("PrivateTransitionWitness {}", e)))?;
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
            key.pubkey,
            &receive_transfer_witness,
            &prev_balance_proof,
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
) -> Result<ProofWithPublicInputs<F, C, D>, SyncError> {
    // sync check
    let validity_prover_block_number = validity_prover.get_block_number().await?;
    if tx_block_number > validity_prover_block_number {
        return Err(SyncError::ValidityProverIsNotUpToDate {
            validity_prover_block_number,
            block_number: tx_block_number,
        });
    }
    let prev_balance_pis = get_prev_balance_pis(key.pubkey, prev_balance_proof);
    if tx_block_number <= prev_balance_pis.public_state.block_number {
        return Err(SyncError::InternalError(
            "tx block number is not greater than prev balance proof".to_string(),
        ));
    }
    if prev_balance_pis.private_commitment != full_private_state.to_private_state().commitment() {
        return Err(SyncError::InternalError(
            "prev balance proof private commitment is not equal to full private state commitment"
                .to_string(),
        ));
    }

    // get witness
    let validity_pis = validity_prover
        .get_validity_pis(tx_block_number)
        .await?
        .ok_or(SyncError::InternalError(format!(
            "validity public inputs not found for block number {}",
            tx_block_number
        )))?;

    let sender_leaves = validity_prover
        .get_sender_leaves(tx_block_number)
        .await?
        .ok_or(SyncError::InternalError(format!(
            "sender leaves not found for block number {}",
            tx_block_number
        )))?;
    let tx_witness = TxWitness {
        validity_pis: validity_pis.clone(),
        sender_leaves: sender_leaves.clone(),
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
    log::info!(
        "update_witness.last_block_number: {}, tx_block_number: {}",
        update_witness.get_last_block_number(),
        tx_block_number
    );

    let sender_leaf = sender_leaves
        .iter()
        .find(|leaf| leaf.sender == key.pubkey)
        .ok_or(SyncError::InternalError(
            "sender leaf not found in sender leaves".to_string(),
        ))?;
    // update private state only if sender leaf has returned signature and validity_pis is valid
    let update_private_state = sender_leaf.did_return_sig && validity_pis.is_valid_block;

    let spent_proof = if tx_data.spent_witness.prev_private_state
        == full_private_state.to_private_state()
        && tx_data.spent_witness.tx.nonce == full_private_state.nonce
    {
        // We can use the original spent proof if prev_private_state matches
        let spent_proof = tx_data.common.spent_proof.clone();

        // update private state
        if update_private_state {
            tx_data
                .spent_witness
                .update_private_state(full_private_state)
                .map_err(|e| SyncError::FailedToUpdatePrivateState(e.to_string()))?;
        }
        spent_proof
    } else {
        // We regenerate spent proof
        let spent_witness = generate_spent_witness(
            full_private_state,
            tx_data.spent_witness.tx.nonce,
            &tx_data.spent_witness.transfers,
        )
        .await?;
        // update private state
        if update_private_state {
            spent_witness
                .update_private_state(full_private_state)
                .map_err(|e| SyncError::FailedToUpdatePrivateState(e.to_string()))?;
        }
        balance_prover.prove_spent(key, &spent_witness).await?
    };
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
    let balance_pis = BalancePublicInputs::from_pis(&balance_proof.public_inputs);
    if balance_pis.private_commitment != full_private_state.to_private_state().commitment() {
        return Err(SyncError::InternalError(format!(
            "balance proof new private commitment {} is not equal to full private state commitment{}",
            balance_pis.private_commitment, full_private_state.to_private_state().commitment(
        ))
        ));
    }
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
) -> Result<ProofWithPublicInputs<F, C, D>, SyncError> {
    // sync check
    let validity_prover_block_number = validity_prover.get_block_number().await?;
    if tx_block_number > validity_prover_block_number {
        return Err(SyncError::ValidityProverIsNotUpToDate {
            validity_prover_block_number,
            block_number: tx_block_number,
        });
    }
    let prev_balance_pis = get_prev_balance_pis(sender, prev_balance_proof);
    let prev_block_number = prev_balance_pis.public_state.block_number;
    if tx_block_number <= prev_block_number {
        return Err(SyncError::InternalError(
            "tx block number is not greater than prev balance proof".to_string(),
        ));
    }

    // get witness
    let validity_pis = validity_prover
        .get_validity_pis(tx_block_number)
        .await?
        .ok_or(SyncError::InternalError(format!(
            "validity public inputs not found for block number {}",
            tx_block_number
        )))?;

    let sender_leaves = validity_prover
        .get_sender_leaves(tx_block_number)
        .await?
        .ok_or(SyncError::InternalError(format!(
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
    log::info!(
        "prev_block_number: {}, update_witness.last_block_number: {}, tx_block_number: {}",
        prev_block_number,
        last_block_number,
        tx_block_number
    );
    if prev_block_number < last_block_number {
        return Err(SyncError::SenderLastBlockNumberError {
            balance_proof_block_number: prev_block_number,
            last_block_number,
        });
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
pub async fn update_no_send<V: ValidityProverClientInterface, B: BalanceProverClientInterface>(
    validity_prover: &V,
    balance_prover: &B,
    key: KeySet,
    prev_balance_proof: &Option<ProofWithPublicInputs<F, C, D>>,
    to_block_number: u32,
) -> Result<ProofWithPublicInputs<F, C, D>, SyncError> {
    // sync check
    let validity_prover_block_number = validity_prover.get_block_number().await?;
    if to_block_number > validity_prover_block_number {
        return Err(SyncError::ValidityProverIsNotUpToDate {
            validity_prover_block_number,
            block_number: to_block_number,
        });
    }
    if to_block_number == 0 {
        return Err(SyncError::InternalError(
            "Block number should be greater than 0".to_string(),
        ));
    }
    let prev_balance_pis = get_prev_balance_pis(key.pubkey, prev_balance_proof);
    let prev_block_number = prev_balance_pis.public_state.block_number;
    if to_block_number <= prev_block_number {
        // no need to update balance proof
        return Ok(prev_balance_proof.clone().unwrap());
    }

    // get update witness
    let update_witness = validity_prover
        .get_update_witness(
            key.pubkey,
            to_block_number,
            prev_balance_pis.public_state.block_number,
            false,
        )
        .await?;
    let last_block_number = update_witness.get_last_block_number();
    if prev_block_number < last_block_number {
        return Err(SyncError::InternalError(format!(
            "prev_block_number {} is less than last_block_number {}",
            prev_block_number, last_block_number
        )));
    }
    let balance_proof = balance_prover
        .prove_update(key, key.pubkey, &update_witness, &prev_balance_proof)
        .await?;
    Ok(balance_proof)
}

pub async fn generate_spent_witness(
    full_private_state: &FullPrivateState,
    tx_nonce: u32,
    transfers: &[Transfer],
) -> Result<SpentWitness, SyncError> {
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
        SyncError::WitnessGenerationError(format!("failed to generate spent witness: {}", e))
    })?;
    Ok(spent_witness)
}
