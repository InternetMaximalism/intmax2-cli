use intmax2_zkp::ethereum_types::bytes32::Bytes32;
use serde_json::json;

use crate::external_api::{
    common::error::ServerError,
    utils::{
        encode::encode_base64,
        handler::{handle_response, ResponseType},
        retry::with_retry,
    },
};

pub async fn save_user_data(
    server_base_url: &str,
    pubkey: Bytes32,
    encypted_data: Vec<u8>,
) -> Result<(), ServerError> {
    let url = format!("{}/user-data", server_base_url,);
    let encrypted_data_encoded = encode_base64(&encypted_data);
    let request = json!({
        "user": pubkey,
        "encryptedUserData": encrypted_data_encoded,
    });
    let response = with_retry(|| async {
        reqwest_wasm::Client::new()
            .post(url.clone())
            .json(&request)
            .send()
            .await
    })
    .await
    .map_err(|e| ServerError::NetworkError(format!("Failed to save user data to server: {}", e)))?;

    match handle_response(response).await? {
        ResponseType::Success(_) => Ok(()),
        ResponseType::ServerError(error) => Err(ServerError::ServerError(error.message)),
        _ => Err(ServerError::UnknownError("Unknown error".to_string())),
    }
}
