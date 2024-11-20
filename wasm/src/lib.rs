use crate::js_types::common::JsTx;
use client::{get_client, get_mock_contract, Config};
use ethers::types::H256;
use intmax2_core_sdk::{
    client::account::generate_intmax_account_from_eth_key as inner_generate_intmax_account_from_eth_key,
    external_api::contract::interface::ContractInterface,
};
use intmax2_zkp::{
    common::transfer::Transfer,
    constants::NUM_TRANSFERS_IN_TX,
    ethereum_types::{u256::U256, u32limb_trait::U32LimbTrait},
    mock::data::{deposit_data::DepositData, transfer_data::TransferData, tx_data::TxData},
};
use js_types::{
    common::JsTransfer,
    data::{JsDepositData, JsTransferData, JsTxData, JsUserData},
    utils::parse_u256,
    wrapper::{JsBlockProposal, JsTxRequestMemo},
};
use num_bigint::BigUint;
use utils::{h256_to_bytes32, parse_h256, str_privkey_to_keyset};
use wasm_bindgen::{prelude::wasm_bindgen, JsError};

pub mod client;
pub mod js_types;
pub mod utils;

#[derive(Debug, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct IntmaxAccount {
    pub privkey: String,
    pub pubkey: String,
}

/// Generate a new key pair from the given ethereum private key (32bytes hex string).
#[wasm_bindgen]
pub async fn generate_intmax_account_from_eth_key(
    eth_private_key: &str,
) -> Result<IntmaxAccount, JsError> {
    let eth_private_key = parse_h256(eth_private_key)?;
    let key_set = inner_generate_intmax_account_from_eth_key(eth_private_key);
    let private_key: U256 = BigUint::from(key_set.privkey).try_into().unwrap();
    Ok(IntmaxAccount {
        privkey: private_key.to_hex(),
        pubkey: key_set.pubkey.to_hex(),
    })
}

/// Function to take a backup before calling the deposit function of the liquidity contract.
/// You can also get the pubkey_salt_hash from the return value.
#[wasm_bindgen]
pub async fn prepare_deposit(
    config: &Config,
    private_key: &str,
    amount: &str,
    token_index: u32,
) -> Result<String, JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let amount = parse_u256(amount)?;

    let client = get_client(config);
    let deposit_call = client
        .prepare_deposit(key, token_index, amount)
        .await
        .map_err(|e| {
            JsError::new(&format!(
                "failed to prepare deposit call: {}",
                e.to_string()
            ))
        })?;
    Ok(deposit_call.pubkey_salt_hash.to_string())
}

/// Function to send a tx request to the block builder. The return value contains information to take a backup.
#[wasm_bindgen]
pub async fn send_tx_request(
    config: &Config,
    block_builder_url: &str,
    private_key: &str,
    transfers: Vec<JsTransfer>,
) -> Result<JsTxRequestMemo, JsError> {
    if transfers.len() > NUM_TRANSFERS_IN_TX {
        return Err(JsError::new(&format!(
            "Number of transfers in a tx must be less than or equal to {}",
            NUM_TRANSFERS_IN_TX
        )));
    }
    let key = str_privkey_to_keyset(private_key)?;
    let transfers: Vec<Transfer> = transfers
        .iter()
        .map(|transfer| transfer.to_transfer())
        .collect::<Result<Vec<_>, JsError>>()?;

    let client = get_client(config);
    let memo = client
        .send_tx_request(block_builder_url, key, transfers)
        .await
        .map_err(|e| JsError::new(&format!("failed to send tx request {}", e)))?;

    Ok(JsTxRequestMemo::from_tx_request_memo(&memo))
}

/// Function to query the block proposal from the block builder.
/// The return value is the block proposal or null if the proposal is not found.
/// If got an invalid proposal, it will return an error.
#[wasm_bindgen]
pub async fn query_proposal(
    config: &Config,
    block_builder_url: &str,
    private_key: &str,
    tx: &JsTx,
) -> Result<Option<JsBlockProposal>, JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let tx = tx.to_tx()?;

    let client = get_client(config);
    let proposal = client.query_proposal(block_builder_url, key, tx).await?;
    let proposal = proposal.map(|proposal| JsBlockProposal::from_block_proposal(&proposal));
    Ok(proposal)
}

/// In this function, query block proposal from the block builder,
/// and then send the signed tx tree root to the block builder.
/// A backup of the tx is also taken.
/// You need to call send_tx_request before calling this function.
/// The return value is the tx tree root.
#[wasm_bindgen]
pub async fn finalize_tx(
    config: &Config,
    block_builder_url: &str,
    private_key: &str,
    tx_request_memo: &JsTxRequestMemo,
    proposal: &JsBlockProposal,
) -> Result<String, JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let tx_request_memo = tx_request_memo.to_tx_request_memo()?;
    let proposal = proposal.to_block_proposal()?;
    let client = get_client(config);
    let tx_tree_root = client
        .finalize_tx(block_builder_url, key, &tx_request_memo, &proposal)
        .await?;
    Ok(tx_tree_root.to_string())
}

/// Synchronize the user's balance proof. It may take a long time to generate ZKP.
#[wasm_bindgen]
pub async fn sync(config: &Config, private_key: &str) -> Result<(), JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let client = get_client(config);
    client.sync(key).await?;
    Ok(())
}

/// Synchronize the user's withdrawal proof, and send request to the withdrawal aggregator.
/// It may take a long time to generate ZKP.
#[wasm_bindgen]
pub async fn sync_withdrawals(config: &Config, private_key: &str) -> Result<(), JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let client = get_client(config);
    client.sync_withdrawals(key).await?;
    Ok(())
}

/// Get the user's data. It is recommended to sync before calling this function.
#[wasm_bindgen]
pub async fn get_user_data(config: &Config, private_key: &str) -> Result<JsUserData, JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let client = get_client(config);
    let user_data = client.get_user_data(key).await?;
    Ok(JsUserData::from_user_data(&user_data))
}

/// Decrypt the deposit data.
#[wasm_bindgen]
pub async fn decrypt_deposit_data(
    private_key: &str,
    data: &[u8],
) -> Result<JsDepositData, JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let deposit_data =
        DepositData::decrypt(data, key).map_err(|e| JsError::new(&format!("{}", e)))?;
    Ok(JsDepositData::from_deposit_data(&deposit_data))
}

/// Decrypt the transfer data. This is also used to decrypt the withdrawal data.
#[wasm_bindgen]
pub async fn decrypt_transfer_data(
    private_key: &str,
    data: &[u8],
) -> Result<JsTransferData, JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let transfer_data =
        TransferData::decrypt(data, key).map_err(|e| JsError::new(&format!("{}", e)))?;
    Ok(JsTransferData::from_transfer_data(&transfer_data))
}

/// Decrypt the tx data.
#[wasm_bindgen]
pub async fn decrypt_tx_data(private_key: &str, data: &[u8]) -> Result<JsTxData, JsError> {
    let key = str_privkey_to_keyset(private_key)?;
    let tx_data = TxData::decrypt(data, key).map_err(|e| JsError::new(&format!("{}", e)))?;
    Ok(JsTxData::from_tx_data(&tx_data))
}

/// Function to mimic the deposit call of the contract. For development purposes only.
#[wasm_bindgen]
pub async fn mimic_deposit(
    contract_server_url: &str,
    pubkey_salt_hash: &str,
    token_index: u32,
    amount: &str,
) -> Result<(), JsError> {
    let pubkey_salt_hash = parse_h256(pubkey_salt_hash)?;
    let pubkey_salt_hash = h256_to_bytes32(pubkey_salt_hash);
    let amount = parse_u256(amount)?;

    let contract = get_mock_contract(contract_server_url);
    contract
        .deposit(H256::default(), pubkey_salt_hash, token_index, amount)
        .await?;
    Ok(())
}
