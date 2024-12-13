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
    Deposit(MetaData, DepositData),            // Receive deposit
    Transfer(MetaData, TransferData<F, C, D>), // Receive transfer
    Tx(MetaData, TxData<F, C, D>),             // Send tx
}

#[derive(Debug, Clone)]
pub struct NextAction {
    pub action: Option<Action>,
    pub pending_deposits: Vec<MetaData>,
    pub pending_transfers: Vec<MetaData>,
    pub pending_txs: Vec<MetaData>,
}

// generate strategy of the balance proof update process
pub async fn determine_next_action<
    S: StoreVaultClientInterface,
    V: ValidityProverClientInterface,
>(
    store_vault_server: &S,
    validity_prover: &V,
    liquidity_contract: &LiquidityContract,
    key: KeySet,
    deposit_timeout: u64,
    tx_timeout: u64,
) -> Result<NextAction, ClientError> {
    // get user data from the data store server
    let user_data = store_vault_server
        .get_user_data(key.pubkey)
        .await?
        .map(|encrypted| UserData::decrypt(&encrypted, key))
        .transpose()
        .map_err(|e| ClientError::DecryptionError(e.to_string()))?
        .unwrap_or(UserData::new(key.pubkey));

    let deposit_info = fetch_deposit_info(
        store_vault_server,
        validity_prover,
        liquidity_contract,
        key,
        user_data.deposit_lpt,
        deposit_timeout,
    )
    .await?;

    let transfer_info = fetch_transfer_info(
        store_vault_server,
        validity_prover,
        key,
        user_data.transfer_lpt,
        tx_timeout,
    )
    .await?;

    let tx_info = fetch_tx_info(
        store_vault_server,
        validity_prover,
        key,
        user_data.tx_lpt,
        tx_timeout,
    )
    .await?;

    let mut all_actions: Vec<(u32, u8, Action)> = Vec::new();

    // Add tx data with priority 1
    for (meta, data) in tx_info.settled.into_iter() {
        all_actions.push((meta.block_number.unwrap(), 1, Action::Tx(meta, data)));
    }
    // Add deposit data with priority 2
    for (meta, data) in deposit_info.settled.into_iter() {
        all_actions.push((meta.block_number.unwrap(), 2, Action::Deposit(meta, data)));
    }
    // Add transfer data with priority 3
    for (meta, data) in transfer_info.settled.into_iter() {
        all_actions.push((meta.block_number.unwrap(), 3, Action::Transfer(meta, data)));
    }

    // Sort by block number first, then by priority
    all_actions.sort_by_key(|(block_num, priority, _)| (*block_num, *priority));

    // Get the next action
    let next_action = all_actions.first().map(|(_, _, action)| action.clone());

    Ok(NextAction {
        action: next_action,
        pending_deposits: deposit_info.pending,
        pending_transfers: transfer_info.pending,
        pending_txs: tx_info.pending,
    })
}
