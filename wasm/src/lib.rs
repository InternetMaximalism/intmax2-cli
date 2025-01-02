use crate::js_types::common::JsTx;
use client::{get_client, Config};
use intmax2_client_sdk::{
    client::key_from_eth::generate_intmax_account_from_eth_key as inner_generate_intmax_account_from_eth_key,
    external_api::utils::time::sleep_for,
};
use intmax2_interfaces::data::{
    deposit_data::{DepositData, TokenType},
    transfer_data::TransferData,
    tx_data::TxData,
};
use intmax2_zkp::{
    common::transfer::Transfer,
    constants::NUM_TRANSFERS_IN_TX,
    ethereum_types::{u256::U256, u32limb_trait::U32LimbTrait},
};
use js_types::{
    common::JsTransfer,
    data::{JsDepositData, JsDepositResult, JsTransferData, JsTxData, JsTxResult, JsUserData},
    utils::{parse_address, parse_u256},
    wrapper::{JsBlockProposal, JsTxRequestMemo},
};
use num_bigint::BigUint;
use utils::{parse_h256, parse_h256_as_u256, str_privkey_to_keyset};
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
    init_logger();
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
    recipient: &str,
    amount: &str,
    token_type: u8,
    token_address: &str,
    token_id: &str,
) -> Result<JsDepositResult, JsError> {
    init_logger();
    let recipient = parse_h256_as_u256(recipient)?;
    let amount = parse_u256(amount)?;
    let token_type = TokenType::try_from(token_type).map_err(|e| JsError::new(&e))?;
    let token_address = parse_address(token_address)?;
    let token_id = parse_u256(token_id)?;
    let client = get_client(config);
    let deposit_result = client
        .prepare_deposit(recipient, amount, token_type, token_address, token_id)
        .await
        .map_err(|e| {
            JsError::new(&format!(
                "failed to prepare deposit call: {}",
                e.to_string()
            ))
        })?;
    Ok(JsDepositResult::from_deposit_result(&deposit_result))
}

/// Function to send a tx request to the block builder. The return value contains information to take a backup.
#[wasm_bindgen]
pub async fn send_tx_request(
    config: &Config,
    block_builder_url: &str,
    private_key: &str,
    transfers: Vec<JsTransfer>,
) -> Result<JsTxRequestMemo, JsError> {
    init_logger();
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
    is_registration_block: bool,
    tx: &JsTx,
) -> Result<Option<JsBlockProposal>, JsError> {
    init_logger();
    let key = str_privkey_to_keyset(private_key)?;
    let tx = tx.to_tx()?;

    let client = get_client(config);
    let proposal = client
        .query_proposal(block_builder_url, key, is_registration_block, tx)
        .await?;
    let proposal = proposal.map(|proposal| JsBlockProposal::from_block_proposal(&proposal));
    Ok(proposal)
}

/// Send the signed tx tree root to the block builder during taking a backup of the tx.
/// You need to call send_tx_request before calling this function.
/// The return value is the tx result, which contains the tx tree root and transfer data.
#[wasm_bindgen]
pub async fn finalize_tx(
    config: &Config,
    block_builder_url: &str,
    private_key: &str,
    tx_request_memo: &JsTxRequestMemo,
    proposal: &JsBlockProposal,
) -> Result<JsTxResult, JsError> {
    init_logger();
    let key = str_privkey_to_keyset(private_key)?;
    let tx_request_memo = tx_request_memo.to_tx_request_memo()?;
    let proposal = proposal.to_block_proposal()?;
    let client = get_client(config);
    let tx_result = client
        .finalize_tx(block_builder_url, key, &tx_request_memo, &proposal)
        .await?;
    Ok(JsTxResult::from_tx_result(&tx_result))
}

/// Batch function of query_proposal and finalize_tx.
#[wasm_bindgen]
pub async fn query_and_finalize(
    config: &Config,
    block_builder_url: &str,
    private_key: &str,
    tx_request_memo: &JsTxRequestMemo,
) -> Result<JsTxResult, JsError> {
    init_logger();
    let key = str_privkey_to_keyset(private_key)?;
    let client = get_client(config);
    let tx_request_memo = tx_request_memo.to_tx_request_memo()?;
    let is_registration_block = tx_request_memo.is_registration_block;
    let tx = tx_request_memo.tx.clone();
    let mut tries = 0;
    let proposal = loop {
        let proposal = client
            .query_proposal(&block_builder_url, key, is_registration_block, tx)
            .await?;
        if proposal.is_some() {
            break proposal.unwrap();
        }
        if tries > config.block_builder_query_limit {
            return Err(JsError::new("Failed to get proposal"));
        }
        tries += 1;
        sleep_for(config.block_builder_query_interval).await;
    };
    let tx_result = client
        .finalize_tx(&block_builder_url, key, &tx_request_memo, &proposal)
        .await?;
    Ok(JsTxResult::from_tx_result(&tx_result))
}

/// Synchronize the user's balance proof. It may take a long time to generate ZKP.
#[wasm_bindgen]
pub async fn sync(config: &Config, private_key: &str) -> Result<(), JsError> {
    init_logger();
    let key = str_privkey_to_keyset(private_key)?;
    let client = get_client(config);
    client.sync(key).await?;
    Ok(())
}

/// Synchronize the user's withdrawal proof, and send request to the withdrawal aggregator.
/// It may take a long time to generate ZKP.
#[wasm_bindgen]
pub async fn sync_withdrawals(config: &Config, private_key: &str) -> Result<(), JsError> {
    init_logger();
    let key = str_privkey_to_keyset(private_key)?;
    let client = get_client(config);
    client.sync_withdrawals(key).await?;
    Ok(())
}

/// Get the user's data. It is recommended to sync before calling this function.
#[wasm_bindgen]
pub async fn get_user_data(config: &Config, private_key: &str) -> Result<JsUserData, JsError> {
    init_logger();
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
    init_logger();
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
    init_logger();
    let key = str_privkey_to_keyset(private_key)?;
    let transfer_data =
        TransferData::decrypt(data, key).map_err(|e| JsError::new(&format!("{}", e)))?;
    Ok(JsTransferData::from_transfer_data(&transfer_data))
}

/// Decrypt the tx data.
#[wasm_bindgen]
pub async fn decrypt_tx_data(private_key: &str, data: &[u8]) -> Result<JsTxData, JsError> {
    init_logger();
    let key = str_privkey_to_keyset(private_key)?;
    let tx_data = TxData::decrypt(data, key).map_err(|e| JsError::new(&format!("{}", e)))?;
    Ok(JsTxData::from_tx_data(&tx_data))
}

fn init_logger() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
}
