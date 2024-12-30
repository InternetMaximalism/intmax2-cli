use intmax2_interfaces::{
    api::{
        store_vault_server::interface::StoreVaultClientInterface,
        validity_prover::interface::ValidityProverClientInterface,
    },
    data::{
        deposit_data::DepositData, meta_data::MetaData, transfer_data::TransferData,
        tx_data::TxData, user_data::UserData,
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
        receive_actions: Vec<ReceiveAction>,
        new_deposit_lpt: u64,
        new_transfer_lpt: u64,
    },
    Tx(MetaData, TxData<F, C, D>),        // Send tx
    PendingTx(MetaData, TxData<F, C, D>), // Pending tx
    None,
}

#[derive(Debug, Clone)]
pub enum ReceiveAction {
    Deposit(MetaData, DepositData),
    Transfer(MetaData, TransferData<F, C, D>),
}

#[derive(Debug, Clone, Default)]
pub struct PendingInfo {
    pub pending_deposits: Vec<(MetaData, DepositData)>,
    pub pending_transfers: Vec<(MetaData, TransferData<F, C, D>)>,
}

pub async fn determine_sequence<S: StoreVaultClientInterface, V: ValidityProverClientInterface>(
    store_vault_server: &S,
    validity_prover: &V,
    liquidity_contract: &LiquidityContract,
    key: KeySet,
    deposit_timeout: u64,
    tx_timeout: u64,
) -> Result<(Action, PendingInfo), ClientError> {
    let user_data = store_vault_server
        .get_user_data(key.pubkey)
        .await?
        .map(|encrypted| UserData::decrypt(&encrypted, key))
        .transpose()
        .map_err(|e| ClientError::DecryptionError(e.to_string()))?
        .unwrap_or(UserData::new(key.pubkey));

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
            Action::PendingTx(meta.clone(), tx_data.clone()),
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

    // settleされたtxそれぞれについて、そのtxのblock numberよりもも厳密に小さいものを取得

    todo!()
}

// For each settled tx, take deposits and transfers that are strictly smaller than the block number of the tx
// If there is no tx, take all deposit and transfer data
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
    receives.sort_by_key(|action| match action {
        ReceiveAction::Deposit(meta, _) => (meta.block_number.unwrap(), meta.uuid.clone()),
        ReceiveAction::Transfer(meta, _) => (meta.block_number.unwrap(), meta.uuid.clone()),
    });

    Ok(receives)
}
