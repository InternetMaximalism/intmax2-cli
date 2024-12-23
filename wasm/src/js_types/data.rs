use intmax2_client_sdk::client::client::TxResult;
use intmax2_interfaces::data::{
    deposit_data::DepositData, transfer_data::TransferData, tx_data::TxData, user_data::UserData,
};
use intmax2_zkp::ethereum_types::u32limb_trait::U32LimbTrait as _;
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use wasm_bindgen::prelude::wasm_bindgen;

use super::common::{JsTransfer, JsTx};

type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsDepositData {
    pub deposit_salt: String,     // hex string
    pub pubkey_salt_hash: String, // hex string
    pub amount: String,           // 10 base string
    pub token_type: u8,
    pub token_address: String, // hex string
    pub token_id: String,      // 10 base string
}

impl JsDepositData {
    pub fn from_deposit_data(deposit_data: &DepositData) -> Self {
        Self {
            deposit_salt: deposit_data.deposit_salt.to_string(),
            pubkey_salt_hash: deposit_data.pubkey_salt_hash.to_hex(),
            amount: deposit_data.amount.to_string(),
            token_type: deposit_data.token_type as u8,
            token_address: deposit_data.token_address.to_hex(),
            token_id: deposit_data.token_id.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsTransferData {
    pub sender: String,
    pub transfer: JsTransfer,
}

impl JsTransferData {
    pub fn from_transfer_data(transfer_data: &TransferData<F, C, D>) -> Self {
        Self {
            sender: transfer_data.sender.to_hex(),
            transfer: JsTransfer::from_transfer(&transfer_data.transfer),
        }
    }
}

#[derive(Debug, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsTxData {
    pub tx: JsTx,
    pub transfers: Vec<JsTransfer>,
}

impl JsTxData {
    pub fn from_tx_data(tx_data: &TxData<F, C, D>) -> Self {
        let tx = JsTx::from_tx(&tx_data.common.tx);
        let transfers = tx_data
            .spent_witness
            .transfers
            .iter()
            .map(JsTransfer::from_transfer)
            .collect::<Vec<_>>();
        Self { tx, transfers }
    }
}

#[derive(Debug, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsTxResult {
    pub tx_tree_root: String,
    pub transfer_data_vec: Vec<JsTransferData>,
}

impl JsTxResult {
    pub fn from_tx_result(tx_result: &TxResult) -> Self {
        let tx_tree_root = tx_result.tx_tree_root.to_hex();
        let transfer_data_vec = tx_result
            .transfer_data_vec
            .iter()
            .map(JsTransferData::from_transfer_data)
            .collect::<Vec<_>>();
        Self {
            tx_tree_root,
            transfer_data_vec,
        }
    }
}

#[derive(Debug, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsUserData {
    /// The user public key
    pub pubkey: String,

    /// The block number of the user data
    pub block_number: u32,

    /// The token balances of the user
    pub balances: Vec<TokenBalance>,

    /// The private commitment of the user
    pub private_commitment: String,

    /// The last unix timestamp of processed deposits
    pub deposit_lpt: u64,

    /// The last unix timestamp of processed transfers
    pub transfer_lpt: u64,

    /// The last unix timestamp of processed txs
    pub tx_lpt: u64,

    /// The last unix timestamp of processed withdrawals
    pub withdrawal_lpt: u64,

    /// Uuids of processed deposits
    pub processed_deposit_uuids: Vec<String>,

    /// Uuids of processed transfers
    pub processed_transfer_uuids: Vec<String>,

    /// Uuids of processed txs
    pub processed_tx_uuids: Vec<String>,

    /// Uuids of processed withdrawals
    pub processed_withdrawal_uuids: Vec<String>,
}

#[derive(Debug, Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct TokenBalance {
    /// Token index of the balance
    pub token_index: u32,

    /// The amount of the token. 10 base string
    pub amount: String,

    /// Flag indicating whether the balance is insufficient for that token index.
    /// If true, subsequent transfers or withdrawals for that token index will be impossible.
    pub is_insufficient: bool,
}

impl JsUserData {
    pub fn from_user_data(user_data: &UserData) -> Self {
        let balances = user_data
            .balances()
            .iter()
            .map(|(token_index, leaf)| {
                let amount = leaf.amount.to_string();
                let is_insufficient = leaf.is_insufficient;
                TokenBalance {
                    token_index: *token_index as u32,
                    amount,
                    is_insufficient,
                }
            })
            .collect();

        Self {
            pubkey: user_data.pubkey.to_hex(),
            block_number: user_data.block_number,
            balances,
            private_commitment: user_data
                .full_private_state
                .to_private_state()
                .commitment()
                .to_string(),
            deposit_lpt: user_data.deposit_lpt,
            transfer_lpt: user_data.transfer_lpt,
            tx_lpt: user_data.tx_lpt,
            withdrawal_lpt: user_data.withdrawal_lpt,
            processed_deposit_uuids: user_data.processed_deposit_uuids.clone(),
            processed_transfer_uuids: user_data.processed_transfer_uuids.clone(),
            processed_tx_uuids: user_data.processed_tx_uuids.clone(),
            processed_withdrawal_uuids: user_data.processed_withdrawal_uuids.clone(),
        }
    }
}
