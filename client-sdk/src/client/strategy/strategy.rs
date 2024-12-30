use intmax2_interfaces::{
    api::{
        store_vault_server::interface::StoreVaultClientInterface,
        validity_prover::interface::ValidityProverClientInterface,
    },
    data::{
        deposit_data::DepositData,
        meta_data::MetaData,
        transfer_data::TransferData,
        tx_data::TxData,
        user_data::{Balances, UserData},
    },
};
use itertools::Itertools;
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};

use intmax2_zkp::common::signature::key_set::KeySet;

use crate::{
    client::error::ClientError, external_api::contract::liquidity_contract::LiquidityContract,
};

use super::{deposit::fetch_deposit_info, transfer::fetch_transfer_info, tx::fetch_tx_info};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

// Next sync action
#[derive(Debug, Clone)]
pub enum Action {
    Receive {
        receives: Vec<ReceiveAction>,
        new_deposit_lpt: u64,
        new_transfer_lpt: u64,
    },
    Tx(MetaData, TxData<F, C, D>),              // Send tx
    PendingReceives(MetaData, TxData<F, C, D>), // Pending receives to proceed the next tx
    PendingTx(MetaData, TxData<F, C, D>),       // Pending tx
}

#[derive(Debug, Clone)]
pub enum ReceiveAction {
    Deposit(MetaData, DepositData),
    Transfer(MetaData, TransferData<F, C, D>),
}

impl ReceiveAction {
    pub fn meta(&self) -> &MetaData {
        match self {
            ReceiveAction::Deposit(meta, _) => meta,
            ReceiveAction::Transfer(meta, _) => meta,
        }
    }

    pub fn apply_to_balances(&self, balances: &mut Balances) {
        match self {
            ReceiveAction::Deposit(_, data) => {
                balances.add_deposit(data);
            }
            ReceiveAction::Transfer(_, data) => {
                balances.add_transfer(data);
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PendingInfo {
    pub pending_deposits: Vec<(MetaData, DepositData)>,
    pub pending_transfers: Vec<(MetaData, TransferData<F, C, D>)>,
}

/// Determine the sequence of receives/send tx to be incorporated into the balance proof
pub async fn determine_sequence<S: StoreVaultClientInterface, V: ValidityProverClientInterface>(
    store_vault_server: &S,
    validity_prover: &V,
    liquidity_contract: &LiquidityContract,
    key: KeySet,
    deposit_timeout: u64,
    tx_timeout: u64,
) -> Result<(Vec<Action>, PendingInfo), ClientError> {
    log::info!("determine_sequence");
    let user_data = store_vault_server
        .get_user_data(key.pubkey)
        .await?
        .map(|encrypted| UserData::decrypt(&encrypted, key))
        .transpose()
        .map_err(|e| ClientError::DecryptionError(e.to_string()))?
        .unwrap_or(UserData::new(key.pubkey));
    let mut balances = user_data.balances();
    if balances.is_insufficient() {
        return Err(ClientError::BalanceError(
            "Balance is insufficient before sync".to_string(),
        ));
    }
    let mut current_timestamp = chrono::Utc::now().timestamp() as u64;
    // Add some buffer to the current timestamp
    current_timestamp = current_timestamp.saturating_sub(tx_timeout);

    let tx_info = fetch_tx_info(
        store_vault_server,
        validity_prover,
        key,
        user_data.tx_lpt,
        &user_data.processed_tx_uuids,
        tx_timeout,
    )
    .await?;

    //  First, if there is a pending tx, return a pending error
    if let Some((meta, tx_data)) = tx_info.pending.first() {
        return Ok((
            vec![Action::PendingTx(meta.clone(), tx_data.clone())],
            PendingInfo::default(),
        ));
    }

    // Then, collect deposit and transfer data
    let deposit_info = fetch_deposit_info(
        store_vault_server,
        validity_prover,
        liquidity_contract,
        key,
        user_data.deposit_lpt,
        &user_data.processed_deposit_uuids,
        deposit_timeout,
    )
    .await?;
    let transfer_info = fetch_transfer_info(
        store_vault_server,
        validity_prover,
        key,
        user_data.transfer_lpt,
        &user_data.processed_transfer_uuids,
        tx_timeout,
    )
    .await?;

    // Get the timestamp of the oldest pending deposit
    let oldest_pending_deposit_timestamp = deposit_info
        .pending
        .iter()
        .map(|(meta, _)| meta.timestamp)
        .min();

    // Get the timestamp of the oldest pending transfer
    let oldest_pending_transfer_timestamp = transfer_info
        .pending
        .iter()
        .map(|(meta, _)| meta.timestamp)
        .min();

    let mut deposits = deposit_info.settled;
    let mut transfers = transfer_info.settled;

    // Next, for each settled tx, take deposits and transfers that are strictly smaller than the block number of the tx
    let mut sequence = Vec::new();
    for (tx_meta, tx_data) in tx_info.settled.iter() {
        let receives = collect_receives(
            &Some((tx_meta.clone(), tx_data.clone())),
            &mut deposits,
            &mut transfers,
        )
        .await?;

        // Apply receives to balances
        for receive in &receives {
            receive.apply_to_balances(&mut balances);
        }
        let is_insufficient = balances.sub_tx(tx_data);
        if is_insufficient {
            if deposit_info.pending.is_empty() && transfer_info.pending.is_empty() {
                // Unresolved balance shortage
                return Err(ClientError::BalanceError(
                    "Insufficient balance during sync".to_string(),
                ));
            } else {
                // To incorporate the tx, you need to incorporate the pending deposit/transfer to solve the balance shortage.
                // TODO: Processing when the balance shortage is not resolved even if the pending deposit/transfer is incorporated
                sequence.push(Action::PendingReceives(tx_meta.clone(), tx_data.clone()));
            }
        }

        // Here tx can be incorporated

        // The smallest timestamp among the remaining deposits and pending deposits is 1 less than the new lpt.
        let new_deposit_lpt = deposits
            .iter()
            .map(|(meta, _)| meta.timestamp)
            .chain(oldest_pending_deposit_timestamp)
            .min()
            .map(|timestamp| timestamp - 1)
            .unwrap_or(current_timestamp);
        // The smallest timestamp among the remaining transfers and pending transfers is 1 less than the new lpt.
        let new_transfer_lpt = transfers
            .iter()
            .map(|(meta, _)| meta.timestamp)
            .chain(oldest_pending_transfer_timestamp)
            .min()
            .map(|timestamp| timestamp - 1)
            .unwrap_or(current_timestamp);

        sequence.push(Action::Receive {
            receives,
            new_deposit_lpt,
            new_transfer_lpt,
        });
        sequence.push(Action::Tx(tx_meta.clone(), tx_data.clone()));
    }

    // Finally, take all deposits and transfers
    let receives = collect_receives(&None, &mut deposits, &mut transfers).await?;
    let new_deposit_lpt = oldest_pending_deposit_timestamp
        .map(|timestamp| timestamp - 1)
        .unwrap_or(current_timestamp);
    let new_transfer_lpt = oldest_pending_transfer_timestamp
        .map(|timestamp| timestamp - 1)
        .unwrap_or(current_timestamp);
    sequence.push(Action::Receive {
        receives,
        new_deposit_lpt,
        new_transfer_lpt,
    });
    Ok((
        sequence,
        PendingInfo {
            pending_deposits: deposit_info.pending,
            pending_transfers: transfer_info.pending,
        },
    ))
}

/// For each settled tx, take deposits and transfers that are strictly smaller than the block number of the tx
/// If there is no tx, take all deposit and transfer data
async fn collect_receives(
    tx: &Option<(MetaData, TxData<F, C, D>)>,
    deposits: &mut Vec<(MetaData, DepositData)>,
    transfers: &mut Vec<(MetaData, TransferData<F, C, D>)>,
) -> Result<Vec<ReceiveAction>, ClientError> {
    let mut receives: Vec<ReceiveAction> = Vec::new();
    if let Some((meta, _tx_data)) = tx {
        let block_number = meta.block_number.unwrap();

        // take and remove deposit that are strictly smaller than the block number of the tx
        let receive_deposit = deposits
            .iter()
            .filter(|(meta, _)| meta.block_number.unwrap() < block_number)
            .map(|(meta, data)| ReceiveAction::Deposit(meta.clone(), data.clone()))
            .collect_vec();
        deposits.retain(|(meta, _)| meta.block_number.unwrap() >= block_number);

        // take and remove transfer that are strictly smaller than the block number of the tx
        let receive_transfer = transfers
            .iter()
            .filter(|(meta, _)| meta.block_number.unwrap() < block_number)
            .map(|(meta, data)| ReceiveAction::Transfer(meta.clone(), data.clone()))
            .collect_vec();
        transfers.retain(|(meta, _)| meta.block_number.unwrap() >= block_number);

        // add to receives
        receives.extend(receive_deposit);
        receives.extend(receive_transfer);
    } else {
        // if there is no tx, take all deposit and transfer data
        let receive_deposit = deposits
            .iter()
            .map(|(meta, data)| ReceiveAction::Deposit(meta.clone(), data.clone()))
            .collect_vec();
        deposits.clear();

        let receive_transfer = transfers
            .iter()
            .map(|(meta, data)| ReceiveAction::Transfer(meta.clone(), data.clone()))
            .collect_vec();
        transfers.clear();

        receives.extend(receive_deposit);
        receives.extend(receive_transfer);
    }

    // sort by block number first, then by uuid to make the order deterministic
    receives.sort_by_key(|action| {
        let meta = action.meta();
        (meta.block_number.unwrap(), meta.uuid.clone())
    });

    Ok(receives)
}
