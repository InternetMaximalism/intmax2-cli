use intmax2_zkp::ethereum_types::bytes32::Bytes32;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::external_api::{
    common::{error::ServerError, response::ServerCommonResponse},
    utils::{
        encode::encode_base64,
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

use super::data_type::EncryptedDataType;

#[derive(Serialize, Deserialize)]
struct SaveEncryptedDataResponse {
    uuid: String,
}

pub async fn save_encrypted_data(
    server_base_url: &str,
    data_type: EncryptedDataType,
    pubkey: Bytes32,
    encrypted_data: Vec<u8>,
) -> Result<(), ServerError> {
    let url = format!("{}/{}", server_base_url, data_type);
    let request = generate_request(data_type, pubkey, encrypted_data);
    let response = with_retry(|| async {
        reqwest_wasm::Client::new()
            .post(&url)
            .json(&request)
            .send()
            .await
    })
    .await
    .map_err(|e| {
        ServerError::NetworkError(format!("Failed to save encrypted data to server: {}", e))
    })?;
    match handle_response(response).await? {
        ResponseType::Success(response) => {
            let response: ServerCommonResponse<SaveEncryptedDataResponse> =
                response
                    .json()
                    .await
                    .map_err(|e| ServerError::DeserializationError(e.to_string()))?;
            log::info!("Saved encrypted data with uuid: {}", response.data.uuid);
            Ok(())
        }
        ResponseType::ServerError(error) => Err(ServerError::ServerError(error.message)),
        _ => Err(ServerError::UnknownError("Unknown error".to_string())),
    }
}

fn generate_request(
    data_type: EncryptedDataType,
    pubkey: Bytes32,
    encrypted_data: Vec<u8>,
) -> Value {
    let encrypted_data = encode_base64(&encrypted_data);
    match data_type {
        EncryptedDataType::Deposit => {
            let data = serde_json::to_value(encrypted_data).unwrap();
            json!({
                "recipient": pubkey,
                "encryptedDepositData": data,
            })
        }
        EncryptedDataType::Transfer => {
            let data = serde_json::to_value(encrypted_data).unwrap();
            json!({
                "sender": pubkey,
                "encryptedTransferData": data,
                "signature": "", // TODO: add signature
            })
        }
        EncryptedDataType::Transaction => {
            let data = serde_json::to_value(encrypted_data).unwrap();
            json!({
                "sender": pubkey,
                "encryptedTransactionData": data,
            })
        }
        EncryptedDataType::Withdrawal => {
            let data = serde_json::to_value(encrypted_data).unwrap();
            json!({
                "sender": pubkey,
                "encryptedWithdrawalData": data,
            })
        }
    }
}
